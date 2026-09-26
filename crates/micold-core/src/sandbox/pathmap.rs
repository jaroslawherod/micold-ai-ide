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
    // Joined as a *string*, not with `PathBuf::push`. `PathBuf` is native to whichever platform
    // compiled it, so on a Windows host `push` writes `\` separators and the container path comes
    // out as `/mnt/host\c\Users/u/p` -- half Linux, half not, and rejected by the runtime. Two
    // `PathBuf`s spelled either way still compare *equal* on Windows, which is why this survived
    // until the argv gate, whose assertions are on rendered strings, first ran there (T115).
    let mut out = String::from(WINDOWS_MOUNT_ROOT);
    out.push('/');
    out.push_str(&drive);
    if !rest.is_empty() {
        out.push('/');
        out.push_str(rest);
    }
    PathBuf::from(out)
}

/// The path a host directory is mounted at inside the container, on *this* platform.
pub fn map(host: &Path) -> PathBuf {
    map_for(host, cfg!(windows))
}

/// The host path a container path names, or `None` when the sandbox does not share it
/// (research R10 decision 3; contract link-recognition C13–C16b).
///
/// The inverse of [`map_for`], but it cannot be computed from the mapping alone: what the container
/// can see is the *mount set*, and only the locations in `locations` are shared at all. Everything
/// else inside the container — its `/tmp`, its system directories, a project this container was not
/// created with — has no host counterpart, and the safe answer for those is `None`. Opening the
/// host's own file of the same name would be the one outcome FR-018 forbids outright.
///
/// The path is normalised **lexically and first**, before any location is considered: a `..` that
/// resolved after the match would let `/work/proj/../../etc/passwd` come back as
/// `<host>/passwd`, and one that climbs above `/` names nothing at all. No filesystem is consulted,
/// so a symbolic link inside the container is followed by neither side — the host opener sees the
/// path the hint showed (SC-006).
///
/// `windows_host` is a parameter for the reason it is in [`map_for`]: the Windows join is exercised
/// by every CI job rather than by the one platform this project has no runner for.
pub fn reverse(
    locations: &[crate::link::SharedLocation],
    denied: &[String],
    container_path: &str,
    windows_host: bool,
) -> Option<String> {
    let path = normalise(container_path)?;

    // The *most specific* location wins (C15): a credential folder shared in its own right sits
    // inside the sandbox's home, and the home's mapping would send its files to the application's
    // copy instead of the host's real one. Chosen by match length rather than by input order, so a
    // caller that hands the locations over unsorted still gets the nesting right.
    let (location, rest) = locations
        .iter()
        .enumerate()
        .filter_map(|(index, location)| {
            let prefix = normalise(&location.container)?;
            let rest = path.strip_prefix(prefix.as_slice())?;
            Some((index, location, rest))
        })
        // `Reverse(index)` breaks a tie towards the earlier location, because two spellings of one
        // container path (`/home/u` and `/home/u/`) match equally and `max_by_key` otherwise keeps
        // the *last* of them. Earlier is what `MountSet::shared_locations` documents for its own
        // ties, so the two agree (review A finding 4).
        .max_by_key(|(index, location, rest)| {
            (
                path.len() - rest.len(),
                location.container.len(),
                std::cmp::Reverse(*index),
            )
        })
        .map(|(_, location, rest)| (location, rest))?;

    let separator = if windows_host { '\\' } else { '/' };
    // A component is a name in the container and is chosen there. On a Windows host Win32 reads
    // several of those names as something other than a name: `\\` and `/` are separators, so a
    // component holding one climbs out of the shared location; `:` opens a drive-relative path or an
    // alternate data stream; and a trailing dot or space is stripped, so `sandbox.token.` opens
    // `sandbox.token`. Each would make the denial below compare a string Windows would never open,
    // so none of them is a path this machine can be asked to open (review B finding 2).
    if windows_host
        && rest.iter().any(|component| {
            component.contains(['\\', '/', ':'])
                || component.ends_with('.')
                || component.ends_with(' ')
        })
    {
        return None;
    }
    let mut host = location.host.trim_end_matches(separator).to_string();
    for component in rest {
        host.push(separator);
        host.push_str(component);
    }

    // The client's own token reaches the container through the state mount, so excluding the secret
    // mount from `locations` is not enough on its own (C16b, research R10 decision 2).
    if denied
        .iter()
        .any(|forbidden| is_at_or_under(&host, forbidden, separator, windows_host))
    {
        return None;
    }
    Some(host)
}

/// A container path's components, with `.` dropped and `..` resolved; `None` when it is not
/// absolute or climbs above `/`.
fn normalise(container_path: &str) -> Option<Vec<&str>> {
    let mut components: Vec<&str> = Vec::new();
    for component in container_path.strip_prefix('/')?.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                // Above the root there is nothing, in the container or anywhere else. Refusing is
                // what stops `/project/../etc` reaching a shared prefix it is not under.
                components.pop()?;
            }
            name => components.push(name),
        }
    }
    Some(components)
}

/// Whether `path` *is* `forbidden` or lies inside it, comparing whole components.
///
/// Whole components because `sandbox.token.bak` is not the token, and case-insensitively on a
/// Windows host because there `SANDBOX.TOKEN` and `sandbox.token` are one file — a case-sensitive
/// denial there would be no denial at all.
fn is_at_or_under(path: &str, forbidden: &str, separator: char, windows_host: bool) -> bool {
    let same = |a: &str, b: &str| {
        if windows_host {
            a.eq_ignore_ascii_case(b)
        } else {
            a == b
        }
    };
    // Windows reads `/` and `\` as the same separator, so a denial written either way is the same
    // denial. Off Windows `\` is an ordinary character in a file name and is left alone.
    let spelled = |p: &str| {
        if windows_host {
            p.replace('/', "\\")
        } else {
            p.to_string()
        }
    };
    let path = &spelled(path);
    let forbidden = &spelled(forbidden);
    let forbidden = forbidden.trim_end_matches(separator);
    if same(path, forbidden) {
        return true;
    }
    // By byte, never by slice: `path` ends in a name the container chose, so a multi-byte character
    // straddling `forbidden`'s length would panic here — and `resolve` runs from the pane's every
    // frame, so that panic would repeat until the line scrolled away (review B finding 1). The
    // separator is ASCII, so a byte equal to it is a character boundary by construction.
    let n = forbidden.len();
    path.as_bytes().get(n) == Some(&(separator as u8))
        && path.is_char_boundary(n)
        && same(&path[..n], forbidden)
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
    use crate::link::SharedLocation;

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
    fn a_windows_container_path_is_spelled_the_linux_way() {
        // Deliberately on the rendered string rather than on the `PathBuf`. Comparing `PathBuf`s
        // is what let a `\`-separated container path pass here for the whole of feature 027:
        // Windows treats `/` and `\` as the same separator, so the assertion above holds on both
        // platforms even when the *text* handed to `docker -v` is only correct on one of them.
        let mapped = map_for(Path::new(r"C:\Users\u\p"), true);
        let rendered = mapped.to_string_lossy();
        assert_eq!(rendered, "/mnt/host/c/Users/u/p");
        assert!(!rendered.contains('\\'), "got {rendered:?}");
    }

    #[test]
    fn windows_mapping_is_deterministic() {
        let a = map_for(Path::new(r"C:\Users\u\p"), true);
        let b = map_for(Path::new(r"C:\Users\u\p"), true);
        assert_eq!(a, b);
    }

    /// A shared pair, spelled the way [`super::reverse`] takes them.
    fn at(container: &str, host: &str) -> SharedLocation {
        SharedLocation {
            container: container.to_string(),
            host: host.to_string(),
        }
    }

    /// U40, C15: nested locations, and the most specific of them maps the path.
    #[test]
    fn the_most_specific_of_nested_shared_locations_maps_the_path() {
        let locations = [
            at("/home/u/.claude", "/home/u/.claude"),
            at("/home/u", "/var/lib/micold-ai-ide/sandbox-home"),
        ];
        assert_eq!(
            reverse(&locations, &[], "/home/u/.claude/x", false).as_deref(),
            Some("/home/u/.claude/x"),
            "the credential folder is shared in its own right, so a file in it is the host's own \
             file and not the sandbox home's copy (C15)"
        );
        assert_eq!(
            reverse(&locations, &[], "/home/u/notes.md", false).as_deref(),
            Some("/var/lib/micold-ai-ide/sandbox-home/notes.md"),
            "a file the narrower location does not hold falls to the home location, where the \
             container really wrote it"
        );
        assert_eq!(
            reverse(&locations, &[], "/home/u", false).as_deref(),
            Some("/var/lib/micold-ai-ide/sandbox-home"),
            "the location itself maps to its host path, with nothing joined onto it"
        );
    }

    /// U43: locations match whole path components, never text prefixes.
    #[test]
    fn a_location_matches_whole_path_components_only() {
        let locations = [at("/work/proj", "/home/u/proj")];
        assert_eq!(
            reverse(&locations, &[], "/work/projector/a.md", false),
            None,
            "`/work/projector` is a different directory from `/work/proj`, and mapping it by text \
             prefix would open `/home/u/projector/a.md`'s neighbour instead"
        );
        assert_eq!(
            reverse(&locations, &[], "/work/proj/a.md", false).as_deref(),
            Some("/home/u/proj/a.md"),
            "the real child still maps"
        );
    }

    /// U42, U145, C13: normalisation is lexical and happens *before* the location match.
    #[test]
    fn a_path_is_normalised_before_it_is_matched_and_never_climbs_above_root() {
        let locations = [at("/work/proj", "/home/u/proj")];
        assert_eq!(
            reverse(&locations, &[], "/work/proj/../../etc/passwd", false),
            None,
            "the path names /etc/passwd, which no location shares — normalising after the match \
             would have handed back /home/u/passwd (C13, U145)"
        );
        assert_eq!(
            reverse(&locations, &[], "/work/proj/a/../b", false).as_deref(),
            Some("/home/u/proj/b"),
            "a `..` inside the location resolves and the remainder is what is left (U145)"
        );
        assert_eq!(
            reverse(&locations, &[], "/../etc/passwd", false),
            None,
            "a path that climbs above `/` names nothing at all (U42)"
        );
        assert_eq!(
            reverse(&locations, &[], "/work/proj/./a.md", false).as_deref(),
            Some("/home/u/proj/a.md"),
            "a `.` component is dropped"
        );
        assert_eq!(
            reverse(&locations, &[], "work/proj/a.md", false),
            None,
            "a container path is absolute; a relative one names nothing"
        );
    }

    /// U48, C14: a Windows host's pairs join with its own separator.
    #[test]
    fn a_windows_host_location_joins_the_remainder_with_backslashes() {
        let locations = [at("/mnt/host/c/Users/u/p", r"C:\Users\u\p")];
        assert_eq!(
            reverse(&locations, &[], "/mnt/host/c/Users/u/p/docs/a.md", true).as_deref(),
            Some(r"C:\Users\u\p\docs\a.md"),
            "the host path is Windows's, so the remainder is joined the Windows way (C14)"
        );
        assert_eq!(
            reverse(&locations, &[], "/mnt/host/c/Users/u/p", true).as_deref(),
            Some(r"C:\Users\u\p"),
            "the location itself keeps its own spelling"
        );
    }

    /// U44, C16b: a result at or under a denied host path is no result.
    #[test]
    fn a_result_equal_to_or_under_a_denied_host_path_maps_to_nothing() {
        let state = at(
            "/var/lib/micold-ai-ide",
            "/home/u/.local/share/micold-ai-ide",
        );
        let denied = ["/home/u/.local/share/micold-ai-ide/sandbox.token".to_string()];
        assert_eq!(
            reverse(
                std::slice::from_ref(&state),
                &denied,
                "/var/lib/micold-ai-ide/sandbox.token",
                false
            ),
            None,
            "the client's own token is reachable through the state mount, and handing it to the \
             system opener is exactly what `denied` exists to stop (C16b)"
        );
        assert_eq!(
            reverse(
                std::slice::from_ref(&state),
                &denied,
                "/var/lib/micold-ai-ide/sandbox.token/x",
                false
            ),
            None,
            "and nothing under it either"
        );
        assert_eq!(
            reverse(
                std::slice::from_ref(&state),
                &denied,
                "/var/lib/micold-ai-ide/projects.json",
                false
            )
            .as_deref(),
            Some("/home/u/.local/share/micold-ai-ide/projects.json"),
            "a sibling of the token is not denied: the rule is the path, not the directory"
        );
        assert_eq!(
            reverse(
                &[state],
                &denied,
                "/var/lib/micold-ai-ide/sandbox.token.bak",
                false
            )
            .as_deref(),
            Some("/home/u/.local/share/micold-ai-ide/sandbox.token.bak"),
            "`sandbox.token.bak` is not the token: the denial matches whole components"
        );
    }

    /// U44 on Windows: the denial cannot be walked around by spelling the path differently.
    #[test]
    fn a_denied_windows_path_is_denied_whatever_its_case() {
        let state = at("/var/lib/micold-ai-ide", r"C:\Users\u\AppData\micold");
        let denied = [r"c:\users\u\appdata\micold\sandbox.token".to_string()];
        assert_eq!(
            reverse(
                &[state],
                &denied,
                "/var/lib/micold-ai-ide/sandbox.token",
                true
            ),
            None,
            "Windows paths are case-insensitive, so a case-sensitive denial would be no denial"
        );
    }

    /// U44, review B finding 1: the denial reads a container-chosen name, so it may never index it
    /// by a byte offset — a multi-byte character straddling the denied path's length would panic,
    /// and `resolve` runs from the pane's every frame, so the crash would repeat until the line
    /// scrolled away.
    #[test]
    fn a_multi_byte_name_at_the_denied_paths_own_length_is_answered_not_panicked() {
        let state = at(
            "/var/lib/micold-ai-ide",
            "/home/u/.local/share/micold-ai-ide",
        );
        let denied = ["/home/u/.local/share/micold-ai-ide/sandbox.token".to_string()];
        // "sandbox.toke" plus a three-byte character: two bytes longer than `sandbox.token`, with no
        // character boundary where that name ends.
        assert_eq!(
            reverse(
                &[state],
                &denied,
                "/var/lib/micold-ai-ide/sandbox.tokeあ",
                false
            )
            .as_deref(),
            Some("/home/u/.local/share/micold-ai-ide/sandbox.tokeあ"),
            "it is not the token, so it maps — and asking the question must not panic"
        );
    }

    /// U48, review B finding 2: on a Windows host a container-chosen component may not carry
    /// anything Win32 reads as a separator or a stream, or the joined path escapes its location and
    /// the denial compares a string Windows would never open.
    #[test]
    fn a_windows_host_refuses_a_component_windows_would_read_as_more_than_a_name() {
        let location = at("/mnt/host/c/Users/u/p", r"C:\Users\u\p");
        let denied = [r"C:\Users\u\AppData\micold\sandbox.token".to_string()];
        for container in [
            // `\` is an ordinary character in a Linux name and a separator to Win32: joined in, it
            // would climb out of the shared location.
            r"/mnt/host/c/Users/u/p/..\..\..\Users\u\AppData\micold\sandbox.token",
            // A drive-relative name and an alternate data stream, both of which name something else.
            "/mnt/host/c/Users/u/p/C:notes.txt",
            "/mnt/host/c/Users/u/p/notes.txt:hidden",
            // Win32 strips a trailing dot or space, so `sandbox.token.` opens `sandbox.token`.
            "/mnt/host/c/Users/u/p/notes.txt.",
            "/mnt/host/c/Users/u/p/notes.txt ",
        ] {
            assert_eq!(
                reverse(std::slice::from_ref(&location), &denied, container, true),
                None,
                "{container} is not a path this machine can be asked to open"
            );
        }
        assert_eq!(
            reverse(
                std::slice::from_ref(&location),
                &denied,
                "/mnt/host/c/Users/u/p/notes.txt",
                true
            )
            .as_deref(),
            Some(r"C:\Users\u\p\notes.txt"),
            "an ordinary name still maps"
        );
        let _ = location;
        assert_eq!(
            reverse(
                &[at("/work/proj", "/home/u/proj")],
                &[],
                r"/work/proj/a\b.md",
                false
            )
            .as_deref(),
            Some(r"/home/u/proj/a\b.md"),
            "off Windows a backslash is an ordinary character in a name and is left alone"
        );
    }

    #[test]
    fn a_path_in_no_shared_location_maps_to_nothing() {
        assert_eq!(
            reverse(&[at("/work/proj", "/home/u/proj")], &[], "/tmp/x", false),
            None,
            "C12: the container's own /tmp is not this machine's"
        );
        assert_eq!(
            reverse(&[], &[], "/work/proj/a.md", false),
            None,
            "a sandbox that shares nothing reaches nothing"
        );
    }

    #[test]
    fn windows_is_honest_about_not_being_the_identity_mapping() {
        // The honesty matters: code that assumes identity is correct on two platforms and wrong on
        // the third, and the wrongness shows up as a broken worktree, not as a failed mount.
        assert!(!is_identity_for(true));
    }
}
