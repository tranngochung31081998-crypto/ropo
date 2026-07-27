//! Fork tùy chỉnh của graphify-8 `extract.py` + `detect.py` + `extractors/`.
//!
//! Giữ nguyên của bản gốc:
//! - Extraction schema: `{nodes, edges, raw_calls}` với confidence EXTRACTED/INFERRED
//! - ID recipe qua `ids.rs`, label convention (`name()`, `.method()`, file node L1)
//! - Hai pass: (1) structural extraction per-file, (2) corpus-wide raw_calls → INFERRED
//! - Builtins/method blocklist chống god-node
//!
//! Tùy chỉnh cho CULI:
//! - KHÔNG dùng tree-sitter (tránh native deps, build Windows nặng): line-based
//!   heuristic parser với brace-depth (Rust/JS) và indentation (Python) tracking.
//!   graphify gốc cũng có regex extractors cho Apex/PowerShell nên cách này vẫn
//!   đúng triết lý "deterministic, local, no LLM".
//! - Ngôn ngữ trọng điểm của workspace CULI: Rust, JS/TS, Python, Markdown.
//! - Stub rewire: stub node (từ type-ref/import) collapse về real definition
//!   hoặc file node khi match duy nhất toàn corpus.

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use walkdir::WalkDir;

use super::ids::make_id;
use super::{EdgeConfidence, GraphEdge, GraphNode, KnowledgeGraph, NodeType};

// ---------------------------------------------------------------------------
// Extraction schema (giữ parity với graphify)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedNode {
    pub id: String,
    pub label: String,
    pub file_type: String,
    pub source_file: String,
    pub source_location: String,
    /// Chỉ có ở stub nodes (source_file rỗng): file đã tham chiếu tới nó.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub confidence: String,
    pub source_file: String,
    pub source_location: String,
    pub weight: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RawCall {
    pub caller_nid: String,
    pub callee: String,
    pub is_member_call: bool,
    pub source_file: String,
    pub source_location: String,
}

#[derive(Debug, Default)]
pub struct Extraction {
    pub nodes: Vec<ExtractedNode>,
    pub edges: Vec<ExtractedEdge>,
    pub raw_calls: Vec<RawCall>,
}

// ---------------------------------------------------------------------------
// Blocklists (port _LANGUAGE_BUILTIN_GLOBALS + _RUST_TRAIT_METHOD_BLOCKLIST)
// ---------------------------------------------------------------------------

fn language_builtins() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        [
            // JS/TS built-ins
            "String", "Number", "Boolean", "Object", "Array", "Symbol", "BigInt",
            "Date", "RegExp", "Error", "TypeError", "RangeError", "SyntaxError",
            "ReferenceError", "EvalError", "URIError", "Promise", "Map", "Set",
            "WeakMap", "WeakSet", "JSON", "Math", "Reflect", "Proxy", "Intl",
            "parseInt", "parseFloat", "isNaN", "isFinite", "encodeURIComponent",
            "decodeURIComponent", "encodeURI", "decodeURI", "URL", "URLSearchParams",
            "FormData", "Blob", "File", "Headers", "Request", "Response",
            "AbortController", "AbortSignal", "TextEncoder", "TextDecoder", "console",
            // Python built-ins
            "str", "int", "float", "bool", "list", "dict", "set", "tuple", "bytes",
            "len", "range", "enumerate", "zip", "map", "filter", "sum", "min", "max",
            "print", "open", "isinstance", "type", "super", "sorted", "reversed",
            "any", "all", "abs", "round", "next", "iter", "hash", "id", "repr",
            "callable", "getattr", "setattr", "hasattr", "delattr", "vars", "dir",
        ]
        .into_iter()
        .collect()
    })
}

/// Method names phổ biến → chỉ chặn khi KHÔNG resolve same-file được
/// (same-file EXTRACTED match vẫn thắng, đúng thứ tự ưu tiên của graphify).
fn method_blocklist() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        [
            "new", "default", "parse", "from_str", "now", "clone", "into", "from",
            "to_string", "to_owned", "len", "is_empty", "iter", "next", "build",
            "start", "run", "init", "app", "get", "set", "push", "pop", "insert",
            "remove", "contains", "collect", "map", "filter", "unwrap", "expect",
            "ok", "err", "some", "none", "send", "recv", "lock", "read", "write",
            // JS array/promise/common
            "foreach", "then", "catch", "finally", "find", "every", "reduce",
            "includes", "indexof", "slice", "splice", "concat", "join", "split",
            "trim", "replace", "match", "test", "exec", "keys", "values", "entries",
            "stringify", "log", "warn", "error", "info", "debug", "append",
            "startswith", "endswith", "tolowercase", "touppercase", "sort", "reverse",
            // Python common
            "add", "update", "format", "encode", "decode", "close", "save", "load",
        ]
        .into_iter()
        .collect()
    })
}

/// Rust std types — skip khi collect type references (chống god-node).
fn rust_std_types() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        [
            "Self", "Vec", "String", "Option", "Result", "Box", "Rc", "Arc", "Cell",
            "RefCell", "Mutex", "RwLock", "HashMap", "HashSet", "BTreeMap", "BTreeSet",
            "VecDeque", "LinkedList", "Cow", "Pin", "Weak", "Duration", "Instant",
        ]
        .into_iter()
        .collect()
    })
}

fn rust_keywords() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        [
            "if", "for", "while", "match", "loop", "return", "break", "continue",
            "let", "fn", "struct", "enum", "impl", "trait", "use", "mod", "pub",
            "const", "static", "type", "where", "else", "move", "ref", "mut", "self",
            "super", "crate", "dyn", "async", "await", "unsafe", "extern", "in", "as",
            "Some", "Ok", "Err", "None", "true", "false",
        ]
        .into_iter()
        .collect()
    })
}

fn js_keywords() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        [
            "if", "for", "while", "switch", "catch", "return", "function", "else",
            "do", "new", "typeof", "instanceof", "in", "of", "await", "async",
            "yield", "delete", "void", "throw", "case", "super", "this", "class",
            "extends", "const", "let", "var", "import", "export", "default", "try",
            "finally", "break", "continue", "require",
        ]
        .into_iter()
        .collect()
    })
}

fn python_keywords() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        [
            "if", "for", "while", "return", "def", "class", "import", "from", "elif",
            "else", "except", "with", "as", "lambda", "yield", "raise", "assert",
            "pass", "break", "continue", "and", "or", "not", "in", "is", "None",
            "True", "False", "self", "cls", "global", "nonlocal", "del", "try",
            "finally", "async", "await", "print",
        ]
        .into_iter()
        .collect()
    })
}

// ---------------------------------------------------------------------------
// Regex helpers
// ---------------------------------------------------------------------------

fn double_quoted_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#""(?:\\.|[^"\\])*""#).unwrap())
}

fn type_ident_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b([A-Z][\w]*)\b").unwrap())
}

fn scoped_call_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\w+)::(\w+)\s*\(").unwrap())
}

fn member_call_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\.(\w+)\s*\(").unwrap())
}

fn plain_call_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(\w+)\s*\(").unwrap())
}

/// Strip double-quoted strings + line comment khỏi 1 dòng (heuristic).
fn clean_line(line: &str) -> String {
    let no_str = double_quoted_re().replace_all(line, "\"\"");
    match no_str.find("//") {
        Some(pos) => no_str[..pos].to_string(),
        None => no_str.to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CallKind {
    Scoped,
    Member,
    Plain,
}

/// Tìm các call sites trong 1 dòng body. Thứ tự: scoped → member → plain,
/// blank out phần đã match để tránh đếm trùng.
fn scan_calls(line: &str) -> Vec<(String, CallKind)> {
    let mut out = Vec::new();
    let mut rest = line.to_string();

    for caps in scoped_call_re().captures_iter(&rest.clone()) {
        out.push((caps[2].to_string(), CallKind::Scoped));
    }
    rest = scoped_call_re().replace_all(&rest, " ").to_string();

    for caps in member_call_re().captures_iter(&rest.clone()) {
        out.push((caps[1].to_string(), CallKind::Member));
    }
    rest = member_call_re().replace_all(&rest, " ").to_string();

    for caps in plain_call_re().captures_iter(&rest) {
        out.push((caps[1].to_string(), CallKind::Plain));
    }
    out
}

/// Tìm dòng kết thúc block (dòng chứa `}` đóng tương ứng) tính từ `start`.
/// Trả None nếu gặp `;` trước `{` (declaration không body) hoặc hết file.
fn find_block_end(lines: &[&str], start: usize) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut opened = false;
    for (i, raw) in lines.iter().enumerate().skip(start) {
        let line = clean_line(raw);
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => {
                    depth -= 1;
                    if opened && depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        if !opened && line.contains(';') {
            return None;
        }
        if !opened && i > start + 10 {
            return None; // signature quá dài, bỏ qua an toàn
        }
    }
    None
}

fn bare_label(label: &str) -> String {
    label
        .trim_end_matches("()")
        .trim_start_matches('.')
        .to_lowercase()
}

fn last_segment(path: &str) -> &str {
    path.rsplit([':', '/']).next().unwrap_or(path)
}

fn file_name_of(source_file: &str) -> String {
    source_file
        .rsplit('/')
        .next()
        .unwrap_or(source_file)
        .to_string()
}

// ---------------------------------------------------------------------------
// Per-file extraction context
// ---------------------------------------------------------------------------

struct Ctx {
    source_file: String,
    stem: String,
    nodes: Vec<ExtractedNode>,
    edges: Vec<ExtractedEdge>,
    raw_calls: Vec<RawCall>,
    seen_ids: HashSet<String>,
    seen_edge_keys: HashSet<String>,
}

impl Ctx {
    fn new(source_file: &str, stem: &str) -> Self {
        Self {
            source_file: source_file.to_string(),
            stem: stem.to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
            raw_calls: Vec::new(),
            seen_ids: HashSet::new(),
            seen_edge_keys: HashSet::new(),
        }
    }

    fn add_node(&mut self, nid: String, label: String, line: usize, file_type: &str) {
        if self.seen_ids.insert(nid.clone()) {
            self.nodes.push(ExtractedNode {
                id: nid,
                label,
                file_type: file_type.to_string(),
                source_file: self.source_file.clone(),
                source_location: format!("L{}", line),
                origin_file: None,
            });
        }
    }

    /// Port `ensure_named_node`: nếu name đã defined trong file → trả id đó,
    /// ngược lại tạo/lấy SOURCELESS stub (corpus rewire sẽ collapse sau).
    fn ensure_named_node(&mut self, name: &str, _line: usize) -> String {
        let nid = make_id(&[&self.stem, name]);
        if self.seen_ids.contains(&nid) {
            return nid;
        }
        let nid = make_id(&[name]);
        if self.seen_ids.insert(nid.clone()) {
            self.nodes.push(ExtractedNode {
                id: nid.clone(),
                label: name.to_string(),
                file_type: "code".to_string(),
                source_file: String::new(),
                source_location: String::new(),
                origin_file: Some(self.source_file.clone()),
            });
        }
        nid
    }

    fn add_edge(
        &mut self,
        source: String,
        target: String,
        relation: &str,
        line: usize,
        confidence: &str,
        weight: f32,
        context: Option<&str>,
    ) {
        let key = format!("{}|{}|{}", source, target, relation);
        if !self.seen_edge_keys.insert(key) {
            return;
        }
        self.edges.push(ExtractedEdge {
            source,
            target,
            relation: relation.to_string(),
            confidence: confidence.to_string(),
            source_file: self.source_file.clone(),
            source_location: format!("L{}", line),
            weight,
            context: context.map(|c| c.to_string()),
        });
    }

    /// Same-file label index: bare label -> nid (chỉ real nodes, first-wins).
    fn label_index(&self) -> HashMap<String, String> {
        let mut index = HashMap::new();
        for n in &self.nodes {
            if n.source_file.is_empty() {
                continue; // stub không vào index
            }
            index.entry(bare_label(&n.label)).or_insert_with(|| n.id.clone());
        }
        index
    }

    /// Call pass dùng chung: duyệt body ranges, same-file → EXTRACTED calls,
    /// còn lại → raw_calls (corpus pass resolve thành INFERRED).
    fn run_call_pass(
        &mut self,
        bodies: &[(String, usize, usize)],
        lines: &[&str],
        keywords: &HashSet<&'static str>,
    ) {
        let label_index = self.label_index();
        let mut seen_extracted: HashSet<(String, String)> = HashSet::new();
        let mut seen_raw: HashSet<(String, String)> = HashSet::new();

        for (caller, start, end) in bodies {
            for ln in *start..=(*end).min(lines.len().saturating_sub(1)) {
                let line = clean_line(lines[ln]);
                for (callee, kind) in scan_calls(&line) {
                    if keywords.contains(callee.as_str())
                        || language_builtins().contains(callee.as_str())
                    {
                        continue;
                    }
                    let bare = callee.to_lowercase();
                    if let Some(tgt) = label_index.get(&bare) {
                        if tgt != caller && seen_extracted.insert((caller.clone(), tgt.clone())) {
                            self.edges.push(ExtractedEdge {
                                source: caller.clone(),
                                target: tgt.clone(),
                                relation: "calls".to_string(),
                                confidence: "EXTRACTED".to_string(),
                                source_file: self.source_file.clone(),
                                source_location: format!("L{}", ln + 1),
                                weight: 1.0,
                                context: Some("call".to_string()),
                            });
                        }
                    } else if kind != CallKind::Scoped && !method_blocklist().contains(bare.as_str()) {
                        // Scoped calls (Type::method) không cross-file resolve (#908 parity)
                        if seen_raw.insert((caller.clone(), bare.clone())) {
                            self.raw_calls.push(RawCall {
                                caller_nid: caller.clone(),
                                callee: bare,
                                is_member_call: kind == CallKind::Member,
                                source_file: self.source_file.clone(),
                                source_location: format!("L{}", ln + 1),
                            });
                        }
                    }
                }
            }
        }
    }

    fn finish(self) -> Extraction {
        let valid = &self.seen_ids;
        let edges = self
            .edges
            .into_iter()
            .filter(|e| valid.contains(&e.source) && valid.contains(&e.target))
            .collect();
        Extraction {
            nodes: self.nodes,
            edges,
            raw_calls: self.raw_calls,
        }
    }
}

/// Collect type identifiers (UpperCamel) trong 1 segment, bỏ std types + 1-letter generics.
fn collect_type_refs(segment: &str, skip_std: bool) -> Vec<String> {
    let mut out = Vec::new();
    for caps in type_ident_re().captures_iter(segment) {
        let name = caps[1].to_string();
        if name.len() <= 1 {
            continue;
        }
        if skip_std && rust_std_types().contains(name.as_str()) {
            continue;
        }
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Rust extractor (port extractors/rust.py — heuristic, không tree-sitter)
// ---------------------------------------------------------------------------

fn rust_use_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(?:pub\s+)?use\s+([^;]+);").unwrap())
}

fn rust_fn_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?(?:default\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]*"\s+)?fn\s+([A-Za-z_]\w*)"#,
        )
        .unwrap()
    })
}

fn rust_struct_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?struct\s+([A-Za-z_]\w*)").unwrap())
}

fn rust_enum_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?enum\s+([A-Za-z_]\w*)").unwrap())
}

fn rust_trait_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?(?:unsafe\s+)?trait\s+([A-Za-z_]\w*)\s*(?::\s*([^\{]+))?").unwrap()
    })
}

fn rust_impl_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^\s*(?:unsafe\s+)?impl(?:<[^>]*>)?\s+(?:(?P<trait>[A-Za-z_][\w:<>, ]*?)\s+for\s+)?(?P<ty>[A-Za-z_][\w:<>]*)",
        )
        .unwrap()
    })
}

fn rust_field_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?\w+\s*:\s*([^,]+),?\s*$").unwrap())
}

fn rust_tuple_variant_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*\w+\s*\(([^)]*)\)").unwrap())
}

pub fn extract_rust(source_file: &str, stem: &str, content: &str) -> Extraction {
    let mut ctx = Ctx::new(source_file, stem);
    let lines: Vec<&str> = content.lines().collect();
    let file_nid = make_id(&[source_file]);
    ctx.add_node(file_nid.clone(), file_name_of(source_file), 1, "code");

    let mut bodies: Vec<(String, usize, usize)> = Vec::new();
    // impl_stack: (impl_nid, end_line) — end_line là dòng `}` đóng impl/trait block
    let mut impl_stack: Vec<(String, usize)> = Vec::new();

    let mut i = 0usize;
    while i < lines.len() {
        let line = clean_line(lines[i]);
        let trimmed = line.trim().to_string();

        // Pop impl/trait contexts đã đóng (i đã vượt qua end_line của chúng)
        while let Some(&(_, end_line)) = impl_stack.last() {
            if i > end_line {
                impl_stack.pop();
            } else {
                break;
            }
        }

        if !trimmed.is_empty() {
            if let Some(caps) = rust_use_re().captures(&trimmed) {
                let raw = caps[1].trim();
                let clean = raw.split('{').next().unwrap_or(raw);
                let clean = clean.trim_end_matches(':').trim_end_matches('*').trim_end_matches(':');
                let module_name = clean.split("::").last().unwrap_or("").trim();
                if !module_name.is_empty() {
                    let tgt = ctx.ensure_named_node(module_name, i + 1);
                    ctx.add_edge(file_nid.clone(), tgt, "imports_from", i + 1, "EXTRACTED", 1.0, Some("import"));
                }
            } else if let Some(caps) = rust_impl_re().captures(&trimmed) {
                let ty = caps.name("ty").map(|m| m.as_str()).unwrap_or("");
                let line_no = i + 1;
                let impl_nid = make_id(&[stem, ty]);
                ctx.add_node(impl_nid.clone(), ty.to_string(), line_no, "code");
                if let Some(tr) = caps.name("trait") {
                    let tname = last_segment(tr.as_str().trim()).to_string();
                    let tgt = ctx.ensure_named_node(&tname, line_no);
                    if tgt != impl_nid {
                        ctx.add_edge(impl_nid.clone(), tgt, "implements", line_no, "EXTRACTED", 1.0, None);
                    }
                }
                // Push context — dùng find_block_end để lấy end_line chính xác
                if line.matches('{').count() > 0 {
                    if let Some(end) = find_block_end(&lines, i) {
                        impl_stack.push((impl_nid, end));
                    }
                }
            } else if let Some(caps) = rust_struct_re().captures(&trimmed) {
                let name = caps[1].to_string();
                let line_no = i + 1;
                let nid = make_id(&[stem, &name]);
                ctx.add_node(nid.clone(), name.clone(), line_no, "code");
                ctx.add_edge(file_nid.clone(), nid.clone(), "contains", line_no, "EXTRACTED", 1.0, None);
                // Field type refs trong block
                if let Some(end) = find_block_end(&lines, i) {
                    for ln in (i + 1)..end {
                        let fline = clean_line(lines[ln]);
                        if let Some(fcaps) = rust_field_re().captures(fline.trim()) {
                            for tref in collect_type_refs(&fcaps[1], true) {
                                let tgt = ctx.ensure_named_node(&tref, ln + 1);
                                if tgt != nid {
                                    ctx.add_edge(nid.clone(), tgt, "references", ln + 1, "EXTRACTED", 1.0, Some("field"));
                                }
                            }
                        }
                    }
                    i = end; // skip block
                } else {
                    // Tuple struct 1 dòng: `struct Wrapper(pub Logger, Config);`
                    for tref in collect_type_refs(&trimmed[name.len()..], true) {
                        if tref != name {
                            let tgt = ctx.ensure_named_node(&tref, line_no);
                            if tgt != nid {
                                ctx.add_edge(nid.clone(), tgt, "references", line_no, "EXTRACTED", 1.0, Some("field"));
                            }
                        }
                    }
                }
            } else if let Some(caps) = rust_enum_re().captures(&trimmed) {
                let name = caps[1].to_string();
                let line_no = i + 1;
                let nid = make_id(&[stem, &name]);
                ctx.add_node(nid.clone(), name.clone(), line_no, "code");
                ctx.add_edge(file_nid.clone(), nid.clone(), "contains", line_no, "EXTRACTED", 1.0, None);
                if let Some(end) = find_block_end(&lines, i) {
                    for ln in (i + 1)..end {
                        let vline = clean_line(lines[ln]).trim().to_string();
                        if let Some(vcaps) = rust_tuple_variant_re().captures(&vline) {
                            for tref in collect_type_refs(&vcaps[1], true) {
                                let tgt = ctx.ensure_named_node(&tref, ln + 1);
                                if tgt != nid {
                                    ctx.add_edge(nid.clone(), tgt, "references", ln + 1, "EXTRACTED", 1.0, Some("field"));
                                }
                            }
                        } else if let Some(fcaps) = rust_field_re().captures(&vline) {
                            for tref in collect_type_refs(&fcaps[1], true) {
                                let tgt = ctx.ensure_named_node(&tref, ln + 1);
                                if tgt != nid {
                                    ctx.add_edge(nid.clone(), tgt, "references", ln + 1, "EXTRACTED", 1.0, Some("field"));
                                }
                            }
                        }
                    }
                    i = end;
                }
            } else if let Some(caps) = rust_trait_re().captures(&trimmed) {
                let name = caps[1].to_string();
                let line_no = i + 1;
                let nid = make_id(&[stem, &name]);
                ctx.add_node(nid.clone(), name.clone(), line_no, "code");
                ctx.add_edge(file_nid.clone(), nid.clone(), "contains", line_no, "EXTRACTED", 1.0, None);
                // Supertrait bounds: bound đầu → inherits, còn lại → references
                if let Some(bounds) = caps.get(2) {
                    for (idx, bound) in bounds.as_str().split('+').enumerate() {
                        for tref in collect_type_refs(bound, true) {
                            let tgt = ctx.ensure_named_node(&tref, line_no);
                            if tgt == nid {
                                continue;
                            }
                            if idx == 0 {
                                ctx.add_edge(nid.clone(), tgt, "inherits", line_no, "EXTRACTED", 1.0, None);
                            } else {
                                ctx.add_edge(nid.clone(), tgt, "references", line_no, "EXTRACTED", 1.0, Some("generic_arg"));
                            }
                        }
                    }
                }
                // Push trait block vào impl_stack để fn bên trong được detect là method
                if line.matches('{').count() > 0 {
                    if let Some(end) = find_block_end(&lines, i) {
                        impl_stack.push((nid, end));
                    }
                }
            } else if let Some(caps) = rust_fn_re().captures(&trimmed) {
                let name = caps[1].to_string();
                let line_no = i + 1;
                let (nid, label) = if let Some((impl_nid, _)) = impl_stack.last() {
                    (make_id(&[impl_nid, &name]), format!(".{}()", name))
                } else {
                    (make_id(&[stem, &name]), format!("{}()", name))
                };
                ctx.add_node(nid.clone(), label, line_no, "code");
                if let Some((impl_nid, _)) = impl_stack.last() {
                    ctx.add_edge(impl_nid.clone(), nid.clone(), "method", line_no, "EXTRACTED", 1.0, None);
                } else {
                    ctx.add_edge(file_nid.clone(), nid.clone(), "contains", line_no, "EXTRACTED", 1.0, None);
                }
                // Signature type refs (params + return)
                let sig = gather_signature(&lines, i);
                emit_signature_refs(&mut ctx, &nid, &sig, &name, line_no);
                if let Some(end) = find_block_end(&lines, i) {
                    bodies.push((nid, i, end));
                    i = end; // nhảy tới dòng closing }
                }
            }
        }

        i += 1;
    }

    ctx.run_call_pass(&bodies, &lines, rust_keywords());
    ctx.finish()
}

/// Gom signature của fn (từ dòng start tới khi gặp `{` hoặc `;`, tối đa 8 dòng).
fn gather_signature(lines: &[&str], start: usize) -> String {
    let mut sig = String::new();
    for raw in lines.iter().skip(start).take(8) {
        let l = clean_line(raw);
        let stop = l.contains('{') || l.contains(';');
        sig.push_str(&l);
        sig.push(' ');
        if stop {
            break;
        }
    }
    sig
}

/// Emit `references` edges cho type idents trong signature (params + return type).
fn emit_signature_refs(ctx: &mut Ctx, func_nid: &str, sig: &str, fn_name: &str, line: usize) {
    let (params_part, return_part) = match sig.find("->") {
        Some(pos) => {
            let after = &sig[pos + 2..];
            let ret = after.split(['{', ';', 'w']).next().unwrap_or(after);
            (&sig[..pos], ret)
        }
        None => (sig, ""),
    };
    for tref in collect_type_refs(params_part, true) {
        if tref == fn_name {
            continue;
        }
        let tgt = ctx.ensure_named_node(&tref, line);
        if tgt != func_nid {
            ctx.add_edge(func_nid.to_string(), tgt, "references", line, "EXTRACTED", 1.0, Some("parameter_type"));
        }
    }
    for tref in collect_type_refs(return_part, true) {
        let tgt = ctx.ensure_named_node(&tref, line);
        if tgt != func_nid {
            ctx.add_edge(func_nid.to_string(), tgt, "references", line, "EXTRACTED", 1.0, Some("return_type"));
        }
    }
}

// ---------------------------------------------------------------------------
// JS/TS extractor (heuristic)
// ---------------------------------------------------------------------------

fn js_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*import\s+(?:[^'";\n]*\s+from\s+)?['"]([^'"]+)['"]"#).unwrap()
    })
}

fn js_require_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"require\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap())
}

fn js_function_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^\s*(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s*\*?\s*([A-Za-z_$][\w$]*)").unwrap()
    })
}

fn js_arrow_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?::[^=\n]+)?=\s*(?:async\s+)?(?:\([^)]*\)|[A-Za-z_$][\w$]*)\s*=>").unwrap()
    })
}

fn js_const_fn_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?::[^=\n]+)?=\s*(?:async\s+)?function\b").unwrap()
    })
}

fn js_class_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^\s*(?:export\s+)?(?:default\s+)?(?:abstract\s+)?class\s+([A-Za-z_$][\w$]*)(?:\s+extends\s+([A-Za-z_$][\w$.]*))?").unwrap()
    })
}

fn js_interface_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^\s*(?:export\s+)?interface\s+([A-Za-z_$][\w$]*)(?:\s+extends\s+([^\{]+))?").unwrap()
    })
}

fn js_type_alias_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(?:export\s+)?type\s+([A-Za-z_$][\w$]*)\s*[=<]").unwrap())
}

fn js_method_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^\s*(?:public\s+|private\s+|protected\s+|static\s+|async\s+|override\s+|readonly\s+|get\s+|set\s+)*([A-Za-z_$][\w$]*)\s*\([^)]*\)\s*(?::\s*[^\{;]+)?\{?\s*$").unwrap()
    })
}

/// Module name từ import specifier: './utils/helper.js' → 'helper', 'react' → 'react',
/// '@scope/pkg/sub' → 'sub' (graphify lấy last segment).
fn js_module_name(spec: &str) -> String {
    let last = spec.rsplit('/').next().unwrap_or(spec);
    let no_ext = last.split('.').next().unwrap_or(last);
    no_ext.to_string()
}

pub fn extract_js(source_file: &str, stem: &str, content: &str) -> Extraction {
    let mut ctx = Ctx::new(source_file, stem);
    let lines: Vec<&str> = content.lines().collect();
    let file_nid = make_id(&[source_file]);
    ctx.add_node(file_nid.clone(), file_name_of(source_file), 1, "code");

    let mut bodies: Vec<(String, usize, usize)> = Vec::new();
    // class_stack: (class_nid, end_line) — end_line là dòng `}` đóng class block
    let mut class_stack: Vec<(String, usize)> = Vec::new();

    let mut i = 0usize;
    while i < lines.len() {
        let line = clean_line(lines[i]);
        let trimmed = line.trim().to_string();

        // Pop class contexts đã đóng
        while let Some(&(_, end_line)) = class_stack.last() {
            if i > end_line {
                class_stack.pop();
            } else {
                break;
            }
        }

        // in_class = đang là direct child của class top
        let in_class = class_stack.last().is_some();

        if !trimmed.is_empty() {
            let import_spec = js_import_re()
                .captures(&trimmed)
                .map(|c| c[1].to_string())
                .or_else(|| js_require_re().captures(&trimmed).map(|c| c[1].to_string()));

            if let Some(spec) = import_spec {
                let module = js_module_name(&spec);
                if !module.is_empty() {
                    let tgt = ctx.ensure_named_node(&module, i + 1);
                    ctx.add_edge(file_nid.clone(), tgt, "imports_from", i + 1, "EXTRACTED", 1.0, Some("import"));
                }
            } else if let Some(caps) = js_class_re().captures(&trimmed) {
                let name = caps[1].to_string();
                let line_no = i + 1;
                let nid = make_id(&[stem, &name]);
                ctx.add_node(nid.clone(), name.clone(), line_no, "code");
                ctx.add_edge(file_nid.clone(), nid.clone(), "contains", line_no, "EXTRACTED", 1.0, None);
                if let Some(base) = caps.get(2) {
                    let bname = last_segment(base.as_str()).to_string();
                    let tgt = ctx.ensure_named_node(&bname, line_no);
                    if tgt != nid {
                        ctx.add_edge(nid.clone(), tgt, "inherits", line_no, "EXTRACTED", 1.0, None);
                    }
                }
                // Push class với end_line chính xác
                if line.contains('{') {
                    if let Some(end) = find_block_end(&lines, i) {
                        class_stack.push((nid, end));
                    }
                }
            } else if let Some(caps) = js_interface_re().captures(&trimmed) {
                let name = caps[1].to_string();
                let line_no = i + 1;
                let nid = make_id(&[stem, &name]);
                ctx.add_node(nid.clone(), name.clone(), line_no, "code");
                ctx.add_edge(file_nid.clone(), nid.clone(), "contains", line_no, "EXTRACTED", 1.0, None);
                if let Some(bases) = caps.get(2) {
                    for (idx, base) in bases.as_str().split(',').enumerate() {
                        let bname = last_segment(base.trim()).to_string();
                        if bname.is_empty() {
                            continue;
                        }
                        let tgt = ctx.ensure_named_node(&bname, line_no);
                        if tgt == nid {
                            continue;
                        }
                        if idx == 0 {
                            ctx.add_edge(nid.clone(), tgt, "inherits", line_no, "EXTRACTED", 1.0, None);
                        } else {
                            ctx.add_edge(nid.clone(), tgt, "references", line_no, "EXTRACTED", 1.0, Some("generic_arg"));
                        }
                    }
                }
            } else if let Some(caps) = js_type_alias_re().captures(&trimmed) {
                let name = caps[1].to_string();
                let line_no = i + 1;
                let nid = make_id(&[stem, &name]);
                ctx.add_node(nid.clone(), name.clone(), line_no, "code");
                ctx.add_edge(file_nid.clone(), nid.clone(), "contains", line_no, "EXTRACTED", 1.0, None);
            } else if let Some(caps) = js_function_re().captures(&trimmed) {
                let name = caps[1].to_string();
                let line_no = i + 1;
                // Chỉ extract top-level hoặc direct class member
                if false {
                    // placeholder — không skip gì cả, detect tất cả fn ở scope này
                } else {
                    let (nid, label) = if in_class {
                        let class_nid = &class_stack.last().unwrap().0;
                        (make_id(&[class_nid, &name]), format!(".{}()", name))
                    } else {
                        (make_id(&[stem, &name]), format!("{}()", name))
                    };
                    ctx.add_node(nid.clone(), label, line_no, "code");
                    if in_class {
                        let class_nid = class_stack.last().unwrap().0.clone();
                        ctx.add_edge(class_nid, nid.clone(), "method", line_no, "EXTRACTED", 1.0, None);
                    } else {
                        ctx.add_edge(file_nid.clone(), nid.clone(), "contains", line_no, "EXTRACTED", 1.0, None);
                    }
                    let sig = gather_signature(&lines, i);
                    emit_signature_refs(&mut ctx, &nid, &sig, &name, line_no);
                    if let Some(end) = find_block_end(&lines, i) {
                        bodies.push((nid, i, end));
                        i = end;
                    }
                }
            } else if let Some(caps) = js_arrow_re().captures(&trimmed).or_else(|| js_const_fn_re().captures(&trimmed)) {
                let name = caps[1].to_string();
                let line_no = i + 1;
                if !in_class {
                    let nid = make_id(&[stem, &name]);
                    ctx.add_node(nid.clone(), format!("{}()", name), line_no, "code");
                    ctx.add_edge(file_nid.clone(), nid.clone(), "contains", line_no, "EXTRACTED", 1.0, None);
                    let sig = gather_signature(&lines, i);
                    emit_signature_refs(&mut ctx, &nid, &sig, &name, line_no);
                    if let Some(end) = find_block_end(&lines, i) {
                        bodies.push((nid, i, end));
                        i = end;
                    }
                }
            } else if let Some(caps) = js_method_re().captures(&trimmed) {
                // Shorthand method trong class: `render() {` — chỉ khi direct child của class
                let name = caps[1].to_string();
                if in_class && !js_keywords().contains(name.as_str()) {
                    let line_no = i + 1;
                    let class_nid = class_stack.last().unwrap().0.clone();
                    let nid = make_id(&[&class_nid, &name]);
                    ctx.add_node(nid.clone(), format!(".{}()", name), line_no, "code");
                    ctx.add_edge(class_nid, nid.clone(), "method", line_no, "EXTRACTED", 1.0, None);
                    let sig = gather_signature(&lines, i);
                    emit_signature_refs(&mut ctx, &nid, &sig, &name, line_no);
                    if let Some(end) = find_block_end(&lines, i) {
                        bodies.push((nid, i, end));
                        i = end;
                    }
                }
            }
        }

        i += 1;
    }

    ctx.run_call_pass(&bodies, &lines, js_keywords());
    ctx.finish()
}

// ---------------------------------------------------------------------------
// Python extractor (indentation-based)
// ---------------------------------------------------------------------------

fn py_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*import\s+([\w.]+)").unwrap())
}

fn py_from_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*from\s+([\w.]+)\s+import\b").unwrap())
}

fn py_def_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\s*)(?:async\s+)?def\s+([A-Za-z_]\w*)\s*\(").unwrap())
}

fn py_class_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\s*)class\s+([A-Za-z_]\w*)\s*(?:\(([^)]*)\))?\s*:").unwrap())
}

pub fn extract_python(source_file: &str, stem: &str, content: &str) -> Extraction {
    let mut ctx = Ctx::new(source_file, stem);
    let lines: Vec<&str> = content.lines().collect();
    let file_nid = make_id(&[source_file]);
    ctx.add_node(file_nid.clone(), file_name_of(source_file), 1, "code");

    let mut bodies: Vec<(String, usize, usize)> = Vec::new();
    // Stack context: (indent, kind) — kind "class" kèm nid
    let mut class_stack: Vec<(usize, String)> = Vec::new();

    let mut i = 0usize;
    while i < lines.len() {
        let raw = lines[i];
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }

        if let Some(caps) = py_import_re().captures(raw) {
            let module = caps[1].rsplit('.').next().unwrap_or("").to_string();
            if !module.is_empty() {
                let tgt = ctx.ensure_named_node(&module, i + 1);
                ctx.add_edge(file_nid.clone(), tgt, "imports_from", i + 1, "EXTRACTED", 1.0, Some("import"));
            }
            i += 1;
            continue;
        }
        if let Some(caps) = py_from_re().captures(raw) {
            let module = caps[1].rsplit('.').next().unwrap_or("").to_string();
            if !module.is_empty() {
                let tgt = ctx.ensure_named_node(&module, i + 1);
                ctx.add_edge(file_nid.clone(), tgt, "imports_from", i + 1, "EXTRACTED", 1.0, Some("import"));
            }
            i += 1;
            continue;
        }

        if let Some(caps) = py_class_re().captures(raw) {
            let indent = caps[1].len();
            let name = caps[2].to_string();
            let line_no = i + 1;
            while let Some(&(ci, _)) = class_stack.last() {
                if indent <= ci {
                    class_stack.pop();
                } else {
                    break;
                }
            }
            let nid = make_id(&[stem, &name]);
            ctx.add_node(nid.clone(), name.clone(), line_no, "code");
            ctx.add_edge(file_nid.clone(), nid.clone(), "contains", line_no, "EXTRACTED", 1.0, None);
            // Base classes → inherits
            if let Some(bases) = caps.get(3) {
                for (idx, base) in bases.as_str().split(',').enumerate() {
                    let bname = base.trim().rsplit('.').next().unwrap_or("").trim().to_string();
                    if bname.is_empty() || python_keywords().contains(bname.as_str()) {
                        continue;
                    }
                    let tgt = ctx.ensure_named_node(&bname, line_no);
                    if tgt == nid {
                        continue;
                    }
                    if idx == 0 {
                        ctx.add_edge(nid.clone(), tgt, "inherits", line_no, "EXTRACTED", 1.0, None);
                    } else {
                        ctx.add_edge(nid.clone(), tgt, "references", line_no, "EXTRACTED", 1.0, Some("generic_arg"));
                    }
                }
            }
            class_stack.push((indent, nid));
            i += 1;
            continue;
        }

        if let Some(caps) = py_def_re().captures(raw) {
            let indent = caps[1].len();
            let name = caps[2].to_string();
            let line_no = i + 1;
            while let Some(&(ci, _)) = class_stack.last() {
                if indent <= ci {
                    class_stack.pop();
                } else {
                    break;
                }
            }
            let in_class = class_stack.last().map(|&(ci, _)| indent > ci).unwrap_or(false);
            let nested_in_fn = indent > 0 && !in_class;
            if !nested_in_fn {
                let (nid, label) = if in_class {
                    let class_nid = &class_stack.last().unwrap().1;
                    (make_id(&[class_nid, &name]), format!(".{}()", name))
                } else {
                    (make_id(&[stem, &name]), format!("{}()", name))
                };
                ctx.add_node(nid.clone(), label, line_no, "code");
                if in_class {
                    let class_nid = class_stack.last().unwrap().1.clone();
                    ctx.add_edge(class_nid, nid.clone(), "method", line_no, "EXTRACTED", 1.0, None);
                } else {
                    ctx.add_edge(file_nid.clone(), nid.clone(), "contains", line_no, "EXTRACTED", 1.0, None);
                }
                // Body range: các dòng indent > def indent
                let mut end = i;
                for (j, raw2) in lines.iter().enumerate().skip(i + 1) {
                    let t2 = raw2.trim();
                    if t2.is_empty() || t2.starts_with('#') {
                        end = j;
                        continue;
                    }
                    let ind2 = raw2.len() - raw2.trim_start().len();
                    if ind2 > indent {
                        end = j;
                    } else {
                        break;
                    }
                }
                bodies.push((nid, i, end));
                i = end + 1;
                continue;
            }
            // Nested def trong fn khác: skip (calls tính cho outer body)
        }

        i += 1;
    }

    // Python không có line comments strip kiểu // nên dùng raw lines cho call pass
    ctx.run_call_pass_py(&bodies, &lines);
    ctx.finish()
}

impl Ctx {
    /// Call pass cho Python: strip `#` comment, không strip strings
    /// (f-strings phức tạp — heuristic chấp nhận noise nhỏ).
    fn run_call_pass_py(&mut self, bodies: &[(String, usize, usize)], lines: &[&str]) {
        let label_index = self.label_index();
        let mut seen_extracted: HashSet<(String, String)> = HashSet::new();
        let mut seen_raw: HashSet<(String, String)> = HashSet::new();

        for (caller, start, end) in bodies {
            for ln in *start..=(*end).min(lines.len().saturating_sub(1)) {
                let raw = lines[ln];
                let line = match raw.find('#') {
                    Some(pos) => &raw[..pos],
                    None => raw,
                };
                for (callee, kind) in scan_calls(line) {
                    if python_keywords().contains(callee.as_str())
                        || language_builtins().contains(callee.as_str())
                    {
                        continue;
                    }
                    let bare = callee.to_lowercase();
                    if let Some(tgt) = label_index.get(&bare) {
                        if tgt != caller && seen_extracted.insert((caller.clone(), tgt.clone())) {
                            self.edges.push(ExtractedEdge {
                                source: caller.clone(),
                                target: tgt.clone(),
                                relation: "calls".to_string(),
                                confidence: "EXTRACTED".to_string(),
                                source_file: self.source_file.clone(),
                                source_location: format!("L{}", ln + 1),
                                weight: 1.0,
                                context: Some("call".to_string()),
                            });
                        }
                    } else if kind != CallKind::Scoped && !method_blocklist().contains(bare.as_str()) {
                        if seen_raw.insert((caller.clone(), bare.clone())) {
                            self.raw_calls.push(RawCall {
                                caller_nid: caller.clone(),
                                callee: bare,
                                is_member_call: kind == CallKind::Member,
                                source_file: self.source_file.clone(),
                                source_location: format!("L{}", ln + 1),
                            });
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Markdown extractor (docs graph: md links + wikilinks → references)
// ---------------------------------------------------------------------------

fn md_link_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[[^\]]*\]\(([^)\s#]+)(?:#[^)]*)?\)").unwrap())
}

fn md_wikilink_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[\[([^\]|#]+)(?:\|[^\]]*)?\]\]").unwrap())
}

/// Resolve relative md link về stem posix (so với scan root).
fn resolve_md_target(source_file: &str, target: &str) -> Option<String> {
    if target.starts_with("http://") || target.starts_with("https://") || target.starts_with("mailto:") {
        return None;
    }
    let target = target.split('#').next().unwrap_or(target);
    if target.is_empty() {
        return None;
    }
    let mut parts: Vec<String> = source_file
        .rsplit_once('/')
        .map(|(dir, _)| dir.split('/').map(|s| s.to_string()).collect())
        .unwrap_or_default();
    for comp in target.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c.to_string()),
        }
    }
    let joined = parts.join("/");
    let no_ext = joined.rsplit_once('.').map(|(s, _)| s).unwrap_or(&joined);
    Some(no_ext.to_string())
}

pub fn extract_markdown(source_file: &str, _stem: &str, content: &str) -> Extraction {
    let mut ctx = Ctx::new(source_file, "");
    let file_nid = make_id(&[source_file]);
    ctx.add_node(file_nid.clone(), file_name_of(source_file), 1, "doc");

    let mut targets: HashSet<String> = HashSet::new();
    for (ln, raw) in content.lines().enumerate() {
        for caps in md_link_re().captures_iter(raw) {
            if let Some(stem) = resolve_md_target(source_file, &caps[1]) {
                targets.insert(stem);
            }
        }
        for caps in md_wikilink_re().captures_iter(raw) {
            let name = caps[1].trim().to_string();
            if !name.is_empty() {
                targets.insert(name);
            }
        }
        let _ = ln;
    }
    for t in targets {
        let tgt = ctx.ensure_named_node(&t, 1);
        if tgt != file_nid {
            ctx.add_edge(file_nid.clone(), tgt, "references", 1, "EXTRACTED", 1.0, Some("doc_link"));
        }
    }
    ctx.finish()
}

// ---------------------------------------------------------------------------
// detect.py port: collect_files + dispatch
// ---------------------------------------------------------------------------

pub const CODE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "mjs", "mts", "cts", "py", "md", "mdx",
];

const SKIP_DIRS: &[&str] = &[
    "target", "node_modules", ".git", "dist", "build", ".next", "__pycache__",
    ".venv", "venv", "graphify-out", ".culi", ".idea", ".vscode", "coverage",
];

const MAX_FILE_SIZE: u64 = 1_000_000; // 1MB guard

pub fn collect_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                return !SKIP_DIRS.contains(&name.as_ref()) && !name.starts_with('.');
            }
            true
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let ext = e
                .path()
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_lowercase();
            CODE_EXTENSIONS.contains(&ext.as_str())
        })
        .filter(|e| e.metadata().map(|m| m.len() <= MAX_FILE_SIZE).unwrap_or(false))
        .map(|e| e.into_path())
        .collect()
}

/// Stem của file so với root: relative path bỏ extension, posix separators.
/// Port `_file_stem` — toàn bộ segments được giữ để tránh collision same-name.
pub fn file_stem(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let s = rel.with_extension("").to_string_lossy().replace('\\', "/");
    if s == "." {
        String::new()
    } else {
        s
    }
}

fn rel_source_file(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn extract_file(root: &Path, path: &Path) -> Result<Extraction> {
    let ext = path
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_lowercase();
    let content = fs::read_to_string(path)?;
    let source_file = rel_source_file(root, path);
    let stem = file_stem(root, path);

    Ok(match ext.as_str() {
        "rs" => extract_rust(&source_file, &stem, &content),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "mts" | "cts" => {
            extract_js(&source_file, &stem, &content)
        }
        "py" => extract_python(&source_file, &stem, &content),
        "md" | "mdx" => extract_markdown(&source_file, &stem, &content),
        _ => Extraction::default(),
    })
}

// ---------------------------------------------------------------------------
// Corpus passes: stub rewire + raw_calls → INFERRED
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Copy)]
pub struct ResolveStats {
    pub rewired_stubs: usize,
    pub inferred: usize,
    pub ambiguous: usize,
    pub unresolved: usize,
}

/// Corpus label index: bare label -> real node ids + filename stems -> file node ids.
fn build_corpus_index(extractions: &[Extraction]) -> HashMap<String, HashSet<String>> {
    let mut index: HashMap<String, HashSet<String>> = HashMap::new();
    for ex in extractions {
        for n in &ex.nodes {
            if n.source_file.is_empty() {
                continue; // stubs không vào index
            }
            index
                .entry(bare_label(&n.label))
                .or_default()
                .insert(n.id.clone());
            // File nodes: index thêm theo filename-không-extension để import
            // `use x::helpers` / `import './helpers'` rewire về file node.
            // Dùng id-check thay vì source_location để tránh nhầm fn ở dòng 1.
            if n.id == make_id(&[n.source_file.as_str()]) {
                let fname = file_name_of(&n.source_file);
                let stem = fname.split('.').next().unwrap_or(&fname).to_lowercase();
                if !stem.is_empty() {
                    index.entry(stem).or_default().insert(n.id.clone());
                }
            }
        }
    }
    index
}

/// Collapse stub nodes về real definition khi match duy nhất toàn corpus.
/// Port ý tưởng corpus-level rewire của graphify (#1402).
pub fn rewire_stubs(extractions: &mut [Extraction], index: &HashMap<String, HashSet<String>>) -> usize {
    // Pass 1: build stub -> real mapping
    let mut mapping: HashMap<String, String> = HashMap::new();
    for ex in extractions.iter() {
        for n in &ex.nodes {
            if !n.source_file.is_empty() {
                continue;
            }
            let key = bare_label(&n.label);
            if let Some(candidates) = index.get(&key) {
                if candidates.len() == 1 && !candidates.contains(&n.id) {
                    let real = candidates.iter().next().unwrap().clone();
                    mapping.insert(n.id.clone(), real);
                }
            }
        }
    }
    if mapping.is_empty() {
        return 0;
    }
    let rewired = mapping.len();
    // Pass 2: apply mapping vào edges + raw_call callers, drop stub nodes
    for ex in extractions.iter_mut() {
        ex.nodes.retain(|n| !mapping.contains_key(&n.id));
        for e in ex.edges.iter_mut() {
            if let Some(real) = mapping.get(&e.source) {
                e.source = real.clone();
            }
            if let Some(real) = mapping.get(&e.target) {
                e.target = real.clone();
            }
        }
        for rc in ex.raw_calls.iter_mut() {
            if let Some(real) = mapping.get(&rc.caller_nid) {
                rc.caller_nid = real.clone();
            }
        }
    }
    rewired
}

/// Cross-file call resolution: raw_call khớp DUY NHẤT 1 definition → INFERRED calls.
pub fn resolve_raw_calls(extractions: &mut [Extraction], index: &HashMap<String, HashSet<String>>) -> ResolveStats {
    let mut stats = ResolveStats::default();
    for ex in extractions.iter_mut() {
        let mut new_edges: Vec<ExtractedEdge> = Vec::new();
        let mut seen: HashSet<(String, String)> = HashSet::new();
        for rc in &ex.raw_calls {
            match index.get(&rc.callee) {
                Some(candidates) if candidates.len() == 1 => {
                    let tgt = candidates.iter().next().unwrap();
                    if tgt == &rc.caller_nid {
                        continue;
                    }
                    if seen.insert((rc.caller_nid.clone(), tgt.clone())) {
                        new_edges.push(ExtractedEdge {
                            source: rc.caller_nid.clone(),
                            target: tgt.clone(),
                            relation: "calls".to_string(),
                            confidence: "INFERRED".to_string(),
                            source_file: rc.source_file.clone(),
                            source_location: rc.source_location.clone(),
                            weight: 0.8,
                            context: Some("cross_file".to_string()),
                        });
                        stats.inferred += 1;
                    }
                }
                Some(_) => stats.ambiguous += 1,
                None => stats.unresolved += 1,
            }
        }
        ex.edges.extend(new_edges);
    }
    stats
}

// ---------------------------------------------------------------------------
// scan_directory: collect → extract → corpus passes → KnowledgeGraph
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ScanStats {
    pub files_scanned: usize,
    pub files_failed: usize,
    pub nodes: usize,
    pub edges: usize,
    pub extracted_edges: usize,
    pub inferred_edges: usize,
    pub rewired_stubs: usize,
    pub ambiguous_calls: usize,
    pub unresolved_calls: usize,
}

fn confidence_of(s: &str) -> EdgeConfidence {
    match s {
        "EXTRACTED" => EdgeConfidence::Extracted,
        "INFERRED" => EdgeConfidence::Inferred,
        "AMBIGUOUS" => EdgeConfidence::Ambiguous,
        _ => EdgeConfidence::Extracted,
    }
}

pub fn scan_directory(root: &Path) -> Result<(KnowledgeGraph, ScanStats)> {
    let files = collect_files(root);
    let mut stats = ScanStats::default();
    let mut extractions: Vec<Extraction> = Vec::new();

    for f in &files {
        match extract_file(root, f) {
            Ok(ex) => {
                stats.files_scanned += 1;
                extractions.push(ex);
            }
            Err(_) => stats.files_failed += 1,
        }
    }

    // Corpus passes
    let index = build_corpus_index(&extractions);
    stats.rewired_stubs = rewire_stubs(&mut extractions, &index);
    let resolve_stats = resolve_raw_calls(&mut extractions, &index);
    stats.inferred_edges = resolve_stats.inferred;
    stats.ambiguous_calls = resolve_stats.ambiguous;
    stats.unresolved_calls = resolve_stats.unresolved;

    // Bridge → KnowledgeGraph. Real nodes first, stub chỉ thêm nếu chưa tồn tại.
    let mut graph = KnowledgeGraph::new();
    let mut real_ids: HashSet<String> = HashSet::new();
    for ex in &extractions {
        for n in &ex.nodes {
            if n.source_file.is_empty() {
                continue;
            }
            real_ids.insert(n.id.clone());
            let mut properties = HashMap::new();
            properties.insert("source_file".to_string(), n.source_file.clone());
            properties.insert("source_location".to_string(), n.source_location.clone());
            let node_type = if n.source_location == "L1" {
                NodeType::File
            } else {
                NodeType::Code
            };
            graph.add_node(GraphNode {
                id: n.id.clone(),
                label: n.label.clone(),
                node_type,
                properties,
                confidence: 1.0,
                source: "graphify-fork".to_string(),
            })?;
        }
    }
    for ex in &extractions {
        for n in &ex.nodes {
            if !n.source_file.is_empty() || real_ids.contains(&n.id) {
                continue;
            }
            real_ids.insert(n.id.clone());
            let mut properties = HashMap::new();
            if let Some(origin) = &n.origin_file {
                properties.insert("origin_file".to_string(), origin.clone());
            }
            graph.add_node(GraphNode {
                id: n.id.clone(),
                label: n.label.clone(),
                node_type: NodeType::Code,
                properties,
                confidence: 1.0,
                source: "graphify-fork-stub".to_string(),
            })?;
        }
    }

    // Edges: EXTRACTED trước để thắng dedup khi trùng (src, tgt, relation).
    let mut seen_edges: HashSet<(String, String, String)> = HashSet::new();
    let mut all_edges: Vec<&ExtractedEdge> = Vec::new();
    for ex in &extractions {
        all_edges.extend(ex.edges.iter().filter(|e| e.confidence == "EXTRACTED"));
    }
    for ex in &extractions {
        all_edges.extend(ex.edges.iter().filter(|e| e.confidence != "EXTRACTED"));
    }
    for e in all_edges {
        if !real_ids.contains(&e.source) || !real_ids.contains(&e.target) {
            continue;
        }
        let key = (e.source.clone(), e.target.clone(), e.relation.clone());
        if !seen_edges.insert(key) {
            continue;
        }
        if e.confidence == "EXTRACTED" {
            stats.extracted_edges += 1;
        }
        graph.add_edge(GraphEdge {
            source_id: e.source.clone(),
            target_id: e.target.clone(),
            relationship: e.relation.clone(),
            weight: e.weight,
            confidence: confidence_of(&e.confidence),
        })?;
    }

    stats.nodes = graph.node_count();
    stats.edges = graph.edge_count();
    Ok((graph, stats))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const RUST_SAMPLE: &str = r#"
use crate::config::Config;
use std::collections::HashMap;

pub struct Cart {
    items: Vec<Product>,
    logger: Logger,
}

pub trait Priced {
    fn price(&self) -> f64;
}

impl Priced for Cart {
    fn price(&self) -> f64 {
        self.total()
    }
}

impl Cart {
    pub fn new(logger: Logger) -> Self {
        Cart { items: vec![], logger }
    }

    pub fn total(&self) -> f64 {
        calculate_total(&self.items)
    }
}

fn calculate_total(items: &[Product]) -> f64 {
    items.iter().map(|p| p.price).sum()
}

pub fn render(cart: &Cart, cfg: Config) -> String {
    format!("{}", cart.total())
}
"#;

    #[test]
    fn test_extract_rust_structure() {
        let ex = extract_rust("src/cart.rs", "src/cart", RUST_SAMPLE);
        let labels: HashSet<&str> = ex.nodes.iter().map(|n| n.label.as_str()).collect();
        // File node + struct + trait + 2 impl type nodes (dedup by id) + fns + stubs
        assert!(labels.contains("cart.rs"));
        assert!(labels.contains("Cart"));
        assert!(labels.contains("Priced"));
        assert!(labels.contains("calculate_total()"));
        assert!(labels.contains("render()"));
        assert!(labels.contains(".total()"));
        assert!(labels.contains(".price()"));
        assert!(labels.contains(".new()"));
        // Stub nodes từ type refs/imports
        assert!(labels.contains("Config"));
        assert!(labels.contains("Product"));
        assert!(labels.contains("Logger"));
    }

    #[test]
    fn test_extract_rust_edges() {
        let ex = extract_rust("src/cart.rs", "src/cart", RUST_SAMPLE);
        let has = |src_label: &str, relation: &str, tgt_label: &str| {
            let find_id = |label: &str| ex.nodes.iter().find(|n| n.label == label).map(|n| n.id.clone());
            match (find_id(src_label), find_id(tgt_label)) {
                (Some(s), Some(t)) => ex.edges.iter().any(|e| e.source == s && e.target == t && e.relation == relation),
                _ => false,
            }
        };
        // implements: impl Priced for Cart → Cart node (impl nid = make_id(stem, "Cart"))
        assert!(has("Cart", "implements", "Priced"));
        // method edges từ impl Cart
        assert!(has("Cart", "method", ".total()"));
        assert!(has("Cart", "method", ".new()"));
        // imports_from tới module stub 'Config' và 'HashMap'
        assert!(has("cart.rs", "imports_from", "Config"));
        // references: Cart field logger: Logger
        assert!(has("Cart", "references", "Logger"));
        // EXTRACTED same-file calls: .total() → calculate_total()
        assert!(has(".total()", "calls", "calculate_total()"));
        // render() gọi cart.total() → member call resolve same-file
        assert!(has("render()", "calls", ".total()"));
    }

    #[test]
    fn test_extract_js() {
        let js = r#"
import React from 'react';
import { helper } from './utils/helper.js';

export class Cart extends BaseComponent {
    constructor(props) {
        super(props);
    }
    render() {
        return formatCart(this.state);
    }
}

export function formatCart(state) {
    return compute(state);
}

const compute = (state) => {
    return state.items.length;
};
"#;
        let ex = extract_js("web/cart.tsx", "web/cart", js);
        let labels: HashSet<&str> = ex.nodes.iter().map(|n| n.label.as_str()).collect();
        assert!(labels.contains("cart.tsx"));
        assert!(labels.contains("Cart"));
        assert!(labels.contains("formatCart()"));
        assert!(labels.contains("compute()"));
        assert!(labels.contains(".render()"));
        assert!(labels.contains("react")); // import stub
        assert!(labels.contains("helper")); // import stub

        let has = |sl: &str, rel: &str, tl: &str| {
            let find = |l: &str| ex.nodes.iter().find(|n| n.label == l).map(|n| n.id.clone());
            match (find(sl), find(tl)) {
                (Some(s), Some(t)) => ex.edges.iter().any(|e| e.source == s && e.target == t && e.relation == rel),
                _ => false,
            }
        };
        assert!(has("Cart", "inherits", "BaseComponent"));
        assert!(has("Cart", "method", ".render()"));
        assert!(has(".render()", "calls", "formatCart()"));
        assert!(has("formatCart()", "calls", "compute()"));
        assert!(has("cart.tsx", "imports_from", "react"));
    }

    #[test]
    fn test_extract_python() {
        let py = r#"
import os
from utils.helpers import clean

class Service(BaseService):
    def run(self):
        return self.fetch()

    def fetch(self):
        return load_data()

def load_data():
    return []
"#;
        let ex = extract_python("app/service.py", "app/service", py);
        let labels: HashSet<&str> = ex.nodes.iter().map(|n| n.label.as_str()).collect();
        assert!(labels.contains("service.py"));
        assert!(labels.contains("Service"));
        assert!(labels.contains(".run()"));
        assert!(labels.contains(".fetch()"));
        assert!(labels.contains("load_data()"));

        let has = |sl: &str, rel: &str, tl: &str| {
            let find = |l: &str| ex.nodes.iter().find(|n| n.label == l).map(|n| n.id.clone());
            match (find(sl), find(tl)) {
                (Some(s), Some(t)) => ex.edges.iter().any(|e| e.source == s && e.target == t && e.relation == rel),
                _ => false,
            }
        };
        assert!(has("Service", "inherits", "BaseService"));
        assert!(has("Service", "method", ".run()"));
        assert!(has(".run()", "calls", ".fetch()"));
        assert!(has(".fetch()", "calls", "load_data()"));
        assert!(has("service.py", "imports_from", "helpers"));
    }

    #[test]
    fn test_extract_markdown_links() {
        let md = "# Task\n\n- [x] Setup DB (xem [chi tiết](./task1_db.md))\n- [ ] API Auth (cần đọc [[Auth Schema]])\n";
        let ex = extract_markdown("docs/todo.md", "docs/todo", md);
        assert!(ex
            .edges
            .iter()
            .any(|e| e.relation == "references" && e.source == make_id(&["docs/todo.md"])));
        let labels: HashSet<&str> = ex.nodes.iter().map(|n| n.label.as_str()).collect();
        assert!(labels.contains("docs/task1_db")); // relative link resolved về stem
        assert!(labels.contains("Auth Schema"));
    }

    fn write_file(dir: &Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        let mut f = fs::File::create(p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_scan_directory_cross_file_inferred() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "src/a.rs", "pub fn shared_helper() -> i32 { 42 }\n");
        write_file(
            tmp.path(),
            "src/b.rs",
            "pub fn caller() -> i32 {\n    shared_helper()\n}\n",
        );
        let (graph, stats) = scan_directory(tmp.path()).unwrap();
        assert_eq!(stats.files_scanned, 2);
        assert!(stats.inferred_edges >= 1, "cross-file call phải thành INFERRED");

        let caller_id = make_id(&["src/b", "caller"]);
        let helper_id = make_id(&["src/a", "shared_helper"]);
        assert!(graph.edges().iter().any(|e| e.source_id == caller_id
            && e.target_id == helper_id
            && e.relationship == "calls"
            && e.confidence == EdgeConfidence::Inferred));
    }

    #[test]
    fn test_scan_directory_stub_rewire_to_file_node() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "src/helpers.rs", "pub fn util() {}\n");
        write_file(
            tmp.path(),
            "src/main.rs",
            "use crate::helpers;\npub fn boot() {}\n",
        );
        let (graph, _stats) = scan_directory(tmp.path()).unwrap();
        let main_file = make_id(&["src/main.rs"]);
        let helpers_file = make_id(&["src/helpers.rs"]);
        // import stub 'helpers' rewire về file node helpers.rs
        assert!(graph.edges().iter().any(|e| e.source_id == main_file
            && e.target_id == helpers_file
            && e.relationship == "imports_from"));
    }

    #[test]
    fn test_collect_files_skips_noise_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "src/ok.rs", "fn a() {}\n");
        write_file(tmp.path(), "target/skip.rs", "fn b() {}\n");
        write_file(tmp.path(), "node_modules/skip.js", "function c() {}\n");
        write_file(tmp.path(), "notes.txt", "not code\n");
        let files = collect_files(tmp.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("ok.rs"));
    }

    #[test]
    fn test_scan_calls_no_double_count() {
        let calls = scan_calls("let x = Type::build(compute(a), b.compute());");
        // scoped: build; member: compute (b.compute); plain: compute
        assert!(calls.iter().any(|(c, k)| c == "build" && *k == CallKind::Scoped));
        assert!(calls.iter().any(|(c, k)| c == "compute" && *k == CallKind::Member));
        assert!(calls.iter().any(|(c, k)| c == "compute" && *k == CallKind::Plain));
        // "build" không được match lại ở plain sau khi scoped blank out
        assert!(!calls.iter().any(|(c, k)| c == "build" && *k == CallKind::Plain));
    }
}
