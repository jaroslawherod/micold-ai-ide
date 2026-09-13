//! Attributing a file-access failure to the macOS permission that caused it (feature 028, FR-028).
//!
//! On macOS a folder the user can plainly see in Finder is not a folder this application may read.
//! TCC gates Documents, Desktop, Downloads, removable drives and network volumes per application,
//! and until the user answers a prompt — or after they decline one — every read under those paths
//! fails with `EACCES`. The operating system reports exactly what it reports for a file mode
//! problem, so the honest generic message ("permission denied") sends the user to `ls -l` and
//! `chmod`, neither of which has anything to do with it.
//!
//! What this module adds is the one fact the error itself cannot carry: *which* permission is
//! missing, so the message can name it and say where it is granted.
//!
//! # Two rules, and the second is what keeps the first useful
//!
//! A failure is attributed only when the error really is `PermissionDenied` **and** the path really
//! is under a location macOS gates. A permission error anywhere else stays a plain permission
//! error. Over-attributing is its own kind of wrong message: telling someone to visit System
//! Settings about a file whose mode is `000` wastes more of their time than saying nothing.
//!
//! Every non-macOS platform answers [`None`], so no other platform's error text changes
//! (Constitution Principle VI). The classification itself is path arithmetic and runs everywhere,
//! which is what makes it testable on all three.
//!
//! # One list, two consumers
//!
//! [`ProtectedLocation`]'s variants and the `NS*UsageDescription` keys in
//! `packaging/macos/Info.plist.in` are the same list. macOS only offers to grant a permission it
//! has a purpose string for, so a location classified here without a string there would produce
//! the worst possible message: one telling the user to grant something the system never asks
//! about. `crates/micold-core/tests/macos_permission_strings.rs` holds the two together.

use std::io;
use std::path::{Path, PathBuf};

/// A folder macOS gates behind a per-application permission.
///
/// Adding a variant means adding the matching `NS*UsageDescription` key to
/// `packaging/macos/Info.plist.in`; the test above fails until it is there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectedLocation {
    Documents,
    Desktop,
    Downloads,
    RemovableVolume,
    NetworkVolume,
}

impl ProtectedLocation {
    /// The name macOS itself uses in its prompt and in System Settings, so the message and the
    /// screen the user is being sent to agree.
    pub fn system_name(&self) -> &'static str {
        match self {
            ProtectedLocation::Documents => "Documents Folder",
            ProtectedLocation::Desktop => "Desktop Folder",
            ProtectedLocation::Downloads => "Downloads Folder",
            ProtectedLocation::RemovableVolume => "Removable Volumes",
            ProtectedLocation::NetworkVolume => "Network Volumes",
        }
    }
}

/// A file access macOS refused because of a permission, not a file mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionFailure {
    /// Which gated location the path fell under.
    pub location: ProtectedLocation,
    /// The path that was refused, for the message.
    pub path: PathBuf,
}

impl PermissionFailure {
    /// One user-facing explanation: what was refused, why, and where it is granted.
    ///
    /// It names the System Settings pane rather than describing it, because the user's next action
    /// is to find that pane — and it says the application must be reopened afterwards, which is the
    /// step that otherwise makes a correctly granted permission look like it did not work.
    pub fn user_message(&self) -> String {
        format!(
            "macOS blocked access to {}. Micold AI IDE hasn't been granted “{}” yet. Open System \
             Settings › Privacy & Security › {}, switch Micold AI IDE on, then reopen the app.",
            self.path.display(),
            self.location.system_name(),
            self.location.system_name(),
        )
    }
}

/// Attribute an I/O failure to a macOS permission, or leave it alone.
///
/// [`None`] on every platform but macOS, and on macOS for any error that is not
/// `PermissionDenied` or any path outside a gated location.
pub fn classify(err: &io::Error, path: &Path) -> Option<PermissionFailure> {
    classify_on(
        err.kind(),
        path,
        home_dir().as_deref(),
        cfg!(target_os = "macos"),
    )
}

/// The rule, with its inputs. Split out so the macOS behaviour is exercised on every platform:
/// a `cfg`-gated body would be checked by one third of the matrix, and the parity requirement
/// (Principle VI) is precisely that this answers the same way everywhere it is asked the same
/// question.
fn classify_on(
    kind: io::ErrorKind,
    path: &Path,
    home: Option<&Path>,
    on_macos: bool,
) -> Option<PermissionFailure> {
    if !on_macos || kind != io::ErrorKind::PermissionDenied {
        return None;
    }
    protected_location(path, home).map(|location| PermissionFailure {
        location,
        path: path.to_path_buf(),
    })
}

/// Which gated location a path falls under, by path alone.
///
/// # The volume split
///
/// macOS mounts removable disks and network shares under `/Volumes/` alike, and nothing in the
/// path separates them. `/Network/` and `/net/` are unambiguous, so they are read as network
/// mounts; everything else under `/Volumes/` is read as removable. The two prompts sit next to
/// each other in the same System Settings pane, so the cost of the imprecision is that the user
/// may be pointed one row away from the switch they need — against a generic "permission denied",
/// which points at nothing at all.
fn protected_location(path: &Path, home: Option<&Path>) -> Option<ProtectedLocation> {
    if path.starts_with("/Network") || path.starts_with("/net") {
        return Some(ProtectedLocation::NetworkVolume);
    }
    if path.starts_with("/Volumes") {
        return Some(ProtectedLocation::RemovableVolume);
    }
    let home = home?;
    for (folder, location) in [
        ("Documents", ProtectedLocation::Documents),
        ("Desktop", ProtectedLocation::Desktop),
        ("Downloads", ProtectedLocation::Downloads),
    ] {
        if path.starts_with(home.join(folder)) {
            return Some(location);
        }
    }
    None
}

/// The user's home directory, or [`None`] when it cannot be determined.
///
/// `None` costs nothing here: the three home-relative locations simply stop matching and the error
/// stays generic, which is the same answer this module gives for any path it does not recognise.
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/Users/mac";

    fn home() -> Option<&'static Path> {
        Some(Path::new(HOME))
    }

    fn classify_denied(path: &str) -> Option<ProtectedLocation> {
        classify_on(
            io::ErrorKind::PermissionDenied,
            Path::new(path),
            home(),
            true,
        )
        .map(|f| f.location)
    }

    #[test]
    fn each_protected_location_is_recognised() {
        assert_eq!(
            classify_denied("/Users/mac/Documents/work/repo"),
            Some(ProtectedLocation::Documents)
        );
        assert_eq!(
            classify_denied("/Users/mac/Desktop/notes"),
            Some(ProtectedLocation::Desktop)
        );
        assert_eq!(
            classify_denied("/Users/mac/Downloads/micold"),
            Some(ProtectedLocation::Downloads)
        );
        assert_eq!(
            classify_denied("/Volumes/Backup/repo"),
            Some(ProtectedLocation::RemovableVolume)
        );
        assert_eq!(
            classify_denied("/Network/Servers/build/repo"),
            Some(ProtectedLocation::NetworkVolume)
        );
    }

    /// The folder itself, not only something inside it — a project opened at `~/Documents` is
    /// refused the same way as one two levels down.
    #[test]
    fn the_protected_folder_itself_counts() {
        assert_eq!(
            classify_denied("/Users/mac/Documents"),
            Some(ProtectedLocation::Documents)
        );
    }

    /// A prefix match on the *string* would attribute this to Documents. `Path::starts_with`
    /// compares components, which is why it is used.
    #[test]
    fn a_neighbouring_folder_with_a_shared_prefix_is_not_protected() {
        assert_eq!(classify_denied("/Users/mac/Documents-old/repo"), None);
        assert_eq!(classify_denied("/Users/mac/Desktops/repo"), None);
    }

    #[test]
    fn an_ordinary_path_is_left_alone() {
        assert_eq!(classify_denied("/Users/mac/code/repo"), None);
        assert_eq!(classify_denied("/opt/repo"), None);
        assert_eq!(classify_denied("/Users/other/Documents/repo"), None);
    }

    /// Over-attributing is the failure this guards against: a file whose mode is wrong is not a
    /// TCC problem, and sending the user to System Settings about it wastes more of their time
    /// than the generic message would.
    #[test]
    fn only_permission_denied_is_attributed() {
        for kind in [
            io::ErrorKind::NotFound,
            io::ErrorKind::Other,
            io::ErrorKind::InvalidInput,
        ] {
            assert_eq!(
                classify_on(kind, Path::new("/Users/mac/Documents/repo"), home(), true),
                None,
                "{kind:?} is not a permission problem"
            );
        }
    }

    /// Principle VI: no other platform's error text changes because macOS has TCC.
    #[test]
    fn no_other_platform_is_affected() {
        for path in [
            "/Users/mac/Documents/repo",
            "/Volumes/Backup/repo",
            "/Network/Servers/build",
        ] {
            assert_eq!(
                classify_on(
                    io::ErrorKind::PermissionDenied,
                    Path::new(path),
                    home(),
                    false,
                ),
                None,
                "{path} must not be attributed off macOS"
            );
        }
    }

    /// An unknown home directory degrades to the generic message rather than to a guess.
    #[test]
    fn without_a_home_directory_only_the_absolute_locations_match() {
        assert_eq!(
            classify_on(
                io::ErrorKind::PermissionDenied,
                Path::new("/Users/mac/Documents/repo"),
                None,
                true,
            ),
            None
        );
        assert_eq!(
            classify_on(
                io::ErrorKind::PermissionDenied,
                Path::new("/Volumes/Backup/repo"),
                None,
                true,
            )
            .map(|f| f.location),
            Some(ProtectedLocation::RemovableVolume)
        );
    }

    /// The message has to carry the three things the user needs: the path, the permission's own
    /// name, and where to grant it.
    #[test]
    fn the_message_names_the_permission_and_where_to_grant_it() {
        let failure = classify_on(
            io::ErrorKind::PermissionDenied,
            Path::new("/Users/mac/Documents/repo"),
            home(),
            true,
        )
        .expect("a denied Documents path is attributed");
        let message = failure.user_message();
        assert!(message.contains("/Users/mac/Documents/repo"), "{message}");
        assert!(message.contains("Documents Folder"), "{message}");
        assert!(message.contains("System Settings"), "{message}");
        assert!(message.contains("reopen"), "{message}");
    }

    /// `/net` is a mount point, not a prefix: a home folder called `network-drives` is not one.
    #[test]
    fn the_network_prefixes_are_whole_components() {
        assert_eq!(classify_denied("/networking/repo"), None);
        assert_eq!(classify_denied("/Networks/repo"), None);
    }
}
