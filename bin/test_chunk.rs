use culi::tools::{ChunkReaderTool, Tool};
use serde_json::json;

#[tokio::main]
async fn main() {
    let tool = ChunkReaderTool::new();
    let args = json!({
        "file_path": "D:\\AGENT_SUPERCODE\\CulirouterAPI\\server.js",
        "mode": "index",
        "chunk_size": 50
    });

    println!("Testing chunk_reader with Sixth AI summarization...");
    match tool.execute(args).await {
        Ok(result) => {
            if result.success {
                println!("Result:\n{}", result.data["content"].as_str().unwrap_or(""));
            } else {
                println!("Tool failed: {:?}", result.error);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
