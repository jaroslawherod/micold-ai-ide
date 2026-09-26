//! Whether a file is read or run (FR-013; contract link-opening §3, research R9).
//!
//! A file this machine would *run* is never handed to the system opener: it is revealed in the file
//! manager instead, so a link in terminal output can never execute anything. The rule depends on the
//! platform, so the platform is a parameter rather than a `cfg` — Linux CI exercises all three arms
//! (Principle VI).

/// Which platform's rules apply, as the shell reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostPlatform {
    Linux,
    MacOs,
    Windows,
}

/// Whether the path is a file or a folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    File,
    Dir,
}

/// What the shell found out about the file, in the same blocking task that opens it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileFacts {
    pub kind: Kind,
    /// Any of `0o111` is set. Always `false` on Windows, which has no such bit.
    pub any_exec_bit: bool,
    /// A macOS bundle directory holding `Contents/Info.plist`.
    pub is_bundle: bool,
}

/// What activating a file link does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileAction {
    /// Hand it to the system opener: a document opens in its application, a folder in the file
    /// manager.
    Open,
    /// Show it in the file manager, selected, without running it.
    Reveal,
}

/// Linux launchers and installers (FR-013, verbatim).
const LINUX_RUNNABLE: [&str; 9] = [
    "desktop",
    "appimage",
    "jar",
    "deb",
    "rpm",
    "snap",
    "flatpak",
    "flatpakref",
    "run",
];

/// macOS bundle directories (FR-013, verbatim).
const MACOS_BUNDLE: [&str; 8] = [
    "app",
    "bundle",
    "framework",
    "plugin",
    "kext",
    "prefpane",
    "appex",
    "xpc",
];

/// macOS runnable files (FR-013, verbatim).
const MACOS_RUNNABLE: [&str; 16] = [
    "app",
    "command",
    "terminal",
    "tool",
    "pkg",
    "mpkg",
    "jar",
    "workflow",
    "action",
    "scpt",
    "applescript",
    "webloc",
    "fileloc",
    "inetloc",
    "url",
    "shortcut",
];

/// Windows runnable files beyond `%PATHEXT%` (FR-013, verbatim).
const WINDOWS_RUNNABLE: [&str; 33] = [
    "exe",
    "com",
    "bat",
    "cmd",
    "ps1",
    "psm1",
    "vbs",
    "vbe",
    "js",
    "jse",
    "wsf",
    "wsh",
    "hta",
    "scr",
    "pif",
    "cpl",
    "msc",
    "msi",
    "msp",
    "reg",
    "lnk",
    "url",
    "jar",
    "appref-ms",
    "application",
    "appx",
    "msix",
    "chm",
    "inf",
    "scf",
    "settingcontent-ms",
    "library-ms",
    "search-ms",
];

/// What to do with the file `name`, given what the shell found out about it (FR-013).
///
/// `name` is the path's final component, and `pathext` the entries of `%PATHEXT%` (empty off
/// Windows). Only the last extension counts, and extensions compare ASCII case-insensitively.
pub fn action_for(
    platform: HostPlatform,
    name: &str,
    facts: FileFacts,
    pathext: &[String],
) -> FileAction {
    // Win32 drops trailing dots and spaces before it resolves a path, so `x.exe.` opens `x.exe`.
    let name = if platform == HostPlatform::Windows {
        name.trim_end_matches(['.', ' ', '\t'])
    } else {
        name
    };
    let ext = last_extension(name);
    let listed = |list: &[&str]| {
        ext.is_some_and(|ext| list.iter().any(|known| known.eq_ignore_ascii_case(ext)))
    };
    let is_file = facts.kind == Kind::File;
    let unix_bit = is_file && facts.any_exec_bit;
    // FR-013's Folders bullet: off macOS a folder opens in the file manager whatever it is called,
    // so an extension only speaks for a file. On macOS a folder with one of those extensions is a
    // bundle, which the Finder runs.
    let runs = match platform {
        HostPlatform::Linux => unix_bit || (is_file && listed(&LINUX_RUNNABLE)),
        HostPlatform::MacOs => {
            unix_bit
                || (facts.kind == Kind::Dir && (facts.is_bundle || listed(&MACOS_BUNDLE)))
                || listed(&MACOS_RUNNABLE)
        }
        // Windows has no execute bit: the extension decides, and `%PATHEXT%` is what this machine
        // itself says it runs.
        HostPlatform::Windows => {
            is_file
                && (listed(&WINDOWS_RUNNABLE)
                    || ext.is_some_and(|ext| {
                        pathext
                            .iter()
                            .any(|entry| entry.trim_start_matches('.').eq_ignore_ascii_case(ext))
                    }))
        }
    };
    if runs {
        FileAction::Reveal
    } else {
        FileAction::Open
    }
}

/// The last extension of `name`, or `None` when it has none — a dotfile such as `.bashrc` has none.
fn last_extension(name: &str) -> Option<&str> {
    let (stem, ext) = name.rsplit_once('.')?;
    (!stem.is_empty() && !ext.is_empty()).then_some(ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-013's lists, transcribed from the spec rather than read out of the code above, so a
    /// dropped entry fails here.
    const LINUX_LAUNCHERS: [&str; 9] = [
        "desktop",
        "appimage",
        "jar",
        "deb",
        "rpm",
        "snap",
        "flatpak",
        "flatpakref",
        "run",
    ];
    const MACOS_BUNDLES: [&str; 8] = [
        "app",
        "bundle",
        "framework",
        "plugin",
        "kext",
        "prefPane",
        "appex",
        "xpc",
    ];
    const MACOS_RUNNABLES: [&str; 16] = [
        "app",
        "command",
        "terminal",
        "tool",
        "pkg",
        "mpkg",
        "jar",
        "workflow",
        "action",
        "scpt",
        "applescript",
        "webloc",
        "fileloc",
        "inetloc",
        "url",
        "shortcut",
    ];
    const WINDOWS_RUNNABLES: [&str; 33] = [
        "exe",
        "com",
        "bat",
        "cmd",
        "ps1",
        "psm1",
        "vbs",
        "vbe",
        "js",
        "jse",
        "wsf",
        "wsh",
        "hta",
        "scr",
        "pif",
        "cpl",
        "msc",
        "msi",
        "msp",
        "reg",
        "lnk",
        "url",
        "jar",
        "appref-ms",
        "application",
        "appx",
        "msix",
        "chm",
        "inf",
        "scf",
        "settingcontent-ms",
        "library-ms",
        "search-ms",
    ];

    const PATHEXT: [&str; 4] = [".COM", ".EXE", ".BAT", ".MYAPP"];

    fn pathext() -> Vec<String> {
        PATHEXT.iter().map(|e| e.to_string()).collect()
    }

    fn file(any_exec_bit: bool) -> FileFacts {
        FileFacts {
            kind: Kind::File,
            any_exec_bit,
            is_bundle: false,
        }
    }

    fn dir(is_bundle: bool) -> FileFacts {
        FileFacts {
            kind: Kind::Dir,
            any_exec_bit: false,
            is_bundle,
        }
    }

    /// U50: the execute bit is what Unix means by runnable.
    #[test]
    fn on_linux_and_macos_the_execute_bit_reveals_the_file() {
        for platform in [HostPlatform::Linux, HostPlatform::MacOs] {
            assert_eq!(
                action_for(platform, "build", file(true), &[]),
                FileAction::Reveal,
                "{platform:?}: a file this machine can run is shown, never run (FR-013)"
            );
            assert_eq!(
                action_for(platform, "build", file(false), &[]),
                FileAction::Open,
                "{platform:?}: the same file without the bit is a document"
            );
        }
    }

    /// U51: Linux launchers and installers carry no execute bit of their own.
    #[test]
    fn on_linux_each_listed_launcher_extension_reveals() {
        for ext in LINUX_LAUNCHERS {
            let name = format!("thing.{ext}");
            assert_eq!(
                action_for(HostPlatform::Linux, &name, file(false), &[]),
                FileAction::Reveal,
                "{name} is launched or installed by the desktop, so it is shown (FR-013)"
            );
        }
        assert_eq!(
            action_for(HostPlatform::Linux, "Thing.AppImage", file(false), &[]),
            FileAction::Reveal,
            "an extension matches whatever its case"
        );
        assert_eq!(
            action_for(HostPlatform::Linux, "notes.txt", file(false), &[]),
            FileAction::Open,
            "a document opens in its application"
        );
    }

    /// U52: a macOS bundle is a directory the Finder runs.
    #[test]
    fn on_macos_a_bundle_directory_reveals_and_an_ordinary_folder_opens() {
        for ext in MACOS_BUNDLES {
            let name = format!("Thing.{ext}");
            assert_eq!(
                action_for(HostPlatform::MacOs, &name, dir(false), &pathext()),
                FileAction::Reveal,
                "{name} is a bundle by its extension (FR-013)"
            );
        }
        assert_eq!(
            action_for(HostPlatform::MacOs, "Thing", dir(true), &[]),
            FileAction::Reveal,
            "a directory holding Contents/Info.plist is a bundle whatever it is called"
        );
        assert_eq!(
            action_for(HostPlatform::MacOs, "Projects", dir(false), &[]),
            FileAction::Open,
            "an ordinary folder opens in the file manager"
        );
    }

    /// U53: macOS scripts, installers and link files.
    #[test]
    fn on_macos_each_listed_extension_reveals() {
        for ext in MACOS_RUNNABLES {
            let name = format!("thing.{ext}");
            assert_eq!(
                action_for(HostPlatform::MacOs, &name, file(false), &[]),
                FileAction::Reveal,
                "{name} is run or installed by macOS, so it is shown (FR-013)"
            );
        }
        assert_eq!(
            action_for(HostPlatform::MacOs, "report.pdf", file(false), &[]),
            FileAction::Open,
            "a document opens in its application"
        );
    }

    /// U54: on Windows the extension decides, and the execute bit means nothing.
    #[test]
    fn on_windows_the_extension_decides_and_the_execute_bit_is_ignored() {
        for ext in WINDOWS_RUNNABLES {
            let name = format!("thing.{ext}");
            assert_eq!(
                action_for(HostPlatform::Windows, &name, file(false), &[]),
                FileAction::Reveal,
                "{name} is run by Windows, so it is shown (FR-013)"
            );
        }
        assert_eq!(
            action_for(
                HostPlatform::Windows,
                "thing.myapp",
                file(false),
                &pathext()
            ),
            FileAction::Reveal,
            "a %PATHEXT% entry is what this machine itself says it runs"
        );
        assert_eq!(
            action_for(HostPlatform::Windows, "notes.txt", file(true), &pathext()),
            FileAction::Open,
            "a file carried over from a Unix filesystem with its execute bit set is still a \
             document on Windows"
        );
    }

    /// U55: `x.exe.txt` is a text file.
    #[test]
    fn only_the_last_extension_counts() {
        assert_eq!(
            action_for(HostPlatform::Windows, "x.exe.txt", file(false), &pathext()),
            FileAction::Open,
            "a name ending in .txt is a document, whatever lies before it"
        );
        assert_eq!(
            action_for(HostPlatform::Windows, "x.txt.exe", file(false), &pathext()),
            FileAction::Reveal,
            "a name ending in .exe is run, whatever lies before it"
        );
    }

    /// U55: Win32 drops trailing dots and spaces before it opens a path, so a name that ends in
    /// them names — and runs — the file without them (review A finding 1).
    #[test]
    fn on_windows_a_trailing_dot_or_space_never_hides_a_runnable_extension() {
        for name in ["x.exe.", "x.exe ", "x.exe. .", "x.myapp."] {
            assert_eq!(
                action_for(HostPlatform::Windows, name, file(false), &pathext()),
                FileAction::Reveal,
                "{name:?}: Windows opens the same file as x.exe, so it is shown, never run (FR-013)"
            );
        }
        assert_eq!(
            action_for(HostPlatform::Windows, "notes.txt.", file(false), &pathext()),
            FileAction::Open,
            "a document is still a document with a dot after it"
        );
    }

    /// U51, U54: FR-013's Folders bullet — off macOS a folder opens whatever it is called. On macOS
    /// a folder carrying a listed extension is a bundle, which U52 covers, so it stays runnable.
    #[test]
    fn a_folder_named_like_a_runnable_file_still_opens() {
        for (platform, name) in [
            (HostPlatform::Linux, "pkg.run"),
            (HostPlatform::Linux, "thing.desktop"),
            (HostPlatform::Windows, "thing.exe"),
            (HostPlatform::Windows, "thing.myapp"),
        ] {
            assert_eq!(
                action_for(platform, name, dir(false), &pathext()),
                FileAction::Open,
                "{platform:?} {name}: every folder but a macOS bundle opens in the file manager \
                 (FR-013, Folders)"
            );
        }
    }
}
