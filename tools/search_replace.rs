use super::{Tool, ToolParameter, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// Search & Replace Tool - Đắp bản vá (Patch) thay vì ghi lại toàn bộ file.
///
/// Triết lý Vibe Coding: Core Agent chỉ output đoạn code CŨ cần tìm và đoạn
/// code MỚI cần thay thế. Tool này chịu trách nhiệm dò đúng vị trí và vá file.
///
/// An toàn:
/// - Atomic: nếu BẤT KỲ patch nào fail, file KHÔNG bị thay đổi.
/// - Chống ambiguous: old_string xuất hiện > 1 lần sẽ bị từ chối (trừ khi replace_all=true).
/// - Tự normalize CRLF/LF theo line ending thực tế của file (Windows-safe).
/// - Hỗ trợ dry_run để kiểm tra trước khi ghi, và backup .bak trước khi vá.
pub struct SearchReplaceTool;

impl SearchReplaceTool {
    pub fn new() -> Self {
        Self
    }
}

/// Normalize line ending của một chuỗi patch theo line ending của file.
fn normalize_line_endings(s: &str, file_uses_crlf: bool) -> String {
    if file_uses_crlf {
        // \n -> \r\n, sau đó sửa lại các đoạn \r\r\n do input đã có sẵn \r\n
        s.replace("\n", "\r\n").replace("\r\r\n", "\r\n")
    } else {
        s.replace("\r\n", "\n")
    }
}

#[async_trait]
impl Tool for SearchReplaceTool {
    fn name(&self) -> &str {
        "search_replace"
    }

    fn description(&self) -> &str {
        "Đắp bản vá (Patch) code bằng Search & Replace. CHỈ gửi đoạn code cũ cần tìm (old_string) và đoạn code mới (new_string), TUYỆT ĐỐI không ghi lại toàn bộ file. Nếu 1 patch fail, file sẽ không bị thay đổi (atomic)."
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "file_path".to_string(),
                description: "Đường dẫn tới file cần vá.".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
            ToolParameter {
                name: "patches".to_string(),
                description: "Mảng các bản vá: [{\"old_string\": \"...\", \"new_string\": \"...\", \"replace_all\": false}]. old_string phải khớp DUY NHẤT 1 vị trí trong file (copy y nguyên kèm indent), trừ khi replace_all=true.".to_string(),
                param_type: "array".to_string(),
                required: true,
            },
            ToolParameter {
                name: "dry_run".to_string(),
                description: "true = chỉ kiểm tra các patch có khớp không, KHÔNG ghi file (mặc định: false).".to_string(),
                param_type: "boolean".to_string(),
                required: false,
            },
            ToolParameter {
                name: "create_backup".to_string(),
                description: "true = tạo file .bak trước khi vá (mặc định: true).".to_string(),
                param_type: "boolean".to_string(),
                required: false,
            },
        ]
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let file_path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
        let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);
        let create_backup = args
            .get("create_backup")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if file_path.is_empty() {
            return Ok(ToolResult::error("Missing parameter: file_path"));
        }

        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(ToolResult::error(&format!("File not found: {}", file_path)));
        }

        let patches = match args.get("patches").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            Some(_) => return Ok(ToolResult::error("Parameter 'patches' is empty")),
            None => {
                return Ok(ToolResult::error(
                    "Missing parameter: patches (array of {old_string, new_string})",
                ))
            }
        };

        let original = fs::read_to_string(path)?;
        let file_uses_crlf = original.contains("\r\n");
        let mut content = original.clone();

        let mut applied: Vec<Value> = Vec::new();
        let mut failed: Vec<Value> = Vec::new();

        for (i, patch) in patches.iter().enumerate() {
            let old_raw = patch.get("old_string").and_then(|v| v.as_str()).unwrap_or("");
            let new_raw = patch.get("new_string").and_then(|v| v.as_str()).unwrap_or("");
            let replace_all = patch
                .get("replace_all")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if old_raw.is_empty() {
                failed.push(json!({
                    "patch_index": i,
                    "reason": "old_string is empty"
                }));
                continue;
            }

            let old_string = normalize_line_endings(old_raw, file_uses_crlf);
            let new_string = normalize_line_endings(new_raw, file_uses_crlf);

            let occurrences = content.matches(&old_string).count();

            if occurrences == 0 {
                failed.push(json!({
                    "patch_index": i,
                    "reason": "old_string not found in file",
                    "hint": "Dùng chunk_reader mode='read_chunk' để copy chính xác đoạn code gốc kèm indent."
                }));
                continue;
            }

            if occurrences > 1 && !replace_all {
                failed.push(json!({
                    "patch_index": i,
                    "reason": format!("old_string matches {} locations - ambiguous", occurrences),
                    "hint": "Thêm context (dòng trên/dưới) vào old_string để duy nhất, hoặc set replace_all=true."
                }));
                continue;
            }

            let lines_before = content.lines().count();
            let replaced_count = if replace_all {
                content = content.replace(&old_string, &new_string);
                occurrences
            } else {
                content = content.replacen(&old_string, &new_string, 1);
                1
            };
            let lines_after = content.lines().count();

            applied.push(json!({
                "patch_index": i,
                "occurrences_replaced": replaced_count,
                "lines_delta": lines_after as i64 - lines_before as i64
            }));
        }

        // Atomic: bất kỳ patch nào fail -> không ghi file
        if !failed.is_empty() {
            return Ok(ToolResult {
                success: false,
                data: json!({
                    "file_path": file_path,
                    "applied": applied,
                    "failed": failed
                }),
                error: Some(format!(
                    "{} patch(es) failed - file NOT modified (atomic rollback)",
                    failed.len()
                )),
                duration_ms: 0,
            });
        }

        if dry_run {
            return Ok(ToolResult::success(json!({
                "dry_run": true,
                "file_path": file_path,
                "patches_applied": applied.len(),
                "applied": applied
            })));
        }

        if create_backup {
            let backup_path = path.with_extension(format!(
                "{}.bak",
                path.extension().and_then(|e| e.to_str()).unwrap_or("")
            ));
            fs::write(&backup_path, &original)?;
        }

        fs::write(path, &content)?;

        Ok(ToolResult::success(json!({
            "file_path": file_path,
            "patches_applied": applied.len(),
            "applied": applied,
            "backup_created": create_backup,
            "total_lines": content.lines().count()
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_temp_file(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[tokio::test]
    async fn test_single_patch_applied() {
        let f = make_temp_file("fn main() {\n    println!(\"old\");\n}\n");
        let tool = SearchReplaceTool::new();
        let args = json!({
            "file_path": f.path().to_string_lossy(),
            "create_backup": false,
            "patches": [{"old_string": "println!(\"old\");", "new_string": "println!(\"new\");"}]
        });
        let result = tool.execute(args).await.unwrap();
        assert!(result.success, "patch should succeed: {:?}", result.error);
        let content = fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("println!(\"new\");"));
        assert!(!content.contains("println!(\"old\");"));
    }

    #[tokio::test]
    async fn test_ambiguous_patch_rejected() {
        let f = make_temp_file("let x = 1;\nlet y = 1;\nlet z = 1;\n");
        let tool = SearchReplaceTool::new();
        let args = json!({
            "file_path": f.path().to_string_lossy(),
            "create_backup": false,
            "patches": [{"old_string": "= 1;", "new_string": "= 2;"}]
        });
        let result = tool.execute(args).await.unwrap();
        assert!(!result.success, "ambiguous patch must fail");
        // File không bị thay đổi (atomic)
        let content = fs::read_to_string(f.path()).unwrap();
        assert_eq!(content, "let x = 1;\nlet y = 1;\nlet z = 1;\n");
    }

    #[tokio::test]
    async fn test_replace_all() {
        let f = make_temp_file("foo\nfoo\nfoo\n");
        let tool = SearchReplaceTool::new();
        let args = json!({
            "file_path": f.path().to_string_lossy(),
            "create_backup": false,
            "patches": [{"old_string": "foo", "new_string": "bar", "replace_all": true}]
        });
        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        let content = fs::read_to_string(f.path()).unwrap();
        assert_eq!(content, "bar\nbar\nbar\n");
    }

    #[tokio::test]
    async fn test_missing_old_string_atomic_rollback() {
        let f = make_temp_file("hello world\n");
        let tool = SearchReplaceTool::new();
        let args = json!({
            "file_path": f.path().to_string_lossy(),
            "create_backup": false,
            "patches": [
                {"old_string": "hello", "new_string": "hi"},
                {"old_string": "NOT_EXIST", "new_string": "x"}
            ]
        });
        let result = tool.execute(args).await.unwrap();
        assert!(!result.success);
        // Patch 1 hợp lệ nhưng patch 2 fail -> file không đổi
        let content = fs::read_to_string(f.path()).unwrap();
        assert_eq!(content, "hello world\n");
    }

    #[tokio::test]
    async fn test_crlf_normalization() {
        let f = make_temp_file("line1\r\nold_func();\r\nline3\r\n");
        let tool = SearchReplaceTool::new();
        // LLM gửi patch với LF, file dùng CRLF
        let args = json!({
            "file_path": f.path().to_string_lossy(),
            "create_backup": false,
            "patches": [{"old_string": "line1\nold_func();", "new_string": "line1\nnew_func();"}]
        });
        let result = tool.execute(args).await.unwrap();
        assert!(result.success, "CRLF file phải khớp patch LF: {:?}", result.error);
        let content = fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("new_func();\r\n"));
        // Không được phá vỡ line ending của file gốc
        assert!(content.starts_with("line1\r\n"));
    }

    #[tokio::test]
    async fn test_dry_run_does_not_write() {
        let f = make_temp_file("abc\n");
        let tool = SearchReplaceTool::new();
        let args = json!({
            "file_path": f.path().to_string_lossy(),
            "dry_run": true,
            "patches": [{"old_string": "abc", "new_string": "xyz"}]
        });
        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        let content = fs::read_to_string(f.path()).unwrap();
        assert_eq!(content, "abc\n");
    }
}
