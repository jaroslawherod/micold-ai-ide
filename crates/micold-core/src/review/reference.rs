//! Paths and import strings in a line of code that name a file of the repository (feature 1150,
//! FR-012, research R8).
//!
//! [`find`] is pure: it cuts a line into candidate tokens and keeps only the ones that resolve, by
//! the caller's `exists`, to a listed file, relative to the current file's directory first and the
//! repository root second. Text that names no listed file is never returned, so it is never a
//! link. Symbol lookup is out of scope: only strings that name a file.

use std::ops::Range;

use super::RelPath;

/// Extensions tried after a name that is not itself a file (`./util` is `util.ts`).
const EXTENSIONS: [&str; 8] = ["rs", "ts", "tsx", "js", "jsx", "py", "go", "md"];

/// What a directory import stands for.
const INDEXES: [&str; 6] = [
    "mod.rs",
    "index.ts",
    "index.tsx",
    "index.js",
    "index.jsx",
    "__init__.py",
];

/// Lines longer than this are not searched: a minified line is no place for links.
const MAX_LINE: usize = 4096;

/// One file named in a line of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// The bytes of the path in the line (a line suffix is not part of it).
    pub range: Range<usize>,
    /// The file it names.
    pub path: RelPath,
    /// The line the text gives, when it does (`:12`, `#L12`, `(12)`).
    pub line: Option<u32>,
}

/// The references in `text`, a line of the file `from` (a path relative to the root), in order.
/// `exists` says whether a root-relative path is a listed file.
pub fn find(text: &str, from: &str, exists: &dyn Fn(&str) -> bool) -> Vec<Reference> {
    if text.len() > MAX_LINE {
        return Vec::new();
    }
    let dir = from.rsplit_once('/').map_or("", |(dir, _)| dir);
    let mut found = Vec::new();
    let mut start = None;
    for (at, c) in text.char_indices().chain([(text.len(), ' ')]) {
        match (start, is_delimiter(c)) {
            (None, false) => start = Some(at),
            (Some(begin), true) => {
                start = None;
                found.extend(candidate(text, begin..at, dir, exists));
            }
            _ => {}
        }
    }
    found
}

/// Characters that end a token: whitespace, quotes, brackets and separators of code. `(` ends it
/// too, so `file.rs(12)` gives the path and a line.
fn is_delimiter(c: char) -> bool {
    c.is_whitespace() || "\"'`<>[]{}(),;=|*!?".contains(c)
}

/// The reference a token makes, when it makes one.
fn candidate(
    text: &str,
    token: Range<usize>,
    dir: &str,
    exists: &dyn Fn(&str) -> bool,
) -> Option<Reference> {
    let word = &text[token.clone()];
    let mut line = None;
    let mut path = word;
    if let Some((head, number)) = split_suffix(word) {
        (path, line) = (head, Some(number));
    } else if let Some(number) = paren_line(&text[token.end..]) {
        line = Some(number);
    }
    // Sentence punctuation around a path is not part of it.
    let trimmed = path
        .trim_end_matches(['.', ':', '#'])
        .trim_start_matches(['@', '+']);
    let start = token.start + (path.len() - path.trim_start_matches(['@', '+']).len());
    let range = start..start + trimmed.len();
    if !(trimmed.contains('/') || trimmed.contains('.'))
        || trimmed.chars().all(|c| c == '.' || c == '/')
    {
        return None;
    }
    let path = resolve(trimmed, dir, exists)?;
    Some(Reference { range, path, line })
}

/// `path:12`, `path:12:3` or `path#L12`: the path and the first number.
fn split_suffix(word: &str) -> Option<(&str, u32)> {
    let number = |s: &str| {
        let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<u32>().ok()
    };
    if let Some((path, tail)) = word.rsplit_once("#L") {
        return number(tail).map(|n| (path, n));
    }
    // The first `:` that starts digits; a second `:col` follows the line.
    word.match_indices(':')
        .filter(|(at, _)| *at > 0)
        .find_map(|(at, _)| number(&word[at + 1..]).map(|n| (&word[..at], n)))
}

/// `(12)` at the start of `after`.
fn paren_line(after: &str) -> Option<u32> {
    let inner = after.strip_prefix('(')?;
    let digits: String = inner.chars().take_while(char::is_ascii_digit).collect();
    inner[digits.len()..].starts_with(')').then_some(())?;
    digits.parse().ok()
}

/// The listed file `word` names, relative to `dir` then to the root.
fn resolve(word: &str, dir: &str, exists: &dyn Fn(&str) -> bool) -> Option<RelPath> {
    let bases: &[&str] = if dir.is_empty() { &[""] } else { &[dir, ""] };
    bases
        .iter()
        .find_map(|base| probe(&normalise(base, word)?, exists))
}

/// `base` joined with `word`, without `.` and `..` parts; none when it climbs above the root.
fn normalise(base: &str, word: &str) -> Option<String> {
    let mut parts: Vec<&str> = base.split('/').filter(|p| !p.is_empty()).collect();
    for part in word.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// `path` itself, else with an extension, else as a directory's index.
fn probe(path: &str, exists: &dyn Fn(&str) -> bool) -> Option<RelPath> {
    let mut tries = vec![path.to_owned()];
    tries.extend(EXTENSIONS.iter().map(|ext| format!("{path}.{ext}")));
    tries.extend(INDEXES.iter().map(|index| format!("{path}/{index}")));
    tries
        .into_iter()
        .find(|t| exists(t))
        .and_then(|t| RelPath::from_native(&t))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILES: [&str; 8] = [
        "README.md",
        "src/main.rs",
        "src/ui/mod.rs",
        "src/ui/button.rs",
        "web/app/index.ts",
        "web/app/util.ts",
        "docs/guide.md",
        "lib/x.py",
    ];

    type Found = (String, String, Option<u32>);

    fn refs(text: &str, from: &str) -> Vec<Found> {
        find(text, from, &|p| FILES.contains(&p))
            .into_iter()
            .map(|r| (text[r.range].to_owned(), r.path.as_str().to_owned(), r.line))
            .collect()
    }

    fn one(text: &str, from: &str) -> Found {
        let mut all = refs(text, from);
        assert_eq!(all.len(), 1, "{text:?} gave {all:?}");
        all.remove(0)
    }

    #[test]
    fn only_listed_files_are_returned() {
        assert!(refs("see src/missing.rs and nothing.txt", "src/main.rs").is_empty());
        assert!(refs("a / b ... and/or", "src/main.rs").is_empty());
        assert_eq!(
            one("see docs/guide.md here", "src/main.rs"),
            ("docs/guide.md".into(), "docs/guide.md".into(), None)
        );
    }

    #[test]
    fn a_relative_name_resolves_before_the_root() {
        assert_eq!(
            one("\"./button.rs\"", "src/ui/mod.rs").1,
            "src/ui/button.rs"
        );
        assert_eq!(one("\"../main.rs\"", "src/ui/mod.rs").1, "src/main.rs");
        assert_eq!(one("see src/main.rs", "src/ui/mod.rs").1, "src/main.rs");
        assert_eq!(one("README.md", "src/ui/mod.rs").1, "README.md");
        // Climbing out of the repository names nothing.
        assert!(refs("\"../../../README.md\"", "src/ui/mod.rs").is_empty());
    }

    #[test]
    fn a_name_without_extension_probes_extensions_and_indexes() {
        assert_eq!(
            one("import x from './util'", "web/app/main.ts").1,
            "web/app/util.ts"
        );
        assert_eq!(
            one("import app from '../app'", "web/lib/a.ts").1,
            "web/app/index.ts"
        );
        assert_eq!(one("\"./ui\"", "src/main.rs").1, "src/ui/mod.rs");
    }

    #[test]
    fn a_line_suffix_gives_the_line_and_stays_out_of_the_range() {
        for text in [
            "src/main.rs:42",
            "src/main.rs:42:7",
            "src/main.rs#L42",
            "src/main.rs(42)",
            "\"src/main.rs:42\"",
        ] {
            let (shown, path, line) = one(text, "x");
            assert_eq!(
                (shown.as_str(), path.as_str(), line),
                ("src/main.rs", "src/main.rs", Some(42)),
                "{text}"
            );
        }
        assert_eq!(one("src/main.rs:", "x").2, None);
        assert_eq!(one("(src/main.rs).", "x").0, "src/main.rs");
    }

    #[test]
    fn the_range_points_at_the_path_in_the_line() {
        let text = "    mod x; // see src/ui/button.rs:3, or docs/guide.md.";
        let found = find(text, "src/main.rs", &|p| FILES.contains(&p));
        let shown: Vec<&str> = found.iter().map(|r| &text[r.range.clone()]).collect();
        assert_eq!(shown, ["src/ui/button.rs", "docs/guide.md"]);
        assert_eq!(found[0].line, Some(3));
        assert_eq!(found[1].line, None);
    }

    #[test]
    fn an_absurdly_long_line_has_no_references() {
        let text = format!("src/main.rs {}", "x".repeat(MAX_LINE));
        assert!(find(&text, "a", &|_| true).is_empty());
    }
}
