use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::{GraphNode, KnowledgeGraph};

/// Graph query engine - inspired by graphify's query system
pub struct GraphQuery {
    graph: KnowledgeGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult {
    pub nodes: Vec<GraphNode>,
    pub total_weight: f32,
}

impl GraphQuery {
    pub fn new(graph: KnowledgeGraph) -> Self {
        Self { graph }
    }

    /// Find path between two nodes (BFS)
    pub fn find_path(&self, source_id: &str, target_id: &str) -> Option<PathResult> {
        let mut visited = HashSet::new();
        let mut queue = vec![(source_id.to_string(), vec![source_id.to_string()], 0.0_f32)];

        while let Some((current, path, weight)) = queue.pop() {
            if current == target_id {
                let nodes: Vec<GraphNode> = path.iter()
                    .filter_map(|id| self.graph.get_node(id).cloned())
                    .collect();
                return Some(PathResult { nodes, total_weight: weight });
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            let connections = self.graph.find_connections(&current);
            for (node, _rel, edge_weight) in connections {
                let mut new_path = path.clone();
                new_path.push(node.id.clone());
                queue.push((node.id.clone(), new_path, weight + edge_weight));
            }
        }

        None
    }

    /// Find connected components
    pub fn find_communities(&self) -> Vec<Vec<&GraphNode>> {
        let mut visited = HashSet::new();
        let mut communities = Vec::new();

        for node in self.graph.nodes.values() {
            if visited.contains(&node.id) {
                continue;
            }

            let mut community = Vec::new();
            let mut stack = vec![node.id.clone()];

            while let Some(current) = stack.pop() {
                if visited.contains(&current) {
                    continue;
                }
                visited.insert(current.clone());

                if let Some(current_node) = self.graph.get_node(&current) {
                    community.push(current_node);
                    let connections = self.graph.find_connections(&current);
                    for (conn, _, _) in connections {
                        if !visited.contains(&conn.id) {
                            stack.push(conn.id.clone());
                        }
                    }
                }
            }

            if !community.is_empty() {
                communities.push(community);
            }
        }

        communities
    }

    /// Find central nodes (most connected)
    pub fn find_central_nodes(&self, top_k: usize) -> Vec<(&GraphNode, usize)> {
        let mut centrality: Vec<(&GraphNode, usize)> = self.graph.nodes.values()
            .map(|node| {
                let degree = self.graph.find_connections(&node.id).len();
                (node, degree)
            })
            .collect();

        centrality.sort_by(|a, b| b.1.cmp(&a.1));
        centrality.truncate(top_k);
        centrality
    }
}