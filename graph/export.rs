use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{GraphNode, GraphEdge, KnowledgeGraph};

/// Graph export formats - inspired by graphify
#[derive(Debug, Serialize, Deserialize)]
pub struct GraphExport {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub metadata: GraphMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphMetadata {
    pub node_count: usize,
    pub edge_count: usize,
    pub format_version: String,
}

/// Graph Exporter
pub struct GraphExporter;

impl GraphExporter {
    pub fn new() -> Self {
        Self
    }

    /// Export graph to JSON
    pub fn to_json(&self, graph: &KnowledgeGraph) -> Result<String> {
        let export = GraphExport {
            nodes: graph.nodes.values().cloned().collect(),
            edges: graph.edges.clone(),
            metadata: GraphMetadata {
                node_count: graph.node_count(),
                edge_count: graph.edge_count(),
                format_version: "1.0".to_string(),
            },
        };
        Ok(serde_json::to_string_pretty(&export)?)
    }

    /// Export graph to DOT format (for visualization)
    pub fn to_dot(&self, graph: &KnowledgeGraph) -> String {
        let mut dot = String::from("digraph KnowledgeGraph {\n");
        dot.push_str("    rankdir=LR;\n");
        dot.push_str("    node [shape=box, style=rounded];\n\n");

        for node in graph.nodes.values() {
            dot.push_str(&format!("    \"{}\" [label=\"{}\"];\n", node.id, node.label));
        }

        dot.push('\n');
        for edge in &graph.edges {
            dot.push_str(&format!(
                "    \"{}\" -> \"{}\" [label=\"{}\", weight={}];\n",
                edge.source_id, edge.target_id, edge.relationship, edge.weight
            ));
        }

        dot.push_str("}\n");
        dot
    }

    /// Export graph to Mermaid format
    pub fn to_mermaid(&self, graph: &KnowledgeGraph) -> String {
        let mut mermaid = String::from("graph LR\n");

        for node in graph.nodes.values() {
            mermaid.push_str(&format!("    {}[\"{}\"]\n", node.id.replace('-', "_"), node.label));
        }

        for edge in &graph.edges {
            let src = edge.source_id.replace('-', "_");
            let tgt = edge.target_id.replace('-', "_");
            mermaid.push_str(&format!("    {} -->|{}| {}\n", src, edge.relationship, tgt));
        }

        mermaid
    }
}
