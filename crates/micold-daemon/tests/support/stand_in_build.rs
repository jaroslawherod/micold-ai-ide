//! Compiles a test's stand-in `claude` once and shares it between test processes (#1761).
//!
//! The binary lives in `CARGO_TARGET_TMPDIR/stand-in-<hash of its source>/`, which `mise run sweep`
//! reclaims, so a rerun finds it and nothing is left in the system temporary directory.
#![allow(dead_code)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// How many times this process ran `rustc`.
pub static RUSTC_RUNS: AtomicUsize = AtomicUsize::new(0);

/// The directory holding `exe_name`, compiled from `source_text` (edition 2021, size-optimised).
pub fn stand_in_dir(source_text: &str, exe_name: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    (source_text, exe_name).hash(&mut hasher);
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let dir = root.join(format!("stand-in-{:016x}", hasher.finish()));
    if dir.join(exe_name).exists() {
        return dir;
    }
    // Compile aside and rename, so parallel test processes never see a half-written binary.
    let partial = root.join(format!(
        "stand-in-{:016x}.{}.tmp",
        hasher.finish(),
        std::process::id()
    ));
    std::fs::create_dir_all(&partial).expect("a directory for the stand-in");
    let source = partial.join("stand_in.rs");
    std::fs::write(&source, source_text).expect("write the stand-in's source");
    let rustc_name = if cfg!(windows) { "rustc.exe" } else { "rustc" };
    let rustc = std::env::var_os("RUSTC")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("CARGO")
                .map(PathBuf::from)
                .and_then(|cargo| cargo.parent().map(|dir| dir.join(rustc_name)))
                .filter(|rustc| rustc.exists())
        })
        .unwrap_or_else(|| PathBuf::from("rustc"));
    RUSTC_RUNS.fetch_add(1, Ordering::SeqCst);
    let out = std::process::Command::new(&rustc)
        .args(["--edition", "2021"])
        .args(["-C", "opt-level=s", "-C", "strip=symbols"])
        .args(["-C", "debuginfo=0", "-C", "panic=abort"])
        .arg(&source)
        .arg("-o")
        .arg(partial.join(exe_name))
        .output()
        .unwrap_or_else(|err| panic!("run {}: {err}", rustc.display()));
    if !out.status.success() {
        let _ = std::fs::remove_dir_all(&partial);
    }
    assert!(
        out.status.success(),
        "compile the stand-in with {}: {}",
        rustc.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    if std::fs::rename(&partial, &dir).is_err() {
        // Another process published it first.
        let _ = std::fs::remove_dir_all(&partial);
        assert!(dir.join(exe_name).exists(), "publish the stand-in");
    }
    dir
}

/// A fresh scratch directory for a test process's home, under the target directory's temporary
/// area; a `static` keeps it for the process and `mise run sweep` reclaims it.
pub fn scratch_home(prefix: &str) -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(root).expect("the target temporary directory");
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(root)
        .expect("a claude config directory")
}
