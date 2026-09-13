//! The bundle producer verifies its own signature, and never asks Gatekeeper's opinion (feature
//! 028, FR-015, research R6).
//!
//! Two rules, and the second is the surprising one.
//!
//! **It must verify.** `codesign --verify --strict` is what catches a bundle whose contents were
//! modified after signing — the exact failure a packaging change introduces, and one that surfaces
//! to a user as "the application is damaged" rather than as anything nameable. `codesign -dvv`
//! reporting `Signature=adhoc` is the second half: it pins *which* signature was applied, so a
//! future change to a real Developer ID identity is a deliberate edit here rather than a silent
//! one.
//!
//! **It must not assess.** `spctl --assess` is expected to *reject* an ad-hoc-signed app. That
//! rejection is not a defect — it is the documented first-launch block the user is told about
//! (FR-013). Gating on it in either direction is wrong: asserting it passes fails every build
//! today, and asserting it fails would fail the build on the day the project adopts notarization,
//! which is the one day nothing should break. `spctl` stays a diagnostic in the developer
//! documentation, never a gate.
//!
//! # Why text, not execution
//!
//! `codesign` and `spctl` exist only on macOS, so the rule has to be checked against the script's
//! text for the check itself to run on every platform (Principle VI). What is being asserted is a
//! property of the *definition*, in the same way `packaging_excludes_showcase.rs` asserts one of
//! the Debian manifest.

use std::fs;
use std::path::{Path, PathBuf};

fn script_path() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("scripts/macos-bundle.sh")
}

fn script() -> String {
    let path = script_path();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Lines that are not entirely a comment, so a rule can be stated in prose without satisfying or
/// violating itself.
fn code_lines(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect()
}

#[test]
fn the_signature_is_verified_strictly() {
    let source = script();
    let code = code_lines(&source);
    assert!(
        code.iter()
            .any(|l| l.contains("codesign") && l.contains("--verify") && l.contains("--strict")),
        "{} never runs `codesign --verify --strict`. A bundle signed and then modified stays \
         signed as far as the file system is concerned; the user meets it as \"the application is \
         damaged and can't be opened\", with nothing naming the packaging step that did it.",
        script_path().display(),
    );
}

#[test]
fn the_signature_is_asserted_to_be_ad_hoc() {
    let source = script();
    let code = code_lines(&source);
    assert!(
        code.iter().any(|l| l.contains("Signature=adhoc")),
        "{} does not check that the applied signature is `Signature=adhoc`. Verification alone \
         says the bundle is intact, not what vouches for it — pinning the kind is what makes \
         adopting a Developer ID identity a visible edit rather than a silent one.",
        script_path().display(),
    );
}

#[test]
fn gatekeeper_is_never_a_gate() {
    let source = script();
    let offenders: Vec<_> = code_lines(&source)
        .into_iter()
        .filter(|l| l.contains("spctl"))
        .collect();
    assert!(
        offenders.is_empty(),
        "{} runs `spctl`:\n  {}\n\
         Gatekeeper rejects an ad-hoc-signed app, and that rejection is the documented \
         first-launch block, not a build failure. Asserting it passes fails every build today; \
         asserting it fails breaks on the day this project adopts notarization. `spctl` belongs \
         in docs/development/macos-packaging.md as a diagnostic.",
        script_path().display(),
        offenders.join("\n  "),
    );
}
