//! Fork của graphify-8 `ids.py` — single source of truth cho node-ID normalization.
//!
//! Recipe: NFKC normalize → thay cụm ký tự non-word bằng 1 underscore → gộp
//! underscore lặp → strip underscore 2 đầu → lowercase. Idempotent.
//!
//! Mọi producer của node ID (extractor, future LLM semantic pass, graph builder)
//! PHẢI đi qua 2 hàm này để tránh ID-drift (một thực thể tách thành nhiều ghost node).

use regex::Regex;
use std::sync::OnceLock;
use unicode_normalization::UnicodeNormalization;

fn non_word_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // \w của crate regex là Unicode-aware mặc định (CJK/accented letters survive)
    RE.get_or_init(|| Regex::new(r"[^\w]+").unwrap())
}

fn underscores_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"_+").unwrap())
}

/// Normalize một ID string về canonical form.
/// Idempotent: `normalize_id(normalize_id(s)) == normalize_id(s)`.
pub fn normalize_id(s: &str) -> String {
    let nfkc: String = s.nfkc().collect();
    let replaced = non_word_re().replace_all(&nfkc, "_");
    let collapsed = underscores_re().replace_all(&replaced, "_");
    collapsed.trim_matches('_').to_lowercase()
}

/// Build canonical node ID từ một hoặc nhiều name parts.
/// Parts được join bằng `_` (sau khi strip `_`/`.` ở 2 đầu mỗi part, bỏ part rỗng),
/// rồi chạy qua `normalize_id`.
pub fn make_id(parts: &[&str]) -> String {
    let joined = parts
        .iter()
        .filter(|p| !p.is_empty())
        .map(|p| p.trim_matches(|c| c == '_' || c == '.'))
        .collect::<Vec<_>>()
        .join("_");
    normalize_id(&joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_id_basic() {
        assert_eq!(normalize_id("src/main.rs"), "src_main_rs");
        assert_eq!(normalize_id("HelloWorld()"), "helloworld");
        assert_eq!(normalize_id("__already_ok__"), "already_ok");
        assert_eq!(normalize_id("a--b  c/d"), "a_b_c_d");
    }

    #[test]
    fn test_normalize_id_unicode() {
        // Unicode letters survive (không collapse thành chuỗi rỗng)
        assert_eq!(normalize_id("Café—Naïve"), "café_naïve");
        assert_eq!(normalize_id("tiếng_việt"), "tiếng_việt");
    }

    #[test]
    fn test_normalize_id_idempotent() {
        let once = normalize_id("Src/Tools::SearchReplace<T>");
        assert_eq!(normalize_id(&once), once);
    }

    #[test]
    fn test_make_id_parts() {
        assert_eq!(make_id(&["src/tools", "SearchReplaceTool"]), "src_tools_searchreplacetool");
        assert_eq!(make_id(&["src/main.rs"]), "src_main_rs");
        // Parts rỗng bị bỏ qua, dấu `.`/`_` ở đầu part bị strip
        assert_eq!(make_id(&["", ".hidden", "name"]), "hidden_name");
        assert_eq!(make_id(&["stem", ".method()"]), "stem_method");
    }
}
