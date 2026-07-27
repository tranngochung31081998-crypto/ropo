use anyhow::Result;
use rusqlite::{Connection, params, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;
use super::{GraphNode, GraphEdge, NodeType, EdgeConfidence};

/// SQLite-backed graph persistence with FTS5 search
/// Inspired by code-review-graph's persistence layer
pub struct GraphStorage {
    conn: Connection,
}

impl GraphStorage {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let storage = Self { conn };
        storage.initialize_schema()?;
        Ok(storage)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let storage = Self { conn };
        storage.initialize_schema()?;
        Ok(storage)
    }

    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS graph_nodes (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                node_type TEXT NOT NULL,
                properties TEXT NOT NULL DEFAULT '{}',
                confidence REAL NOT NULL DEFAULT 1.0,
                source TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS graph_edges (
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relationship TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0,
                confidence TEXT NOT NULL DEFAULT 'Extracted',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (source_id, target_id, relationship),
                FOREIGN KEY (source_id) REFERENCES graph_nodes(id),
                FOREIGN KEY (target_id) REFERENCES graph_nodes(id)
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS graph_nodes_fts 
            USING fts5(id, label, properties, node_type, content='graph_nodes', content_rowid='rowid');

            CREATE INDEX IF NOT EXISTS idx_edges_source ON graph_edges(source_id);
            CREATE INDEX IF NOT EXISTS idx_edges_target ON graph_edges(target_id);
            CREATE INDEX IF NOT EXISTS idx_nodes_type ON graph_nodes(node_type);"
        )?;
        Ok(())
    }

    pub fn save_node(&self, node: &GraphNode) -> Result<()> {
        let properties = serde_json::to_string(&node.properties)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO graph_nodes (id, label, node_type, properties, confidence, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![node.id, node.label, format!("{:?}", node.node_type), properties, node.confidence, node.source],
        )?;

        // Update FTS index
        self.conn.execute(
            "INSERT OR REPLACE INTO graph_nodes_fts (id, label, properties, node_type)
             VALUES (?1, ?2, ?3, ?4)",
            params![node.id, node.label, properties, format!("{:?}", node.node_type)],
        )?;
        Ok(())
    }

    pub fn save_edge(&self, edge: &GraphEdge) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO graph_edges (source_id, target_id, relationship, weight, confidence)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![edge.source_id, edge.target_id, edge.relationship, edge.weight, format!("{:?}", edge.confidence)],
        )?;
        Ok(())
    }

    pub fn load_all_nodes(&self) -> Result<Vec<GraphNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, node_type, properties, confidence, source FROM graph_nodes"
        )?;
        let nodes = stmt.query_map([], |row| {
            let node_type_str: String = row.get(2)?;
            let properties_str: String = row.get(3)?;
            let properties: HashMap<String, String> = serde_json::from_str(&properties_str).unwrap_or_default();
            let node_type = match node_type_str.as_str() {
                "Concept" => NodeType::Concept,
                "Entity" => NodeType::Entity,
                "Action" => NodeType::Action,
                "Pattern" => NodeType::Pattern,
                "File" => NodeType::File,
                "Code" => NodeType::Code,
                "Agent" => NodeType::Agent,
                "Task" => NodeType::Task,
                _ => NodeType::Concept,
            };
            Ok(GraphNode {
                id: row.get(0)?,
                label: row.get(1)?,
                node_type,
                properties,
                confidence: row.get(4)?,
                source: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    pub fn load_all_edges(&self) -> Result<Vec<GraphEdge>> {
        let mut stmt = self.conn.prepare(
            "SELECT source_id, target_id, relationship, weight, confidence FROM graph_edges"
        )?;
        let edges = stmt.query_map([], |row| {
            let confidence_str: String = row.get(4)?;
            let confidence = match confidence_str.as_str() {
                "Extracted" => EdgeConfidence::Extracted,
                "Inferred" => EdgeConfidence::Inferred,
                "Ambiguous" => EdgeConfidence::Ambiguous,
                "Learned" => EdgeConfidence::Learned,
                _ => EdgeConfidence::Extracted,
            };
            Ok(GraphEdge {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                relationship: row.get(2)?,
                weight: row.get(3)?,
                confidence,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(edges)
    }

    /// FTS5 full-text search over graph nodes
    pub fn fts_search(&self, query: &str, limit: usize) -> Result<Vec<(GraphNode, f32)>> {
        let sql = format!(
            "SELECT n.id, n.label, n.node_type, n.properties, n.confidence, n.source,
                    rank
             FROM graph_nodes_fts f
             JOIN graph_nodes n ON n.id = f.id
             WHERE graph_nodes_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let results = stmt.query_map(params![query, limit as i64], |row| {
            let properties_str: String = row.get(3)?;
            let properties: HashMap<String, String> = serde_json::from_str(&properties_str).unwrap_or_default();
            let node_type_str: String = row.get(2)?;
            let node_type = match node_type_str.as_str() {
                "Concept" => NodeType::Concept,
                "Entity" => NodeType::Entity,
                "Action" => NodeType::Action,
                "Pattern" => NodeType::Pattern,
                "File" => NodeType::File,
                "Code" => NodeType::Code,
                "Agent" => NodeType::Agent,
                "Task" => NodeType::Task,
                _ => NodeType::Concept,
            };
            let rank: f32 = row.get(6)?;
            Ok((GraphNode {
                id: row.get(0)?,
                label: row.get(1)?,
                node_type,
                properties,
                confidence: row.get(4)?,
                source: row.get(5)?,
            }, rank))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }

    /// Impact radius: find all nodes within N hops from a start node
    pub fn find_impact_radius(&self, node_id: &str, max_hops: usize) -> Result<Vec<(GraphNode, usize, f32)>> {
        let mut results: Vec<(GraphNode, usize, f32)> = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue: Vec<(String, usize, f32)> = vec![(node_id.to_string(), 0, 0.0)];

        while let Some((current_id, hops, total_weight)) = queue.pop() {
            if hops > max_hops || visited.contains(&current_id) {
                continue;
            }
            visited.insert(current_id.clone());

            // Tìm node hiện tại
            let mut stmt = self.conn.prepare(
                "SELECT id, label, node_type, properties, confidence, source FROM graph_nodes WHERE id = ?1"
            )?;
            if let Some(node_row) = stmt.query_row(params![current_id], |row| {
                let properties_str: String = row.get(3)?;
                let properties: HashMap<String, String> = serde_json::from_str(&properties_str).unwrap_or_default();
                let node_type_str: String = row.get(2)?;
                let node_type = match node_type_str.as_str() {
                    "Concept" => NodeType::Concept,
                    "Entity" => NodeType::Entity,
                    "Action" => NodeType::Action,
                    "Pattern" => NodeType::Pattern,
                    "File" => NodeType::File,
                    "Code" => NodeType::Code,
                    "Agent" => NodeType::Agent,
                    "Task" => NodeType::Task,
                    _ => NodeType::Concept,
                };
                Ok(GraphNode {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    node_type,
                    properties,
                    confidence: row.get(4)?,
                    source: row.get(5)?,
                })
            }).optional()? {
                results.push((node_row, hops, total_weight));
            }

            if hops < max_hops {
                // Find neighbors
                let mut edge_stmt = self.conn.prepare(
                    "SELECT target_id, weight FROM graph_edges WHERE source_id = ?1
                     UNION
                     SELECT source_id, weight FROM graph_edges WHERE target_id = ?1"
                )?;
                let neighbors: Vec<(String, f32)> = edge_stmt.query_map(params![current_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, f32>(1)?))
                })?.collect::<Result<Vec<_>, _>>()?;

                for (neighbor_id, weight) in neighbors {
                    if !visited.contains(&neighbor_id) {
                        queue.push((neighbor_id, hops + 1, total_weight + weight));
                    }
                }
            }
        }

        Ok(results)
    }

    /// Hub node detection: find nodes with most connections
    pub fn find_hub_nodes(&self, top_k: usize) -> Result<Vec<(GraphNode, usize)>> {
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.label, n.node_type, n.properties, n.confidence, n.source,
                    (SELECT COUNT(*) FROM graph_edges e WHERE e.source_id = n.id OR e.target_id = n.id) as degree
             FROM graph_nodes n
             ORDER BY degree DESC
             LIMIT ?1"
        )?;
        let hubs = stmt.query_map(params![top_k as i64], |row| {
            let properties_str: String = row.get(3)?;
            let properties: HashMap<String, String> = serde_json::from_str(&properties_str).unwrap_or_default();
            let node_type_str: String = row.get(2)?;
            let node_type = match node_type_str.as_str() {
                "Concept" => NodeType::Concept,
                "Entity" => NodeType::Entity,
                "Action" => NodeType::Action,
                "Pattern" => NodeType::Pattern,
                "File" => NodeType::File,
                "Code" => NodeType::Code,
                "Agent" => NodeType::Agent,
                "Task" => NodeType::Task,
                _ => NodeType::Concept,
            };
            let degree: usize = row.get(6)?;
            Ok((GraphNode {
                id: row.get(0)?,
                label: row.get(1)?,
                node_type,
                properties,
                confidence: row.get(4)?,
                source: row.get(5)?,
            }, degree))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(hubs)
    }

    /// Delete all data
    pub fn clear(&self) -> Result<()> {
        self.conn.execute_batch(
            "DELETE FROM graph_nodes_fts;
             DELETE FROM graph_edges;
             DELETE FROM graph_nodes;"
        )?;
        Ok(())
    }
}
