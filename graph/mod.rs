pub mod builder;
pub mod query;
pub mod export;
pub mod persistence;
pub mod ids;
pub mod extractor;
pub mod blast_radius;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use builder::*;
pub use query::*;
pub use export::*;
pub use persistence::GraphStorage;
pub use ids::*;
pub use extractor::*;
pub use blast_radius::*;

/// Knowledge Graph node - inspired by graphify
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: NodeType,
    pub properties: HashMap<String, String>,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Concept,
    Entity,
    Action,
    Pattern,
    File,
    Code,
    Agent,
    Task,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source_id: String,
    pub target_id: String,
    pub relationship: String,
    pub weight: f32,
    pub confidence: EdgeConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeConfidence {
    Extracted,
    Inferred,
    Ambiguous,
    Learned,
}

/// Knowledge Graph with nodes and edges
pub struct KnowledgeGraph {
    nodes: HashMap<String, GraphNode>,
    edges: Vec<GraphEdge>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: GraphNode) -> Result<()> {
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    pub fn add_edge(&mut self, edge: GraphEdge) -> Result<()> {
        self.edges.push(edge);
        Ok(())
    }

    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    pub fn find_connections(&self, node_id: &str) -> Vec<(&GraphNode, &str, f32)> {
        let mut connections = Vec::new();
        for edge in &self.edges {
            if edge.source_id == node_id {
                if let Some(target) = self.nodes.get(&edge.target_id) {
                    connections.push((target, edge.relationship.as_str(), edge.weight));
                }
            }
            if edge.target_id == node_id {
                if let Some(source) = self.nodes.get(&edge.source_id) {
                    connections.push((source, edge.relationship.as_str(), edge.weight));
                }
            }
        }
        connections
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Iterate tất cả nodes (cho persistence/export từ bên ngoài module graph)
    pub fn nodes(&self) -> impl Iterator<Item = &GraphNode> {
        self.nodes.values()
    }

    /// Tất cả edges (directed: source_id -> target_id)
    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }
}
