//! The user guide's marked tables (#943 T061, #1276 T003): a Markdown table between
//! `<!-- <marker>:start -->` and `<!-- <marker>:end -->` on a guide page, which a test holds equal
//! to a table the application binds. Shared by `terminal_only_doc.rs` and `review_keys_doc.rs`.

/// The marker comments for `marker`: `(start, end)`.
pub fn markers(marker: &str) -> (String, String) {
    (
        format!("<!-- {marker}:start -->"),
        format!("<!-- {marker}:end -->"),
    )
}

/// The text of `page`, a path from the workspace root (`docs/user-guide/…`), whichever crate
/// includes this file. Callers spell the whole path, so `documentation_is_not_read` sees the page.
pub fn guide_page(page: &str) -> String {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .ancestors()
        .find(|dir| dir.join("crates").join("micold-core").is_dir())
        .unwrap_or(&manifest)
        .to_path_buf();
    let path = root.join(page);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The text of `page` between `marker`'s two comments. Panics naming the marker that is missing,
/// or `end` when it comes only before `start`.
pub fn marked_section(page: &str, marker: &str) -> String {
    let text = guide_page(page);
    let (start, end) = markers(marker);
    let at = text
        .find(&start)
        .unwrap_or_else(|| panic!("`{page}` has `{start}`"));
    let len = text[at..]
        .find(&end)
        .unwrap_or_else(|| panic!("`{page}` has `{end}` after `{start}`"));
    text[at + start.len()..at + len].to_string()
}

/// The table's body rows as `(first cell, second cell)`, header and rule left out.
pub fn rows(section: &str) -> Vec<(String, String)> {
    section
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('|'))
        .skip(2)
        .map(|l| {
            let cells: Vec<&str> = l.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells.len(), 2, "two cells: {l}");
            (cells[0].to_string(), cells[1].to_string())
        })
        .collect()
}
