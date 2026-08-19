//! Host ↔ sandbox path identity (research R2).
//!
//! Git records **absolute** paths in worktree metadata — in `.git/worktrees/<name>/gitdir` and in
//! each worktree's own `.git` file — and both processes run git: the daemon
//! (`micold-daemon/src/server.rs`) and the client (`micold-client/src/shell/workspace.rs`). If the
//! container sees a project at a different path than the host does, the two disagree about
//! `git worktree list`, and a worktree created by one is broken for the other. That is a Principle
//! III failure, not a cosmetic mismatch.
//!
//! So on Linux and macOS a project is mounted at its own absolute path — `/home/u/p` at
//! `/home/u/p` — which costs nothing and removes the problem outright.
//!
//! # Windows
//!
//! `C:\Users\u\p` has no Linux-container equivalent, so the mapping is unavoidable there. The
//! resolution is **not** to translate paths and hope both sides agree: it is to stop running git on
//! the host at all, by routing the client's git calls through the daemon (the client already
//! funnels every one of them through a single injected capability, so this is a new impl of an
//! existing trait rather than a refactor). That is also the only answer that survives the remote
//! placement FR-003a promises, since a remote daemon leaves no host filesystem to run git against
//! at any path.
//!
//! Until that lands, [`map`] produces a deterministic container path on Windows so the sandbox is
//! usable, and [`is_identity`] reports honestly that it is not the identity mapping.

use std::path::{Path, PathBuf};

/// Where Windows host paths appear inside the container.
///
/// A fixed prefix rather than a per-project one so the layout is predictable in diagnostics and
/// does not depend on the order projects were registered.
pub const WINDOWS_MOUNT_ROOT: &str = "/mnt/host";

/// The path a host directory is mounted at inside the container.
///
/// Pure and platform-parameterised rather than `cfg`-gated, so a Linux CI run can exercise the
/// Windows mapping and vice versa — the alternative is a branch that only one of the three
/// platforms ever compiles, which is how parity bugs survive.
pub fn map_for(host: &Path, windows_host: bool) -> PathBuf {
    if !windows_host {
        return host.to_path_buf();
    }

    // `C:\Users\u\p` -> `/mnt/host/c/Users/u/p`. The drive letter becomes a path segment so two
    // projects on different drives cannot collide, and separators are normalised because the
    // container is Linux regardless of the host.
    let raw = host.to_string_lossy().replace('\\', "/");
    let (drive, rest) = match raw.split_once(':') {
        Some((d, r))
            if d.len() == 1 && d.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) =>
        {
            (d.to_ascii_lowercase(), r)
        }
        // A UNC path or something else unusual: keep it whole under a `share` segment rather than
        // silently producing a path that collides with a drive-letter one.
        _ => ("share".to_string(), raw.as_str()),
    };
    let rest = rest.trim_start_matches('/');
    let mut out = PathBuf::from(WINDOWS_MOUNT_ROOT);
    out.push(drive);
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

/// The path a host directory is mounted at inside the container, on *this* platform.
pub fn map(host: &Path) -> PathBuf {
    map_for(host, cfg!(windows))
}

/// Whether this platform mounts projects at their own absolute paths.
///
/// `true` on Linux and macOS, where git on both sides sees one set of paths. `false` on Windows,
/// which is exactly the condition that makes the daemon-backed git capability necessary there.
pub fn is_identity_for(windows_host: bool) -> bool {
    !windows_host
}

/// Whether *this* platform mounts projects at their own absolute paths.
pub fn is_identity() -> bool {
    is_identity_for(cfg!(windows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_paths_map_to_themselves() {
        // Conformance check K-5 in miniature, and the claim git's worktree metadata depends on.
        for p in [
            "/home/u/projects/micold",
            "/Users/u/src/micold",
            "/home/u/p/.claude/worktrees/feat-x",
        ] {
            assert_eq!(map_for(Path::new(p), false), PathBuf::from(p));
        }
        assert!(is_identity_for(false));
    }

    #[test]
    fn a_windows_drive_letter_becomes_a_path_segment() {
        assert_eq!(
            map_for(Path::new(r"C:\Users\u\p"), true),
            PathBuf::from("/mnt/host/c/Users/u/p")
        );
    }

    #[test]
    fn two_drives_cannot_collide() {
        // The reason the drive letter is kept at all: `C:\p` and `D:\p` are different directories,
        // and a mapping that dropped the drive would mount one over the other.
        let c = map_for(Path::new(r"C:\p"), true);
        let d = map_for(Path::new(r"D:\p"), true);
        assert_ne!(c, d);
    }

    #[test]
    fn a_unc_path_does_not_collide_with_a_drive_path() {
        let unc = map_for(Path::new(r"\\server\share\p"), true);
        assert!(unc.starts_with("/mnt/host/share"), "got {unc:?}");
        assert_ne!(unc, map_for(Path::new(r"S:\share\p"), true));
    }

    #[test]
    fn windows_mapping_is_deterministic() {
        let a = map_for(Path::new(r"C:\Users\u\p"), true);
        let b = map_for(Path::new(r"C:\Users\u\p"), true);
        assert_eq!(a, b);
    }

    #[test]
    fn windows_is_honest_about_not_being_the_identity_mapping() {
        // The honesty matters: code that assumes identity is correct on two platforms and wrong on
        // the third, and the wrongness shows up as a broken worktree, not as a failed mount.
        assert!(!is_identity_for(true));
    }
}
