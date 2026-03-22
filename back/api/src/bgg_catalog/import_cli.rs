//! CLI helpers for the `import_bgg_catalog` binary (testable without running the binary).

use std::path::Path;

/// Parses a **`BGG_IMPORT_MAX_ROWS`**-style value: empty, `0`, or invalid → `None`; positive integer → `Some(n)`.
#[must_use]
pub fn parse_bgg_import_max_rows_from_str(value: Option<&str>) -> Option<usize> {
    let t = value?.trim();
    if t.is_empty() {
        return None;
    }
    let n: usize = t.parse().ok()?;
    if n == 0 {
        None
    } else {
        Some(n)
    }
}

/// Reads **`BGG_IMPORT_MAX_ROWS`** from the process environment (same semantics as [`parse_bgg_import_max_rows_from_str`]).
#[must_use]
pub fn parse_bgg_import_max_rows() -> Option<usize> {
    parse_bgg_import_max_rows_from_str(std::env::var("BGG_IMPORT_MAX_ROWS").ok().as_deref())
}

/// Returns true when the path argument looks like a documentation placeholder (Unicode ellipsis, `<path>`, etc.).
#[must_use]
pub fn looks_like_doc_placeholder(path: &Path) -> bool {
    path.to_str().map_or(false, |s| {
        let t = s.trim();
        t == "…"
            || t == "..."
            || t.eq_ignore_ascii_case("<path>")
            || t.eq_ignore_ascii_case("<csv>")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_max_rows_variants() {
        assert_eq!(parse_bgg_import_max_rows_from_str(None), None);
        assert_eq!(parse_bgg_import_max_rows_from_str(Some("")), None);
        assert_eq!(parse_bgg_import_max_rows_from_str(Some("   ")), None);
        assert_eq!(parse_bgg_import_max_rows_from_str(Some("0")), None);
        assert_eq!(parse_bgg_import_max_rows_from_str(Some("5000")), Some(5000));
        assert_eq!(parse_bgg_import_max_rows_from_str(Some(" 42 ")), Some(42));
        assert_eq!(parse_bgg_import_max_rows_from_str(Some("nope")), None);
    }

    #[test]
    fn looks_like_placeholder() {
        assert!(looks_like_doc_placeholder(Path::new("…")));
        assert!(looks_like_doc_placeholder(Path::new("...")));
        assert!(looks_like_doc_placeholder(Path::new("<path>")));
        assert!(!looks_like_doc_placeholder(Path::new("data/bgg/boardgames_ranks.csv")));
    }
}
