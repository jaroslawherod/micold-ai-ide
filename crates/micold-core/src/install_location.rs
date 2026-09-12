//! Where the running executable lives, as far as it concerns whether the app should run at all
//! (feature 028, FR-019).
//!
//! macOS delivers applications inside a disk image, and a `.dmg` mounts as a read-only volume that
//! looks, to the user, exactly like a folder with the app in it. Double-clicking the app there
//! launches it — and it half-works. It cannot be updated, it disappears when the image is ejected,
//! Gatekeeper's first-launch approval does not stick to it, and on the next boot the app the user
//! believes they installed is simply gone. None of that surfaces as an error; it surfaces weeks
//! later as a bug report about state that vanished.
//!
//! Quarantined copies have a second shape. Gatekeeper *translocates* an app launched from a
//! quarantined location: it runs from a randomised, read-only mount whose path contains
//! `AppTranslocation`. Same consequences, different symptom, and the user has no way to tell the
//! difference from the window.
//!
//! So the application asks once, at boot, and refuses to pretend. Anything but [`Installed`]
//! replaces the session UI with a screen saying what happened and what to do — it does not
//! continue, and it offers no dismissal, because a dismissible warning about a copy that is going
//! to disappear is a warning the user learns to click through.
//!
//! # Path only
//!
//! Classification is arithmetic over the path: no filesystem access, no macOS API, no `cfg` branch
//! in the rule itself. That is what lets the whole thing be unit-tested on Linux and Windows as
//! well as macOS (Constitution Principle VI), and it is why [`current`] can never fail — an
//! unreadable `current_exe()` answers [`InstallLocation::Installed`], because refusing to start on
//! an inconclusive answer would be a worse failure than the one being prevented.
//!
//! # The accepted false positive
//!
//! An application genuinely installed on an external disk mounted under `/Volumes/` is classified
//! as [`InstallLocation::MountedImage`] and asked to be installed properly. No path-only signal
//! separates a mounted disk image from any other mounted volume, and a clear, actionable false
//! positive beats an unclear false negative — the false negative here is silent data loss
//! (research R9).

use std::path::{Component, Path};

/// Where the executable is running from.
///
/// `Installed` is the default because it is the answer that lets the application run: a state
/// constructed before anything has asked the question must not be a state that refuses to start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallLocation {
    /// A normal, persistent location. Always the answer off macOS.
    #[default]
    Installed,
    /// Running from the delivery container the user has not installed from.
    MountedImage,
    /// macOS relocated a quarantined copy to a randomised read-only path.
    Translocated,
}

impl InstallLocation {
    /// Whether the application may run normally from here.
    pub fn is_installed(&self) -> bool {
        matches!(self, InstallLocation::Installed)
    }

    /// The one sentence explaining what the user is looking at.
    ///
    /// Two sentences rather than one shared one: the gestures differ. A mounted image is fixed by
    /// dragging the app across; a translocated copy is already somewhere the user cannot see, so
    /// the instruction has to start from the disk image they still have.
    pub fn explanation(&self) -> &'static str {
        match self {
            InstallLocation::Installed => "",
            InstallLocation::MountedImage => {
                "Micold AI IDE is running from the downloaded disk image, not from your \
                 Applications folder. Anything you do here disappears when the image is ejected."
            }
            InstallLocation::Translocated => {
                "macOS is running Micold AI IDE from a temporary, read-only copy, because the app \
                 was opened from where it was downloaded. Anything you do here disappears when the \
                 app is closed."
            }
        }
    }

    /// What to do about it.
    pub fn remedy(&self) -> &'static str {
        match self {
            InstallLocation::Installed => "",
            InstallLocation::MountedImage | InstallLocation::Translocated => {
                "Drag Micold AI IDE into the Applications folder, eject the disk image, and open \
                 the app from Applications."
            }
        }
    }
}

/// The path component macOS uses for a translocated mount.
const TRANSLOCATION: &str = "AppTranslocation";

/// The mount point every volume other than the startup disk appears under.
const VOLUMES: &str = "Volumes";

/// Classify where an executable is running from.
pub fn classify(executable: &Path) -> InstallLocation {
    classify_on(executable, cfg!(target_os = "macos"))
}

/// The boot path's question: `classify(current_exe())`, [`InstallLocation::Installed`] on error.
pub fn current() -> InstallLocation {
    std::env::current_exe()
        .map(|exe| classify(&exe))
        .unwrap_or(InstallLocation::Installed)
}

/// The rule, with the platform as an input rather than a `cfg` branch — so the macOS behaviour is
/// exercised by every leg of the matrix instead of one.
fn classify_on(executable: &Path, on_macos: bool) -> InstallLocation {
    if !on_macos {
        return InstallLocation::Installed;
    }
    // Translocation is checked first. A translocated copy of a quarantined app can carry either
    // shape, and its explanation is the more specific one: telling someone to eject a disk image
    // they may already have ejected explains nothing.
    if executable
        .components()
        .any(|c| c.as_os_str() == TRANSLOCATION)
    {
        return InstallLocation::Translocated;
    }
    // The contract says "a path beginning `/Volumes/`" -- with the trailing separator, and that
    // detail is the rule. `/Volumes` itself is the directory mount points are made in, not a
    // mounted volume, so a path that stops there names no volume and is nothing to refuse over.
    let mut components = executable.components();
    if components.next() == Some(Component::RootDir)
        && components.next().is_some_and(|c| c.as_os_str() == VOLUMES)
        && components.next().is_some()
    {
        return InstallLocation::MountedImage;
    }
    InstallLocation::Installed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on_mac(path: &str) -> InstallLocation {
        classify_on(Path::new(path), true)
    }

    #[test]
    fn an_installed_app_is_installed() {
        assert_eq!(
            on_mac("/Applications/micold-ai-ide.app/Contents/MacOS/micold-ai-ide"),
            InstallLocation::Installed
        );
        assert_eq!(
            on_mac("/Users/mac/Applications/micold-ai-ide.app/Contents/MacOS/micold-ai-ide"),
            InstallLocation::Installed
        );
    }

    #[test]
    fn a_copy_on_a_mounted_volume_is_a_mounted_image() {
        assert_eq!(
            on_mac("/Volumes/Micold AI IDE/micold-ai-ide.app/Contents/MacOS/micold-ai-ide"),
            InstallLocation::MountedImage
        );
    }

    #[test]
    fn a_translocated_copy_is_translocated() {
        assert_eq!(
            on_mac(
                "/private/var/folders/xy/AppTranslocation/1B2C-3D/d/micold-ai-ide.app/Contents/MacOS/micold-ai-ide"
            ),
            InstallLocation::Translocated
        );
    }

    /// Both shapes at once. Translocation wins, because its remedy is the one that helps.
    #[test]
    fn translocation_wins_over_a_mounted_volume() {
        assert_eq!(
            on_mac("/Volumes/x/AppTranslocation/d/micold-ai-ide.app/Contents/MacOS/micold-ai-ide"),
            InstallLocation::Translocated
        );
    }

    /// `Volumes` has to be the first component of an absolute path, not merely present in it —
    /// otherwise a project directory named `Volumes` would refuse to launch the app inside it.
    #[test]
    fn volumes_deeper_in_the_path_is_not_a_mounted_image() {
        assert_eq!(
            on_mac("/Users/mac/Volumes/micold-ai-ide.app/Contents/MacOS/micold-ai-ide"),
            InstallLocation::Installed
        );
        assert_eq!(
            on_mac("Volumes/micold-ai-ide.app/Contents/MacOS/micold-ai-ide"),
            InstallLocation::Installed
        );
        // A shared prefix is not the mount point either.
        assert_eq!(
            on_mac("/VolumesBackup/micold-ai-ide.app/Contents/MacOS/micold-ai-ide"),
            InstallLocation::Installed
        );
    }

    /// Total over anything, because `current_exe()` is not something this can afford to be
    /// surprised by.
    #[test]
    fn odd_paths_answer_installed_rather_than_refusing_to_start() {
        assert_eq!(on_mac(""), InstallLocation::Installed);
        assert_eq!(on_mac("micold-ai-ide"), InstallLocation::Installed);
        assert_eq!(on_mac("./micold-ai-ide"), InstallLocation::Installed);
        assert_eq!(on_mac("/"), InstallLocation::Installed);
        assert_eq!(on_mac("/Volumes"), InstallLocation::Installed);
    }

    /// Principle VI: no other platform grows a new way to refuse to start. `/Volumes` is an
    /// ordinary directory name on Linux, and `AppTranslocation` means nothing there.
    #[test]
    fn no_other_platform_is_affected() {
        for path in [
            "/Volumes/Micold AI IDE/micold-ai-ide",
            "/x/AppTranslocation/d/micold-ai-ide",
        ] {
            assert_eq!(
                classify_on(Path::new(path), false),
                InstallLocation::Installed,
                "{path} must not stop the app off macOS"
            );
        }
    }

    /// The refusal has to say both halves, or it is a dead end rather than an instruction.
    #[test]
    fn every_refusal_explains_itself_and_offers_a_remedy() {
        for location in [InstallLocation::MountedImage, InstallLocation::Translocated] {
            assert!(!location.is_installed(), "{location:?}");
            assert!(
                location.explanation().len() > 40,
                "{location:?} explains nothing"
            );
            assert!(
                location.remedy().contains("Applications"),
                "{location:?} does not say where the app belongs"
            );
        }
        assert!(InstallLocation::Installed.is_installed());
    }

    /// `current()` answers rather than failing, whatever the executable's path turns out to be.
    #[test]
    fn current_always_answers() {
        let here = current();
        assert!(
            matches!(
                here,
                InstallLocation::Installed
                    | InstallLocation::MountedImage
                    | InstallLocation::Translocated
            ),
            "{here:?}"
        );
        // And it agrees with classifying the same path by hand.
        if let Ok(exe) = std::env::current_exe() {
            assert_eq!(here, classify(exe.as_path()));
        }
    }
}
