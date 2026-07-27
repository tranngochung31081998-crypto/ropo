use super::{Tool, ToolParameter, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub struct ChunkReaderTool;

impl ChunkReaderTool {
    pub fn new() -> Self {
        Self
    }
}

/// Trích xuất nội dung phản hồi từ response của CulirouterAPI.
/// Hỗ trợ cả 2 dạng: JSON chuẩn (stream=false) và SSE stream (data: {...}).
fn extract_llm_content(body: &str) -> Option<String> {
    // 1. Thử parse JSON non-streaming trước
    if let Ok(json) = serde_json::from_str::<Value>(body) {
        if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
            return Some(content.to_string());
        }
    }

    // 2. Fallback: parse SSE stream, gom delta.content từ các chunk
    let mut content = String::new();
    let mut found_any = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("data: ") || trimmed == "data: [DONE]" {
            continue;
        }
        if let Ok(chunk) = serde_json::from_str::<Value>(&trimmed[6..]) {
            if let Some(delta) = chunk["choices"][0]["delta"]["content"].as_str() {
                content.push_str(delta);
                found_any = true;
            }
        }
    }

    if found_any {
        Some(content)
    } else {
        None
    }
}

#[async_trait]
impl Tool for ChunkReaderTool {
    fn name(&self) -> &str {
        "chunk_reader"
    }

    fn description(&self) -> &str {
        "Đọc các file lớn bằng cơ chế Map-Reduce. Trả về mục lục (Index) hoặc các chunk nhỏ để tránh tràn context window (Lost in the Middle)."
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "file_path".to_string(),
                description: "Đường dẫn tới file cần đọc.".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
            ToolParameter {
                name: "mode".to_string(),
                description: "'index' (tạo mục lục tóm tắt) hoặc 'read_chunk' (đọc 1 phần cụ thể).".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
            ToolParameter {
                name: "chunk_index".to_string(),
                description: "Chỉ số chunk cần đọc (nếu mode = 'read_chunk'). Bắt đầu từ 0.".to_string(),
                param_type: "integer".to_string(),
                required: false,
            },
            ToolParameter {
                name: "chunk_size".to_string(),
                description: "Số dòng mỗi chunk (mặc định: 200).".to_string(),
                param_type: "integer".to_string(),
                required: false,
            },
        ]
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let file_path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
        let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("index");
        let chunk_index = args.get("chunk_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let chunk_size = args.get("chunk_size").and_then(|v| v.as_u64()).unwrap_or(200) as usize;

        if file_path.is_empty() {
            return Ok(ToolResult::error("Missing parameter: file_path"));
        }

        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(ToolResult::error(&format!("File not found: {}", file_path)));
        }

        let content = fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        let total_chunks = (total_lines + chunk_size - 1) / chunk_size;

        if mode == "index" {
            // Map-Reduce logic: Gọi LLM (CulirouterAPI - sixth) để tóm tắt
            let client = reqwest::Client::new();
            let mut index_report = format!("File: {} ({} lines, {} chunks)\n", file_path, total_lines, total_chunks);
            
            for i in 0..total_chunks {
                let start = i * chunk_size;
                let end = std::cmp::min(start + chunk_size, total_lines);
                let chunk_content = lines[start..end].join("\n");
                
                let prompt = format!("Tóm tắt thật ngắn gọn (1-2 câu) nội dung đoạn code này, liệt kê các tên hàm/class chính:\n\n{}", chunk_content);
                let payload = serde_json::json!({
                    "message": prompt
                });

                let mut summary = String::from("Không thể tạo tóm tắt");
                if let Ok(res) = client.post("http://localhost:3111/api/harness").json(&payload).send().await {
                    if let Ok(json) = res.json::<Value>().await {
                        if let Some(msg) = json.get("message").and_then(|m| m.as_str()) {
                            summary = msg.replace('\n', " ");
                        }
                    }
                }
                
                index_report.push_str(&format!("- Chunk {}: Lines {} to {} | Summary: {}\n", i, start, end, summary));
            }
            index_report.push_str("\nDùng mode='read_chunk' và chỉ định chunk_index để đọc chi tiết.");
            
            return Ok(ToolResult::success(serde_json::json!({
                "mode": "index",
                "content": index_report
            })));
        } else if mode == "read_chunk" {
            if chunk_index >= total_chunks {
                return Ok(ToolResult::error(&format!("chunk_index {} out of bounds (max {})", chunk_index, total_chunks - 1)));
            }
            
            let start = chunk_index * chunk_size;
            let end = std::cmp::min(start + chunk_size, total_lines);
            let chunk_content = lines[start..end].join("\n");

            return Ok(ToolResult::success(serde_json::json!({
                "mode": "read_chunk",
                "chunk_index": chunk_index,
                "lines": format!("{} to {}", start, end),
                "content": chunk_content
            })));
        }

        Ok(ToolResult::error("Invalid mode. Use 'index' or 'read_chunk'."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_from_json_response() {
        let body = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "model": "claude-fable-5",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "Hàm tính tổng giá."}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        }"#;
        let content = extract_llm_content(body);
        assert_eq!(content, Some("Hàm tính tổng giá.".to_string()));
    }

    #[test]
    fn test_extract_from_sse_stream() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hàm \"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"tính tổng\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" giá.\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"total_tokens\":20}}\n\n",
            "data: [DONE]\n\n"
        );
        let content = extract_llm_content(body);
        assert_eq!(content, Some("Hàm tính tổng giá.".to_string()));
    }

    #[test]
    fn test_extract_sse_skips_reasoning_and_garbage() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":null}}]}\n\n",
            "garbage line không phải SSE\n",
            "data: not-json\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\n",
            "data: [DONE]\n\n"
        );
        let content = extract_llm_content(body);
        assert_eq!(content, Some("ok".to_string()));
    }

    #[test]
    fn test_extract_returns_none_on_invalid() {
        assert_eq!(extract_llm_content("hoàn toàn không hợp lệ"), None);
        assert_eq!(extract_llm_content("{\"choices\":[]}"), None);
    }
}
