//! FR-008: the orchestrator never dials a daemon. Daemons dial it, so nothing under the orchestrator's
//! source may call `connect`, `Stream::connect` or `open_pipe` (the client side of a local socket or a
//! Windows pipe). This is the source-level half of the proof; `netns.rs` is the runtime half.

use std::path::{Path, PathBuf};

/// The directories whose source must hold no outbound call, from the workspace root.
const SCANNED: [&str; 2] = [
    "crates/micold-daemon/src/orchestrator",
    "crates/micold-core/src/orchestrator",
];

/// Calls that open a connection. Matched as whole words, so `Stream::connect` is caught by `connect`
/// and `disconnect` or `connected` are not.
const FORBIDDEN: [&str; 3] = ["connect", "Stream::connect", "open_pipe"];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the daemon crate sits two levels below the workspace root")
        .to_path_buf()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// `source` with comments and string literals blanked (newlines kept, so line numbers hold).
fn code_only(source: &str) -> String {
    let b: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let next = b.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < b.len() && b[i] != '\n' {
                out.push(' ');
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < b.len() {
                if b[i] == '/' && b.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    i += 2;
                } else if b[i] == '*' && b.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(b[i]));
                    i += 1;
                }
            }
        } else if c == '"' || (c == 'r' && matches!(next, Some('"' | '#')) && raw_start(&b, i)) {
            let hashes = if c == 'r' {
                b[i + 1..].iter().take_while(|&&h| h == '#').count()
            } else {
                0
            };
            let mut closing = vec!['"'];
            closing.extend(std::iter::repeat_n('#', hashes));
            i += if c == 'r' { 2 + hashes } else { 1 };
            out.push(' ');
            while i < b.len() {
                if c == '"' && b[i] == '\\' {
                    out.push_str("  ");
                    i += 2;
                    continue;
                }
                if b[i..].starts_with(&closing) {
                    out.extend(std::iter::repeat_n(' ', closing.len()));
                    i += closing.len();
                    break;
                }
                out.push(blank(b[i]));
                i += 1;
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Whether the `r` at `i` opens a raw string (`r"` or `r#…#"`) rather than ending an identifier.
fn raw_start(b: &[char], i: usize) -> bool {
    let after = b[i + 1..].iter().find(|&&h| h != '#');
    after == Some(&'"') && !(i > 0 && (b[i - 1].is_alphanumeric() || b[i - 1] == '_'))
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Line numbers where `word` occurs delimited by non-identifier characters.
fn whole_word_lines(code: &str, word: &str) -> Vec<usize> {
    let mut lines = Vec::new();
    for (n, line) in code.lines().enumerate() {
        let mut from = 0;
        while let Some(at) = line[from..].find(word) {
            let start = from + at;
            let end = start + word.len();
            let before_ok = !line[..start].chars().next_back().is_some_and(is_word_char);
            let after_ok = !line[end..].chars().next().is_some_and(is_word_char);
            if before_ok && after_ok {
                lines.push(n + 1);
                break;
            }
            from = end;
        }
    }
    lines
}

fn violations(source: &str) -> Vec<(usize, &'static str)> {
    let code = code_only(source);
    let mut found: Vec<_> = FORBIDDEN
        .iter()
        .flat_map(|word| {
            whole_word_lines(&code, word)
                .into_iter()
                .map(|l| (l, *word))
        })
        .collect();
    found.sort();
    found
}

#[test]
fn the_orchestrator_source_opens_no_outbound_connection() {
    let root = workspace_root();
    let mut files = Vec::new();
    for dir in SCANNED {
        rust_files(&root.join(dir), &mut files);
    }
    assert!(
        files.len() >= 8,
        "found only {files:?}: the scan is not looking at the sources"
    );

    let mut report = Vec::new();
    for file in &files {
        let source = std::fs::read_to_string(file).unwrap();
        for (line, word) in violations(&source) {
            report.push(format!("{}:{line}: `{word}`", file.display()));
        }
    }
    assert!(
        report.is_empty(),
        "outbound calls (FR-008):\n{}",
        report.join("\n")
    );
}

#[test]
fn the_scan_catches_a_call_and_ignores_comments_strings_and_longer_words() {
    let bad = "fn f() {\n    let s = Stream::connect(name).await;\n    open_pipe(p);\n}\n";
    assert_eq!(
        violations(bad),
        [(2, "Stream::connect"), (2, "connect"), (3, "open_pipe")]
    );
    let bad_method = "fn f() { sock.connect(addr); }";
    assert_eq!(violations(bad_method), [(1, "connect")]);

    let fine = concat!(
        "// connect here\n",
        "/* open_pipe /* nested connect */ still comment connect */\n",
        "let a = \"connect\"; let b = r#\"open_pipe \"quoted\" connect\"#;\n",
        "fn disconnect() { reconnect_later(); connected(); }\n",
    );
    assert_eq!(violations(fine), []);
}
