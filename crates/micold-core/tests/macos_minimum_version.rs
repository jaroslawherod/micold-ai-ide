//! The macOS version floor is declared where the operating system can enforce it (feature 028,
//! FR-004).
//!
//! FR-004 requires the floor to be enforced by the system rather than left to the user to check.
//! `LSMinimumSystemVersion` is that enforcement: LaunchServices refuses to open the bundle below
//! it and says why. A missing or placeholder value silently removes the floor — the app launches
//! on an unsupported system and fails later, somewhere unrelated.
//!
//! One half asserts the declaration exists and carries a real version. The other asserts it equals
//! the floor stated in `docs/user-guide/install-macos.md`.
//!
//! # Why the documentation is held to the plist
//!
//! The floor moves. Apple ships a major release every autumn, the project supports the two most
//! recent, and the value therefore changes on a schedule — which is exactly the kind of value that
//! gets updated in one place. The plist is what the operating system enforces, so a stale *document*
//! is the silent half of that drift: it tells a user their Mac is supported while LaunchServices
//! refuses to open the application, and nothing in the build notices.
//!
//! So the value is recorded once, in `packaging/macos/Info.plist.in`, and the page has to agree.
//! Moving the floor is then two edits and a green suite rather than two edits and a hope.

use std::fs;
use std::path::{Path, PathBuf};

const KEY: &str = "LSMinimumSystemVersion";

/// The page that has to agree with the plist.
const GUIDE: &str = "docs/user-guide/install-macos.md";

/// The sentence in that page the floor is read from. Spelled out rather than inferred from any
/// "macOS <number>" in the prose: the page names other versions in passing — the release that
/// removed the old right-click shortcut, the last Intel release — and a scan loose enough to find
/// the floor would find those too.
const MARKER: &str = "**Minimum macOS version:";

fn repo_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn plist_path() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("packaging/macos/Info.plist.in")
}

/// The declared floor, exactly as it appears in the template.
pub fn declared_floor() -> Option<String> {
    let path = plist_path();
    let source =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let after = source.split_once(&format!("<key>{KEY}</key>"))?.1;
    let open = after.find("<string>")?;
    let rest = &after[open + "<string>".len()..];
    let close = rest.find("</string>")?;
    Some(rest[..close].trim().to_string())
}

#[test]
fn the_floor_is_declared() {
    let floor = declared_floor().unwrap_or_else(|| {
        panic!(
            "{} declares no {KEY}. Without it macOS applies no floor at all: the app opens on a \
             system it does not support and fails somewhere the user cannot connect to a version \
             requirement.",
            plist_path().display(),
        )
    });
    assert!(
        !floor.is_empty(),
        "{KEY} is empty in {}; an empty string is not a floor",
        plist_path().display(),
    );
}

#[test]
fn the_floor_is_a_version_and_not_a_placeholder() {
    let Some(floor) = declared_floor() else {
        return; // the test above owns absence
    };
    assert!(
        !floor.contains('@'),
        "{KEY} is {floor:?} — an unsubstituted template token, not a version",
    );
    let mut parts = floor.split('.');
    let major = parts.next().unwrap_or_default();
    assert!(
        major.parse::<u32>().is_ok_and(|m| m >= 10),
        "{KEY} is {floor:?}, whose major component is not a macOS release number. macOS versions \
         since 10.x are `<major>` or `<major>.<minor>`; anything else is a placeholder that \
         LaunchServices will not enforce as intended.",
    );
    assert!(
        parts.all(|p| p.parse::<u32>().is_ok()),
        "{KEY} is {floor:?}, which is not a dotted version",
    );
}

/// The floor as the user guide states it, from the marker sentence.
fn documented_floor() -> Option<String> {
    let path = repo_root().join(GUIDE);
    let source =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let after = source.split_once(MARKER)?.1;
    let end = after.find("**")?;
    Some(after[..end].trim().to_string())
}

#[test]
fn the_documented_floor_is_the_declared_one() {
    let declared = declared_floor().unwrap_or_else(|| {
        panic!("no {KEY} in the plist; `the_floor_is_declared` owns that failure")
    });
    let documented = documented_floor().unwrap_or_else(|| {
        panic!(
            "{GUIDE} does not state the floor. It must carry a sentence beginning \
             `{MARKER} <version>**`, so a reader and this test read the same number. \
             The plist declares {declared:?}."
        )
    });

    assert_eq!(
        documented, declared,
        "the version floor has drifted: {GUIDE} says {documented:?}, \
         packaging/macos/Info.plist.in declares {declared:?}.\n\n\
         The plist is what LaunchServices enforces, so the page is the half that is wrong for a \
         user: it tells them their Mac is supported while the application refuses to open. Moving \
         the floor means editing both — see docs/development/macos-packaging.md."
    );
}

#[test]
fn the_marker_appears_once() {
    // Two of them and the test reads the first while a reader reads whichever they reach.
    let path = repo_root().join(GUIDE);
    let source =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert_eq!(
        source.matches(MARKER).count(),
        1,
        "{GUIDE} states the version floor more than once; there is one floor and it is stated in \
         one place"
    );
}
