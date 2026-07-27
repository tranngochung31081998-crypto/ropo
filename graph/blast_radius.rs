//! Fork của graphify-8 `affected.py` — Blast Radius (phạm vi ảnh hưởng).
//!
//! Port nguyên logic:
//! - `resolve_seed`: fuzzy node resolution (exact id → exact label → bare callable
//!   name → source_file → _prefer_file_node → contains)
//! - `affected_nodes`: reverse BFS walk (ai bị ảnh hưởng nếu SEED thay đổi?),
//!   filter theo relation, seed thêm member nodes qua `method`/`contains` (#1669)
//! - `format_affected`: text report
//!
//! Thêm cho CULI: `explain_node` (port lệnh `graphify explain`).

use std::collections::{HashSet, VecDeque};
use unicode_normalization::UnicodeNormalization;

use super::{GraphNode, KnowledgeGraph};

/// Relations được walk khi tính blast radius (port DEFAULT_AFFECTED_RELATIONS).
pub const DEFAULT_AFFECTED_RELATIONS: &[&str] = &[
    "calls",
    "indirect_call",
    "references",
    "imports",
    "imports_from",
    "re_exports",
    "inherits",
    "extends",
    "implements",
    "uses",
    "mixes_in",
    "embeds",
];

#[derive(Debug, Clone)]
pub struct AffectedHit {
    pub node_id: String,
    pub depth: usize,
    pub via_relation: String,
}

// ---------------------------------------------------------------------------
// Label helpers
// ---------------------------------------------------------------------------

fn normalize_label(s: &str) -> String {
    let nfc: String = s.nfc().collect();
    nfc.to_lowercase()
}

/// Lowercased label bỏ callable decoration `()` ở cuối.
fn bare_name(label: &str) -> String {
    let n = normalize_label(label);
    n.strip_suffix("()").map(|s| s.to_string()).unwrap_or(n)
}

fn prop<'a>(node: &'a GraphNode, key: &str) -> &'a str {
    node.properties.get(key).map(|s| s.as_str()).unwrap_or("")
}

fn node_label(graph: &KnowledgeGraph, node_id: &str) -> String {
    graph
        .get_node(node_id)
        .map(|n| n.label.clone())
        .unwrap_or_else(|| node_id.to_string())
}

fn format_location(node: &GraphNode) -> String {
    let file = prop(node, "source_file");
    let loc = prop(node, "source_location");
    match (file.is_empty(), loc.is_empty()) {
        (false, false) => format!("{}:{}", file, loc),
        (false, true) => file.to_string(),
        _ => prop(node, "origin_file").to_string(),
    }
}

// ---------------------------------------------------------------------------
// resolve_seed (port affected.py)
// ---------------------------------------------------------------------------

/// Khi query là source_file khớp nhiều nodes: ưu tiên file-level node (L1).
fn prefer_file_node(graph: &KnowledgeGraph, node_ids: &[String], query: &str) -> Option<String> {
    let query_basename = normalize_label(query.rsplit('/').next().unwrap_or(query));

    let exact_file: Vec<&String> = node_ids
        .iter()
        .filter(|id| {
            graph.get_node(id).map(|n| {
                prop(n, "source_location") == "L1" && normalize_label(&n.label) == query_basename
            }).unwrap_or(false)
        })
        .collect();
    if exact_file.len() == 1 {
        return Some(exact_file[0].clone());
    }

    let l1_nodes: Vec<&String> = node_ids
        .iter()
        .filter(|id| graph.get_node(id).map(|n| prop(n, "source_location") == "L1").unwrap_or(false))
        .collect();
    if l1_nodes.len() == 1 {
        return Some(l1_nodes[0].clone());
    }

    let basename_nodes: Vec<&String> = node_ids
        .iter()
        .filter(|id| graph.get_node(id).map(|n| normalize_label(&n.label) == query_basename).unwrap_or(false))
        .collect();
    if basename_nodes.len() == 1 {
        return Some(basename_nodes[0].clone());
    }

    None
}

/// Resolve query string về node id duy nhất. None nếu không match hoặc ambiguous.
pub fn resolve_seed(graph: &KnowledgeGraph, query: &str) -> Option<String> {
    let query = query.trim_end_matches(['/', '\\']);
    if query.is_empty() {
        return None;
    }
    // 1. Exact node id
    if graph.get_node(query).is_some() {
        return Some(query.to_string());
    }
    let query_lower = normalize_label(query);

    // 2. Exact label match
    let exact: Vec<String> = graph
        .nodes()
        .filter(|n| normalize_label(&n.label) == query_lower)
        .map(|n| n.id.clone())
        .collect();
    if exact.len() == 1 {
        return Some(exact[0].clone());
    }

    // 3. Bare callable name ("name" khớp "name()")
    let query_bare = bare_name(query);
    let bare_matches: Vec<String> = graph
        .nodes()
        .filter(|n| bare_name(&n.label) == query_bare)
        .map(|n| n.id.clone())
        .collect();
    if bare_matches.len() == 1 {
        return Some(bare_matches[0].clone());
    }

    // 4. Source file match
    let source_matches: Vec<String> = graph
        .nodes()
        .filter(|n| normalize_label(prop(n, "source_file")) == query_lower)
        .map(|n| n.id.clone())
        .collect();
    if source_matches.len() == 1 {
        return Some(source_matches[0].clone());
    }
    if !source_matches.is_empty() {
        if let Some(preferred) = prefer_file_node(graph, &source_matches, query) {
            return Some(preferred);
        }
    }

    // 5. Contains match
    let contains: Vec<String> = graph
        .nodes()
        .filter(|n| normalize_label(&n.label).contains(&query_lower))
        .map(|n| n.id.clone())
        .collect();
    if contains.len() == 1 {
        return Some(contains[0].clone());
    }

    None
}

// ---------------------------------------------------------------------------
// affected_nodes (port affected.py — reverse BFS + member seeding #1669)
// ---------------------------------------------------------------------------

pub fn affected_nodes(
    graph: &KnowledgeGraph,
    seed: &str,
    relations: &[&str],
    depth: usize,
) -> Vec<AffectedHit> {
    let relation_set: HashSet<&str> = relations.iter().copied().collect();
    let mut seen: HashSet<String> = HashSet::from([seed.to_string()]);
    let mut queue: VecDeque<(String, usize)> = VecDeque::from([(seed.to_string(), 0)]);
    let mut hits: Vec<AffectedHit> = Vec::new();

    // Member seeding: caller có thể bind vào method node thay vì class node,
    // nên seed thêm các member (1 hop `method`/`contains` đi ra) ở depth 0.
    // Member nodes chỉ là seeds, KHÔNG report làm hits.
    for e in graph.edges().iter().filter(|e| {
        e.source_id == seed && (e.relationship == "method" || e.relationship == "contains")
    }) {
        if seen.insert(e.target_id.clone()) {
            queue.push_back((e.target_id.clone(), 0));
        }
    }

    while let Some((current, current_depth)) = queue.pop_front() {
        if current_depth >= depth {
            continue;
        }
        // Reverse walk: ai trỏ TỚI current qua relations được chọn?
        for e in graph.edges().iter().filter(|e| {
            e.target_id == current && relation_set.contains(e.relationship.as_str())
        }) {
            if seen.insert(e.source_id.clone()) {
                hits.push(AffectedHit {
                    node_id: e.source_id.clone(),
                    depth: current_depth + 1,
                    via_relation: e.relationship.clone(),
                });
                queue.push_back((e.source_id.clone(), current_depth + 1));
            }
        }
    }

    hits
}

// ---------------------------------------------------------------------------
// Reports
// ---------------------------------------------------------------------------

pub struct AffectedReport {
    pub seed_id: String,
    pub seed_label: String,
    pub hits: Vec<AffectedHit>,
    pub text: String,
}

/// Blast radius report. None nếu không resolve được seed duy nhất.
pub fn affected_report(graph: &KnowledgeGraph, query: &str, depth: usize) -> Option<AffectedReport> {
    let seed = resolve_seed(graph, query)?;
    let seed_label = node_label(graph, &seed);
    let hits = affected_nodes(graph, &seed, DEFAULT_AFFECTED_RELATIONS, depth);

    let mut lines = vec![
        format!("Blast Radius: {}", seed_label),
        format!("Relations: {}", DEFAULT_AFFECTED_RELATIONS.join(", ")),
        format!("Depth: {}", depth),
    ];
    if hits.is_empty() {
        lines.push("Không có node nào bị ảnh hưởng.".to_string());
    } else {
        for hit in &hits {
            let label = node_label(graph, &hit.node_id);
            let loc = graph.get_node(&hit.node_id).map(format_location).unwrap_or_default();
            lines.push(format!(
                "- {} [{}] (depth {}) {}",
                label, hit.via_relation, hit.depth, loc
            ));
        }
    }

    Some(AffectedReport {
        seed_id: seed,
        seed_label,
        hits,
        text: lines.join("\n"),
    })
}

/// Port lệnh `graphify explain`: chi tiết node + toàn bộ connections 2 chiều.
pub fn explain_node(graph: &KnowledgeGraph, query: &str, max_connections: usize) -> Option<String> {
    let seed = resolve_seed(graph, query)?;
    let node = graph.get_node(&seed)?;
    let loc = format_location(node);

    let mut out_edges: Vec<String> = Vec::new();
    let mut in_edges: Vec<String> = Vec::new();
    for e in graph.edges() {
        if e.source_id == seed {
            out_edges.push(format!(
                "  --> {} [{}] [{:?}]",
                node_label(graph, &e.target_id),
                e.relationship,
                e.confidence
            ));
        } else if e.target_id == seed {
            in_edges.push(format!(
                "  <-- {} [{}] [{:?}]",
                node_label(graph, &e.source_id),
                e.relationship,
                e.confidence
            ));
        }
    }

    let degree = out_edges.len() + in_edges.len();
    let mut lines = vec![
        format!("Node: {} [{:?}]", node.label, node.node_type),
        format!("Source: {}", if loc.is_empty() { "-" } else { &loc }),
        format!("Degree: {}", degree),
        format!("Connections ({}):", degree),
    ];
    for l in out_edges.iter().chain(in_edges.iter()).take(max_connections) {
        lines.push(l.clone());
    }
    if degree > max_connections {
        lines.push(format!("  ... ({} more)", degree - max_connections));
    }
    Some(lines.join("\n"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{EdgeConfidence, GraphEdge, GraphNode, NodeType};
    use std::collections::HashMap;

    fn node(id: &str, label: &str, file: &str, loc: &str) -> GraphNode {
        let mut properties = HashMap::new();
        properties.insert("source_file".to_string(), file.to_string());
        properties.insert("source_location".to_string(), loc.to_string());
        GraphNode {
            id: id.to_string(),
            label: label.to_string(),
            node_type: if loc == "L1" { NodeType::File } else { NodeType::Code },
            properties,
            confidence: 1.0,
            source: "test".to_string(),
        }
    }

    fn edge(src: &str, tgt: &str, rel: &str) -> GraphEdge {
        GraphEdge {
            source_id: src.to_string(),
            target_id: tgt.to_string(),
            relationship: rel.to_string(),
            weight: 1.0,
            confidence: EdgeConfidence::Extracted,
        }
    }

    /// Graph mẫu:
    /// ```text
    /// main.rs (file) --contains--> main()
    /// main() --calls--> helper()
    /// render() --calls--> helper()
    /// Cart --method--> .total()
    /// checkout() --calls--> .total()
    /// helper() --references--> Config
    /// stray() --contains_x--> helper()   (relation không thuộc default set)
    /// ```
    fn sample_graph() -> KnowledgeGraph {
        let mut g = KnowledgeGraph::new();
        g.add_node(node("src_main_rs", "main.rs", "src/main.rs", "L1")).unwrap();
        g.add_node(node("src_main_main", "main()", "src/main.rs", "L5")).unwrap();
        g.add_node(node("src_lib_helper", "helper()", "src/lib.rs", "L3")).unwrap();
        g.add_node(node("src_lib_render", "render()", "src/lib.rs", "L9")).unwrap();
        g.add_node(node("src_cart_cart", "Cart", "src/cart.rs", "L2")).unwrap();
        g.add_node(node("src_cart_cart_total", ".total()", "src/cart.rs", "L7")).unwrap();
        g.add_node(node("src_app_checkout", "checkout()", "src/app.rs", "L4")).unwrap();
        g.add_node(node("config", "Config", "", "")).unwrap();
        g.add_node(node("src_lib_stray", "stray()", "src/lib.rs", "L20")).unwrap();

        g.add_edge(edge("src_main_rs", "src_main_main", "contains")).unwrap();
        g.add_edge(edge("src_main_main", "src_lib_helper", "calls")).unwrap();
        g.add_edge(edge("src_lib_render", "src_lib_helper", "calls")).unwrap();
        g.add_edge(edge("src_cart_cart", "src_cart_cart_total", "method")).unwrap();
        g.add_edge(edge("src_app_checkout", "src_cart_cart_total", "calls")).unwrap();
        g.add_edge(edge("src_lib_helper", "config", "references")).unwrap();
        g.add_edge(edge("src_lib_stray", "src_lib_helper", "contains_x")).unwrap();
        g
    }

    #[test]
    fn test_resolve_seed_exact_id() {
        let g = sample_graph();
        assert_eq!(resolve_seed(&g, "src_lib_helper"), Some("src_lib_helper".to_string()));
    }

    #[test]
    fn test_resolve_seed_bare_callable() {
        let g = sample_graph();
        // "helper" (không "()") phải khớp "helper()"
        assert_eq!(resolve_seed(&g, "helper"), Some("src_lib_helper".to_string()));
        // Case-insensitive
        assert_eq!(resolve_seed(&g, "HELPER"), Some("src_lib_helper".to_string()));
    }

    #[test]
    fn test_resolve_seed_source_file_prefers_file_node() {
        let g = sample_graph();
        assert_eq!(resolve_seed(&g, "src/main.rs"), Some("src_main_rs".to_string()));
    }

    #[test]
    fn test_resolve_seed_ambiguous_returns_none() {
        let g = sample_graph();
        // "()" chứa trong nhiều labels → None
        assert_eq!(resolve_seed(&g, "()"), None);
        // "src/lib.rs" khớp 3 nodes cùng source_file: file node lib.rs không có
        // trong graph (chỉ có helper/render/stray) → prefer_file_node fail → None
        assert_eq!(resolve_seed(&g, "src/lib.rs"), None);
    }

    #[test]
    fn test_affected_nodes_reverse_walk() {
        let g = sample_graph();
        let hits = affected_nodes(&g, "src_lib_helper", DEFAULT_AFFECTED_RELATIONS, 2);
        let ids: HashSet<&str> = hits.iter().map(|h| h.node_id.as_str()).collect();
        // ai gọi helper()? main() và render() ở depth 1
        assert!(ids.contains("src_main_main"));
        assert!(ids.contains("src_lib_render"));
        // depth 2: main.rs (contains — KHÔNG thuộc default relations, không lan)
        assert!(!ids.contains("src_main_rs"));
        // stray() dùng relation "contains_x" không thuộc set → không walk
        assert!(!ids.contains("src_lib_stray"));
        // helper trỏ TỚI Config (chiều xuôi) → Config KHÔNG bị ảnh hưởng bởi helper
        assert!(!ids.contains("config"));
    }

    #[test]
    fn test_affected_nodes_member_seeding() {
        let g = sample_graph();
        // Blast radius của Cart phải bao gồm checkout() vì checkout gọi .total()
        // (member của Cart) — logic #1669
        let hits = affected_nodes(&g, "src_cart_cart", DEFAULT_AFFECTED_RELATIONS, 2);
        let ids: HashSet<&str> = hits.iter().map(|h| h.node_id.as_str()).collect();
        assert!(ids.contains("src_app_checkout"));
        // Member node .total() KHÔNG xuất hiện như 1 hit
        assert!(!ids.contains("src_cart_cart_total"));
    }

    #[test]
    fn test_affected_nodes_depth_limit() {
        let g = sample_graph();
        // depth 1: chỉ callers trực tiếp
        let hits = affected_nodes(&g, "src_lib_helper", DEFAULT_AFFECTED_RELATIONS, 1);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn test_affected_report_format() {
        let g = sample_graph();
        let report = affected_report(&g, "helper", 2).unwrap();
        assert!(report.text.contains("Blast Radius: helper()"));
        assert!(report.text.contains("main()"));
        assert_eq!(report.hits.len(), 2);
        // Query không resolve được → None
        assert!(affected_report(&g, "nonexistent_xyz", 2).is_none());
    }

    #[test]
    fn test_explain_node() {
        let g = sample_graph();
        let text = explain_node(&g, "helper", 50).unwrap();
        assert!(text.contains("Node: helper()"));
        assert!(text.contains("Source: src/lib.rs:L3"));
        assert!(text.contains("--> Config [references]"));
        assert!(text.contains("<-- main() [calls]"));
        assert!(text.contains("Degree: 4"));
    }
}
