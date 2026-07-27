use super::{Tool, ToolParameter, ToolResult};
use crate::graph::{
    affected_report, explain_node, resolve_seed, scan_directory, GraphQuery, GraphStorage,
    KnowledgeGraph,
};
use anyhow::{bail, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Graphify Tool — Blast Radius bằng Knowledge Graph nội bộ.
///
/// Đã fork lõi graphify-8 về `src/graph/` (extractor + blast_radius), không còn
/// phụ thuộc CLI graphify bên ngoài. Quy trình 2 bước:
///   1. `scan`  : cắt AST/tham chiếu của 1 thư mục → graph.json db (SQLite) tại
///      `<root>/.culi/graph.db`
///   2. Query   : `affected` (blast radius), `explain` (chi tiết node),
///      `path` (đường đi giữa 2 node) — load graph từ db
pub struct GraphifyTool;

impl GraphifyTool {
    pub fn new() -> Self {
        Self
    }

    fn db_path(root: &str) -> PathBuf {
        Path::new(root).join(".culi").join("graph.db")
    }

    fn load_graph(root: &str) -> Result<KnowledgeGraph> {
        let db = Self::db_path(root);
        if !db.exists() {
            bail!(
                "Graph chưa được build tại {}. Chạy graphify action='scan' với root='{}' trước.",
                db.display(),
                root
            );
        }
        let storage = GraphStorage::open(&db)?;
        let mut graph = KnowledgeGraph::new();
        for n in storage.load_all_nodes()? {
            graph.add_node(n)?;
        }
        for e in storage.load_all_edges()? {
            graph.add_edge(e)?;
        }
        Ok(graph)
    }

    fn run_scan(root: &str) -> Result<ToolResult> {
        let scan_root = Path::new(root);
        if !scan_root.is_dir() {
            return Ok(ToolResult::error(&format!(
                "root không phải thư mục: {}",
                root
            )));
        }
        let (graph, stats) = scan_directory(scan_root)?;

        let db = Self::db_path(root);
        if let Some(parent) = db.parent() {
            fs::create_dir_all(parent)?;
        }
        let storage = GraphStorage::open(&db)?;
        storage.clear()?;
        for n in graph.nodes() {
            storage.save_node(n)?;
        }
        for e in graph.edges() {
            storage.save_edge(e)?;
        }

        Ok(ToolResult::success(json!({
            "status": "scanned",
            "root": root,
            "graph_db": db.to_string_lossy(),
            "stats": serde_json::to_value(&stats)?,
            "hint": "Giờ dùng action='affected' với target=<tên hàm/class> TRƯỚC KHI sửa code.",
        })))
    }

    fn run_affected(root: &str, target: &str, depth: usize) -> Result<ToolResult> {
        let graph = Self::load_graph(root)?;
        match affected_report(&graph, target, depth) {
            Some(report) => Ok(ToolResult::success(json!({
                "status": "success",
                "seed": report.seed_label,
                "seed_id": report.seed_id,
                "hits": report.hits.len(),
                "blast_radius": report.text,
                "warning": "Các node trên phụ thuộc vào target. Cân nhắc cập nhật chúng (và todo.md) nếu sửa target.",
            }))),
            None => Ok(ToolResult::error(&format!(
                "Không resolve được node duy nhất cho '{}'. Thử tên đầy đủ hơn (VD: 'calculateTotal' thay vì 'calc'), hoặc chạy action='scan' lại.",
                target
            ))),
        }
    }

    fn run_explain(root: &str, target: &str) -> Result<ToolResult> {
        let graph = Self::load_graph(root)?;
        match explain_node(&graph, target, 50) {
            Some(text) => Ok(ToolResult::success(json!({
                "status": "success",
                "explanation": text,
            }))),
            None => Ok(ToolResult::error(&format!(
                "Không resolve được node duy nhất cho '{}'.",
                target
            ))),
        }
    }

    fn run_path(root: &str, source: &str, target: &str) -> Result<ToolResult> {
        let graph = Self::load_graph(root)?;
        let src = resolve_seed(&graph, source);
        let tgt = resolve_seed(&graph, target);
        let (src, tgt) = match (src, tgt) {
            (Some(s), Some(t)) => (s, t),
            (None, _) => {
                return Ok(ToolResult::error(&format!(
                    "Không resolve được source node: '{}'",
                    source
                )))
            }
            (_, None) => {
                return Ok(ToolResult::error(&format!(
                    "Không resolve được target node: '{}'",
                    target
                )))
            }
        };
        let query = GraphQuery::new(graph);
        match query.find_path(&src, &tgt) {
            Some(path) => {
                let chain = path
                    .nodes
                    .iter()
                    .map(|n| n.label.clone())
                    .collect::<Vec<_>>()
                    .join(" -> ");
                Ok(ToolResult::success(json!({
                    "status": "success",
                    "hops": path.nodes.len().saturating_sub(1),
                    "path": chain,
                })))
            }
            None => Ok(ToolResult::error(&format!(
                "Không có đường đi giữa '{}' và '{}'.",
                source, target
            ))),
        }
    }
}

#[async_trait]
impl Tool for GraphifyTool {
    fn name(&self) -> &str {
        "graphify"
    }

    fn description(&self) -> &str {
        "Quét Blast Radius (phạm vi ảnh hưởng) bằng Knowledge Graph nội bộ (fork graphify-8). Quy trình: action='scan' cho thư mục dự án (1 lần / khi code đổi nhiều), sau đó 'affected'/'explain'/'path'. LUÔN gọi 'affected' TRƯỚC KHI sửa code để biết các hàm phụ thuộc."
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "action".to_string(),
                description: "'scan' (build graph cho thư mục), 'affected' (blast radius của 1 symbol), 'explain' (chi tiết + connections của 1 node), 'path' (đường đi giữa 2 node).".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
            ToolParameter {
                name: "target".to_string(),
                description: "Tên hàm/class/symbol (VD: 'calculateTotal'). Bắt buộc cho 'affected'/'explain'/'path'.".to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParameter {
                name: "source".to_string(),
                description: "Node nguồn, chỉ dùng với action='path'.".to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParameter {
                name: "root".to_string(),
                description: "Thư mục gốc của dự án cần quét (mặc định: '.'). Graph lưu tại <root>/.culi/graph.db.".to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParameter {
                name: "depth".to_string(),
                description: "Độ sâu blast radius cho action='affected' (mặc định: 2).".to_string(),
                param_type: "number".to_string(),
                required: false,
            },
        ]
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let target = args.get("target").and_then(|v| v.as_str()).unwrap_or("");
        let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let root = args.get("root").and_then(|v| v.as_str()).unwrap_or(".");
        let depth = args
            .get("depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;

        match action {
            "scan" => Self::run_scan(root),
            "affected" => {
                if target.is_empty() {
                    return Ok(ToolResult::error("Action 'affected' requires 'target'"));
                }
                Self::run_affected(root, target, depth)
            }
            "explain" => {
                if target.is_empty() {
                    return Ok(ToolResult::error("Action 'explain' requires 'target'"));
                }
                Self::run_explain(root, target)
            }
            "path" => {
                if source.is_empty() || target.is_empty() {
                    return Ok(ToolResult::error(
                        "Action 'path' requires both 'source' and 'target'",
                    ));
                }
                Self::run_path(root, source, target)
            }
            _ => Ok(ToolResult::error(&format!(
                "Unsupported action: '{}'. Dùng: scan | affected | explain | path",
                action
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_file(dir: &Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        let mut f = fs::File::create(p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    fn sample_workspace() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        write_file(
            tmp.path(),
            "src/lib.rs",
            "pub fn helper() -> i32 { 42 }\n\npub fn render() -> i32 {\n    helper()\n}\n",
        );
        write_file(
            tmp.path(),
            "src/main.rs",
            "fn main() {\n    let x = helper();\n    println!(\"{}\", x);\n}\n",
        );
        tmp
    }

    #[tokio::test]
    async fn test_scan_then_affected_end_to_end() {
        let tmp = sample_workspace();
        let root = tmp.path().to_string_lossy().to_string();
        let tool = GraphifyTool::new();

        // 1. scan
        let scan = tool
            .execute(json!({"action": "scan", "root": root}))
            .await
            .unwrap();
        assert!(scan.success, "scan failed: {:?}", scan.error);
        assert!(scan.data["stats"]["nodes"].as_u64().unwrap() > 0);
        assert!(GraphifyTool::db_path(&root).exists());

        // 2. affected: helper() bị gọi bởi main() và render()
        let aff = tool
            .execute(json!({"action": "affected", "root": root, "target": "helper"}))
            .await
            .unwrap();
        assert!(aff.success, "affected failed: {:?}", aff.error);
        let report = aff.data["blast_radius"].as_str().unwrap();
        assert!(report.contains("helper()"), "report: {}", report);
        assert!(report.contains("render()"), "report: {}", report);
        assert!(report.contains("main()"), "report: {}", report);
    }

    #[tokio::test]
    async fn test_explain_and_path() {
        let tmp = sample_workspace();
        let root = tmp.path().to_string_lossy().to_string();
        let tool = GraphifyTool::new();
        tool.execute(json!({"action": "scan", "root": root}))
            .await
            .unwrap();

        let exp = tool
            .execute(json!({"action": "explain", "root": root, "target": "helper"}))
            .await
            .unwrap();
        assert!(exp.success, "explain failed: {:?}", exp.error);
        let text = exp.data["explanation"].as_str().unwrap();
        assert!(text.contains("Node: helper()"), "text: {}", text);
        assert!(text.contains("<-- render() [calls]"), "text: {}", text);

        let path = tool
            .execute(json!({
                "action": "path", "root": root,
                "source": "main", "target": "helper"
            }))
            .await
            .unwrap();
        assert!(path.success, "path failed: {:?}", path.error);
        assert!(path.data["path"].as_str().unwrap().contains("helper()"));
    }

    #[tokio::test]
    async fn test_query_before_scan_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        let tool = GraphifyTool::new();
        let res = tool
            .execute(json!({"action": "affected", "root": root, "target": "foo"}))
            .await;
        // load_graph bail → execute trả Err (anyhow) — tool caller thấy lỗi rõ ràng
        assert!(res.is_err());
        let msg = res.err().unwrap().to_string();
        assert!(msg.contains("scan"), "msg: {}", msg);
    }

    #[tokio::test]
    async fn test_unresolved_target_hint() {
        let tmp = sample_workspace();
        let root = tmp.path().to_string_lossy().to_string();
        let tool = GraphifyTool::new();
        tool.execute(json!({"action": "scan", "root": root}))
            .await
            .unwrap();
        let res = tool
            .execute(json!({"action": "affected", "root": root, "target": "nonexistent_xyz"}))
            .await
            .unwrap();
        assert!(!res.success);
        assert!(res.error.unwrap().contains("Không resolve được"));
    }
}
