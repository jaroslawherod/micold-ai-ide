//! Every background spawn that can run on Windows hides its console (feature 030, FR-024, E6.2).
//!
//! The installed app and daemon are GUI-subsystem programs. When one of them starts a console
//! program (`git`, `powershell`, `docker`), Windows gives that child a fresh console window unless
//! the spawn passes `CREATE_NO_WINDOW`. Nothing fails: a black window flashes up on every `git`
//! call, and on a busy project that is several per second. No test on Linux or macOS can see it,
//! and the Windows smoke test sees it only for the paths it happens to drive, so the rule is
//! checked where it is written: every `Command::new(` in production source is wrapped in
//! `micold_core::process::no_window`.
//!
//! A spawn is exempt when its function only compiles off Windows, or when it is listed in
//! [`ALLOWED`] with the reason it needs no wrapper.
//!
//! # Why text, not a parser
//!
//! Spawn sites follow two shapes in this codebase: a builder chain that starts at
//! `Command::new(`, and a `let mut cmd = Command::new(..)` configured over later statements. Both
//! are a line-local question plus the function they sit in, which a line scan answers exactly.
//! Parsing Rust would add a dependency for no extra certainty.

use std::fs;
use std::path::{Path, PathBuf};

/// Spawn sites that need no `no_window`, as (repo-relative file, enclosing function, reason).
const ALLOWED: &[(&str, &str, &str)] = &[(
    "crates/micold-core/src/spawn.rs",
    "spawn_detached_daemon",
    "the daemon spawn uses DETACHED_PROCESS, and Windows ignores CREATE_NO_WINDOW combined with it",
)];

/// `cfg` attributes that compile a function out of Windows builds.
const NOT_WINDOWS_CFGS: &[&str] = &[
    "#[cfg(unix)]",
    "#[cfg(not(windows))]",
    "#[cfg(target_os = \"linux\")]",
    "#[cfg(target_os = \"macos\")]",
];

const SPAWN: &str = "Command::new(";

fn repo_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Every `crates/*/src/**/*.rs`, as repo-relative paths with `/` separators.
fn production_sources() -> Vec<(String, String)> {
    let root = repo_root();
    let mut files = Vec::new();
    for krate in fs::read_dir(root.join("crates")).expect("read crates/") {
        let src = krate.expect("crate entry").path().join("src");
        if src.is_dir() {
            rust_sources(&src, &mut files);
        }
    }
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let relative = path
                .strip_prefix(&root)
                .expect("under the repo root")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            (relative, source)
        })
        .collect()
}

/// The lines before the file's `#[cfg(test)] mod` block, which by convention closes the file.
fn non_test_lines(source: &str) -> Vec<&str> {
    let lines: Vec<&str> = source.lines().collect();
    let end = lines
        .iter()
        .enumerate()
        .position(|(i, line)| {
            line.trim() == "#[cfg(test)]"
                && lines
                    .get(i + 1)
                    .is_some_and(|next| next.trim_start().starts_with("mod "))
        })
        .unwrap_or(lines.len());
    lines[..end].to_vec()
}

/// The name of the function whose signature is `line`, if it is one.
fn fn_name(line: &str) -> Option<&str> {
    let (_, rest) = line.trim_start().split_once("fn ")?;
    let before = line.trim_start().split_once("fn ")?.0;
    let is_signature = before.split_whitespace().all(|word| {
        word == "pub" || word.starts_with("pub(") || matches!(word, "async" | "unsafe" | "const")
    });
    is_signature.then(|| rest.split(['(', '<']).next().unwrap_or(rest).trim())
}

/// The index of the signature line of the function enclosing `index`.
fn enclosing_fn(lines: &[&str], index: usize) -> Option<usize> {
    (0..=index).rev().find(|&i| fn_name(lines[i]).is_some())
}

/// `true` when the attributes directly above the signature at `signature` exclude Windows.
fn compiled_out_of_windows(lines: &[&str], signature: usize) -> bool {
    lines[..signature]
        .iter()
        .rev()
        .map(|line| line.trim())
        .take_while(|line| line.starts_with("#[") || line.starts_with("///"))
        .any(|line| NOT_WINDOWS_CFGS.contains(&line))
}

/// `true` when the spawn on `lines[index]` goes through `no_window`.
fn hides_console(lines: &[&str], index: usize, body_end: usize) -> bool {
    let line = lines[index];
    if line.contains(&format!("no_window(&mut {SPAWN}")) {
        return true;
    }
    // `let mut cmd = Command::new(..)`, configured over later statements in the same function.
    let Some(binding) = line
        .trim_start()
        .strip_prefix("let mut ")
        .and_then(|rest| rest.split_once(" = "))
        .map(|(name, _)| name.trim())
    else {
        return false;
    };
    let wrapped = format!("no_window(&mut {binding})");
    lines[index..body_end]
        .iter()
        .any(|later| later.contains(&wrapped))
}

#[test]
fn every_background_spawn_hides_its_console() {
    let mut spawns = 0;
    let mut allowed_seen = Vec::new();
    let mut offenders = Vec::new();

    for (file, source) in production_sources() {
        let lines = non_test_lines(&source);
        for (index, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("//") || !line.contains(SPAWN) {
                continue;
            }
            spawns += 1;
            let signature = enclosing_fn(&lines, index);
            let function = signature.and_then(|i| fn_name(lines[i])).unwrap_or("");
            if signature.is_some_and(|i| compiled_out_of_windows(&lines, i)) {
                continue;
            }
            if ALLOWED
                .iter()
                .any(|(f, name, _)| *f == file && *name == function)
            {
                allowed_seen.push((file.clone(), function.to_string()));
                continue;
            }
            let body_end = (index + 1..lines.len())
                .find(|&i| fn_name(lines[i]).is_some())
                .unwrap_or(lines.len());
            if !hides_console(&lines, index, body_end) {
                offenders.push(format!("{file}:{} (in `{function}`)", index + 1));
            }
        }
    }

    assert!(
        spawns > 0,
        "found no `{SPAWN}` in crates/*/src — the scan is broken, not the code"
    );
    for (file, function, reason) in ALLOWED {
        assert!(
            allowed_seen.iter().any(|(f, n)| f == file && n == function),
            "ALLOWED lists `{function}` in {file} ({reason}) but no spawn is there — a stale \
             exemption hides the next real one"
        );
    }
    assert!(
        offenders.is_empty(),
        "these spawns can run on Windows and do not hide their console, so each call flashes a \
         console window over the app: {offenders:#?}\n\
         Wrap the command: `micold_core::process::no_window(&mut Command::new(..))`, or \
         `no_window(&mut cmd)` for a `let mut cmd` binding. A spawn that genuinely needs its \
         console goes in ALLOWED with the reason."
    );
}
