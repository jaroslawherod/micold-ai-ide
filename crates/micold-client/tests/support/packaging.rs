//! Readers for the packaging manifests the packaging tests share (feature 1260, T020).
//!
//! Included with `#[path = "support/packaging.rs"] mod packaging;` rather than through
//! `support/mod.rs`, so a packaging test does not compile the renderer-backed scaffolding beside it.
//!
//! The asset lists are read as text rather than parsed: the properties under test are about what a
//! *declarative file says*, and a TOML parser would add a dependency to assert something a substring
//! already settles.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The repository root: `tests/` -> `crates/micold-client` -> `crates` -> root.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root above crates/micold-client")
        .to_path_buf()
}

/// The client crate's `Cargo.toml`, which holds the deb and rpm asset lists.
pub fn manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

/// The `[package.metadata.deb] assets` list, verbatim, or `None` when the section is absent.
///
/// The closing bracket is found **at depth**, not by the first `]`. Every entry in this list is
/// itself an array, so a naive `find(']')` returns only the first asset — which is how the first
/// draft of this function passed a manifest that shipped the showcase.
pub fn deb_assets(manifest: &str) -> Option<&str> {
    list_after(manifest, "[package.metadata.deb]", &['['], &[']'])
}

/// The `[package.metadata.generate-rpm] assets` list, verbatim, or `None` when absent. Same reading
/// as `deb_assets`, with braces counted too because the entries are inline tables
/// (`{ source = ..., dest = ... }`).
pub fn rpm_assets(manifest: &str) -> Option<&str> {
    list_after(
        manifest,
        "[package.metadata.generate-rpm]",
        &['[', '{'],
        &[']', '}'],
    )
}

/// The body of the first `assets = [ ... ]` after `section`, closed at depth.
fn list_after<'a>(
    manifest: &'a str,
    section: &str,
    opens: &[char],
    closes: &[char],
) -> Option<&'a str> {
    let after = manifest.split_once(section)?.1;
    let open = after.find("assets = [")?;
    let rest = &after[open + "assets = [".len()..];
    let mut depth = 0usize;
    for (i, c) in rest.char_indices() {
        if opens.contains(&c) {
            depth += 1;
        } else if c == ']' && depth == 0 {
            return Some(&rest[..i]);
        } else if closes.contains(&c) {
            depth -= 1;
        }
    }
    None
}

/// The font licence set every release bundle ships (feature 1260, FR-008, data-model.md): the
/// licence of each bundled face and the provenance record that maps face to licence. Paths from
/// the repository root — never a bare `LICENSE`, which names the repository's own licence, a
/// declared documentation file no test may read (`documentation_is_not_read.rs`).
pub const FONT_LICENCES: &[&str] = &[
    "assets/fonts/LICENSE-FiraMono-OFL.txt",
    "assets/fonts/LICENSE-Roboto-OFL.txt",
    "assets/fonts/LICENSE",
    "assets/fonts/PROVENANCE.md",
];

/// The file name at the end of a path, with either separator (Inno Setup paths use `\`).
pub fn basename(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// The `(Source, DestDir)` pairs of an Inno Setup script's `[Files]` section, quotes stripped and
/// `;` comment lines skipped. A line without a `DestDir:` pairs its source with `""`.
pub fn iss_file_entries(script: &str) -> Vec<(&str, &str)> {
    let mut in_files = false;
    let mut entries = Vec::new();
    for line in script.lines().map(str::trim) {
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            in_files = line.eq_ignore_ascii_case("[Files]");
            continue;
        }
        if !in_files || !line.starts_with("Source:") {
            continue;
        }
        let param = |name: &str| {
            line.split(';')
                .map(str::trim)
                .find_map(|part| part.strip_prefix(name))
                .map_or("", |value| value.trim().trim_matches('"'))
        };
        entries.push((param("Source:"), param("DestDir:")));
    }
    entries
}
