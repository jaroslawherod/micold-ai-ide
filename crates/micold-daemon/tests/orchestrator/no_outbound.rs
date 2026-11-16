//! FR-008: the orchestrator never dials a daemon. Daemons dial it, so nothing under the orchestrator's
//! source may call `connect`, `Stream::connect`, `open_pipe`, `dial` or a `connect_*` helper (the client
//! side of a local socket or a Windows pipe), or reach a module that dials. The one allowed route to a
//! dial is `crate::singleton::acquire`, which probes the orchestrator's *own* socket to see whether
//! another orchestrator holds it: a self-probe, not a daemon. This is the source-level half of the
//! proof; `netns.rs` is the runtime half.

use std::path::{Path, PathBuf};

/// The directories whose source must hold no outbound call, from the workspace root.
const SCANNED: [&str; 2] = [
    "crates/micold-daemon/src/orchestrator",
    "crates/micold-core/src/orchestrator",
];

/// Calls that open a connection. Matched as whole words, so `Stream::connect` is caught by `connect`
/// and `disconnect` or `connected` are not.
const FORBIDDEN: [&str; 4] = ["connect", "Stream::connect", "open_pipe", "dial"];

/// Prefixes of identifiers that dial (`connect_daemon`, `connect_to`): matched at a word start.
const FORBIDDEN_PREFIXES: [&str; 1] = ["connect_"];

/// Modules that dial, so any path into them is forbidden, except the allow-listed items below.
const FORBIDDEN_PATHS: [&str; 3] = [
    "micold_core::connect",
    "crate::remote",
    "crate::singleton::",
];

/// The self-probe: `crate::singleton::acquire` and its result type (see the module doc).
const ALLOWED_PATHS: [&str; 2] = ["crate::singleton::acquire", "crate::singleton::Acquisition"];

/// The daemon's retry and status rules (#1725) live in the core orchestrator directory under a name
/// that says what they are for. The file is pure: it holds no I/O at all, which
/// [`the_dialer_rules_file_holds_no_io`] checks, so the scan skips it and the two lines naming it.
const PURE_DIAL_RULES: &str = "crates/micold-core/src/orchestrator/dial.rs";

/// The lines of `mod.rs` that declare and re-export [`PURE_DIAL_RULES`], blanked before the scan.
const PURE_DIAL_LINES: [&str; 2] = ["pub mod dial;", "pub use dial::"];

/// The SQL store (#1948) opens a database, never a daemon: its pool connecting to a SQLite file or
/// the operator's PostgreSQL is not covered by FR-008. Only those call sites, in this directory,
/// are blanked before the scan; anything else there that dials is still reported. `netns.rs` still
/// proves the running orchestrator dials no daemon.
const DATABASE_DIR: &str = "crates/micold-daemon/src/orchestrator/db";

/// The database pool's connect calls in [`DATABASE_DIR`].
const DATABASE_LINES: [&str; 1] = [".connect_with("];

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
        } else if c == '\'' && char_literal_len(&b, i).is_some() {
            let len = char_literal_len(&b, i).unwrap();
            out.extend(std::iter::repeat_n(' ', len));
            i += len;
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

/// The length of the char literal starting at `i` (`'x'`, `'"'`, `'\n'`, `'\u{1F600}'`), or `None` when
/// the quote opens a lifetime.
fn char_literal_len(b: &[char], i: usize) -> Option<usize> {
    match (b.get(i + 1)?, b.get(i + 2)) {
        ('\\', _) => (i + 3..b.len().min(i + 14))
            .find(|&j| b[j] == '\'')
            .map(|j| j + 1 - i),
        (_, Some('\'')) => Some(3),
        _ => None,
    }
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

/// Line numbers where an identifier starts with `prefix`.
fn prefix_lines(code: &str, prefix: &str) -> Vec<usize> {
    code.lines()
        .enumerate()
        .filter(|(_, line)| {
            line.match_indices(prefix)
                .any(|(at, _)| !line[..at].chars().next_back().is_some_and(is_word_char))
        })
        .map(|(n, _)| n + 1)
        .collect()
}

/// Line numbers of a forbidden module path that is not one of the allowed ones.
fn path_lines(code: &str, path: &str) -> Vec<usize> {
    code.lines()
        .enumerate()
        .filter(|(_, line)| {
            line.match_indices(path)
                .any(|(at, _)| !ALLOWED_PATHS.iter().any(|ok| line[at..].starts_with(ok)))
        })
        .map(|(n, _)| n + 1)
        .collect()
}

fn violations(source: &str) -> Vec<(usize, &'static str)> {
    let code = code_only(source);
    let mut found = Vec::new();
    for word in FORBIDDEN {
        found.extend(whole_word_lines(&code, word).into_iter().map(|l| (l, word)));
    }
    for prefix in FORBIDDEN_PREFIXES {
        found.extend(prefix_lines(&code, prefix).into_iter().map(|l| (l, prefix)));
    }
    for path in FORBIDDEN_PATHS {
        found.extend(path_lines(&code, path).into_iter().map(|l| (l, path)));
    }
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
        if file.ends_with(PURE_DIAL_RULES) {
            continue;
        }
        let mut source = std::fs::read_to_string(file).unwrap();
        for line in PURE_DIAL_LINES {
            source = source.replace(line, &" ".repeat(line.len()));
        }
        if file.starts_with(root.join(DATABASE_DIR)) {
            for line in DATABASE_LINES {
                source = source.replace(line, &" ".repeat(line.len()));
            }
        }
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

/// The file the scan skips may not grow an I/O dependency unnoticed.
#[test]
fn the_dialer_rules_file_holds_no_io() {
    let source = std::fs::read_to_string(workspace_root().join(PURE_DIAL_RULES)).unwrap();
    let code = code_only(&source);
    for word in [
        "tokio",
        "interprocess",
        "std::net",
        "std::os",
        "std::fs",
        "std::io",
    ] {
        assert!(!code.contains(word), "{PURE_DIAL_RULES} uses `{word}`");
    }
    assert_eq!(
        violations(&source),
        [],
        "{PURE_DIAL_RULES} holds a forbidden call or path"
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
    assert_eq!(
        violations("fn f() { connect_daemon(); dial(); }"),
        [(1, "connect_"), (1, "dial")]
    );
    assert_eq!(
        violations("use crate::remote::x; use crate::singleton::probe;"),
        [(1, "crate::remote"), (1, "crate::singleton::")]
    );
    assert_eq!(
        violations("crate::singleton::acquire(&e); crate::singleton::Acquisition::Bound(b);"),
        []
    );
    // A `'"'` is a char, not the start of a string that would hide the call after it.
    assert_eq!(
        violations("let q = '\"'; sock.connect(a);"),
        [(1, "connect")]
    );
    assert_eq!(
        violations("fn f<'a>(x: &'a str) { connect(x) }"),
        [(1, "connect")]
    );

    let fine = concat!(
        "// connect here\n",
        "/* open_pipe /* nested connect */ still comment connect */\n",
        "let a = \"connect\"; let b = r#\"open_pipe \"quoted\" connect\"#;\n",
        "fn disconnect() { reconnect_later(); connected(); }\n",
    );
    assert_eq!(violations(fine), []);
}
