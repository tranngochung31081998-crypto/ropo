use anyhow::Result;
use tracing::info;

use super::{GraphNode, GraphEdge, KnowledgeGraph, NodeType, EdgeConfidence};
use std::collections::HashMap;

/// Knowledge Graph Builder - từ graphify's code→graph approach
pub struct GraphBuilder {
    graph: KnowledgeGraph,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self {
            graph: KnowledgeGraph::new(),
        }
    }

    /// Add concept node
    pub fn add_concept(&mut self, id: &str, label: &str, properties: HashMap<String, String>) -> Result<()> {
        let node = GraphNode {
            id: id.to_string(),
            label: label.to_string(),
            node_type: NodeType::Concept,
            properties,
            confidence: 1.0,
            source: "user".to_string(),
        };
        self.graph.add_node(node)
    }

    /// Add entity node
    pub fn add_entity(&mut self, id: &str, label: &str, properties: HashMap<String, String>) -> Result<()> {
        let node = GraphNode {
            id: id.to_string(),
            label: label.to_string(),
            node_type: NodeType::Entity,
            properties,
            confidence: 1.0,
            source: "user".to_string(),
        };
        self.graph.add_node(node)
    }

    /// Build relationship between nodes
    pub fn connect(&mut self, source: &str, target: &str, relationship: &str, weight: f32) -> Result<()> {
        let edge = GraphEdge {
            source_id: source.to_string(),
            target_id: target.to_string(),
            relationship: relationship.to_string(),
            weight,
            confidence: EdgeConfidence::Extracted,
        };
        self.graph.add_edge(edge)
    }

    /// Build the final graph
    pub fn build(self) -> KnowledgeGraph {
        info!("Graph built: {} nodes, {} edges", self.graph.node_count(), self.graph.edge_count());
        self.graph
    }
}
