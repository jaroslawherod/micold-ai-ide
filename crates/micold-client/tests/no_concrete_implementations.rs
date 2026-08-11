//! Only the shell names a real implementation, and it chooses each one once (feature 021, T041 —
//! FR-017, FR-018).
//!
//! # Two properties, and they are not in the same state
//!
//! **FR-017 — non-shell code names no concrete implementation.** This holds today and this file is
//! a regression lock on it, in the sense T014a used: the property is true, eight extractions and a
//! shell split are about to move code around it, and a guard is what keeps it true across them.
//!
//! **FR-018 — each real implementation is chosen in exactly one place.** It does now. It did not
//! when this file was written: five real implementations were named from eleven places in
//! `main.rs` — `GitCli` twice, `JsonFileSettingsStore` three times, `StdFolderScanner` four — and
//! [`each_implementation_is_chosen_in_exactly_one_place`] was `#[ignore]`d at full strength rather
//! than weakened to fit them. T049 assembled them into `shell/capabilities.rs::Capabilities::real`
//! and the attribute came off unchanged.
//!
//! One capability is chosen outside that function and the reason is the GUI framework's:
//! `SystemThemeProbe` is constructed inside a subscription mapping closure, which iced forbids from
//! capturing anything. It is invisible to this guard regardless — the derivation below reads
//! `micold-core`, and that type is defined in the client — so the exception is recorded in
//! `shell/capabilities.rs` rather than enforced here.
//!
//! # What counts as a real implementation
//!
//! Derived, not listed: every type in `micold-core` with an `impl <Port> for <Type>` against one of
//! the seven service ports, minus the fakes. T041's own text names four — `GitCli`,
//! `JsonFileStore`, `JsonFileSettingsStore`, `StdFolderScanner` — and deriving finds a fifth,
//! `ClaudeProvider`, which the list omits. That is the argument for deriving in one line: a
//! hardcoded list was already incomplete on the day it was written.
//!
//! [`the_derivation_finds_the_implementations_this_task_names`] holds the derivation to finding at
//! least those four, so a scan that silently matched nothing cannot make the rest of this file
//! vacuous.
//!
//! # What counts as "naming" one
//!
//! An occurrence of the type's name in code, outside a `use` line. For these types every such
//! occurrence today *is* a construction (`GitCli::new()`, `JsonFileStore::default_location()`,
//! `let provider = ClaudeProvider;`), and stating it as "names it" rather than "constructs it"
//! makes the rule both simpler and stricter: FR-018 asks that one place decide which implementation
//! is used, and a file that merely mentions one has already made that decision.
//!
//! Comments are exempt, through the shared `code_only` — `main.rs`'s module doc mentions `GitCli`
//! when explaining what the shell is for, and prose describing the boundary is not a breach of it.

mod inventory;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The four T041 names explicitly. The derivation must find at least these.
const NAMED_BY_THE_TASK: &[&str] = &[
    "GitCli",
    "JsonFileStore",
    "JsonFileSettingsStore",
    "StdFolderScanner",
];

/// The shell: the one part of the client allowed to choose a real implementation.
///
/// `shell/` does not exist until T050 splits `main.rs` into it. Listed now so this guard keeps
/// answering the same question after the split instead of needing an edit at the moment it matters
/// most — a guard that has to be relaxed to let a refactor through is not holding anything.
fn is_shell(path: &str) -> bool {
    path == "main.rs" || path.starts_with("shell/")
}

/// The real implementations: every port implementation in the core that is not a fake.
///
/// The scan itself lives in `inventory` (T042 moved it there) because the fake-coverage guard
/// needs the same derivation, and two answers to "what implements a capability" is the drift
/// FR-014 objects to.
fn real_implementations() -> BTreeSet<String> {
    inventory::port_impls_under(&inventory::core_src())
        .into_iter()
        .filter(|found| !found.is_fake())
        .map(|found| found.ty)
        .collect()
}

/// Client sources, keyed by a path relative to `src/`, with comments stripped.
fn client_sources() -> BTreeMap<String, String> {
    inventory::sources_under(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
        .into_iter()
        .map(|(path, text)| (path, inventory::code_only(&text)))
        .collect()
}

/// How many times `code` names `ty` outside a `use` line.
fn names(code: &str, ty: &str) -> usize {
    code.lines()
        .filter(|line| !line.trim_start().starts_with("use "))
        .map(|line| line.match_indices(ty).count())
        .sum()
}

#[test]
fn the_derivation_finds_the_implementations_this_task_names() {
    // The vacuity guard. Every test below iterates the derivation, so a scan that matched nothing —
    // because an `impl` was reformatted, a port renamed, or the core moved — would pass them all
    // without looking at anything.
    let real = real_implementations();

    for expected in NAMED_BY_THE_TASK {
        assert!(
            real.contains(*expected),
            "the derivation did not find `{expected}`, which T041 names explicitly. The scan for \
             `impl <Port> for <Type>` has drifted from the source it reads, and every other test \
             in this file is now vacuous. Found: {real:?}"
        );
    }
    assert!(
        real.len() >= NAMED_BY_THE_TASK.len(),
        "fewer real implementations than the four T041 names: {real:?}"
    );
}

#[test]
fn the_only_excluded_implementations_are_fakes() {
    // `implementations` excludes by name prefix, so the prefix has to mean what it says. A real
    // implementation called `FakeSomething` would be waved through every test here.
    let excluded: BTreeSet<String> = inventory::port_impls_under(&inventory::core_src())
        .into_iter()
        .filter(inventory::PortImpl::is_fake)
        .map(|found| found.ty)
        .collect();
    let known: BTreeSet<String> = [
        "FakeGit",
        "FakeHandle",
        "FakeTerminalBackend",
        // T046's, added in the commit it appeared in — which is what this list is for.
        "FakeEnvIncludeResolver",
        // T047's.
        "FakeOsThemeProbe",
        // T048's four, which is what that task is: the remaining ports gaining a shared fake so
        // no test has to hand-roll one.
        "FakeFolderScanner",
        "FakeProjectStore",
        "FakeSettingsStore",
        "FakeAiCliProvider",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let unexpected: Vec<_> = excluded.difference(&known).collect();
    assert!(
        unexpected.is_empty(),
        "a new `Fake`-prefixed port implementation appeared: {unexpected:?}\n\nT048 adds fakes for \
         the remaining ports, and each one is expected — add it here so the exclusion stays a list \
         somebody agreed to rather than a prefix nobody checks."
    );
}

#[test]
fn no_code_outside_the_shell_names_a_real_implementation() {
    // FR-017. Holds today; this keeps it holding while Tier 3 and the shell split move code past
    // it. A feature module that reaches for `GitCli` has stopped being testable without a git
    // repository, and nothing else in the suite would say so.
    let sources = client_sources();
    let mut leaked = Vec::new();

    for ty in real_implementations() {
        for (path, code) in &sources {
            if is_shell(path) {
                continue;
            }
            let count = names(code, &ty);
            if count > 0 {
                leaked.push(format!(
                    "`{ty}` is named {count}× in `{path}`, which is not the shell — only the shell \
                     chooses a real implementation (FR-017)"
                ));
            }
        }
    }

    assert!(
        leaked.is_empty(),
        "a real implementation escaped the shell:\n  - {}",
        leaked.join("\n  - ")
    );
}

#[test]
fn each_implementation_is_chosen_in_exactly_one_place() {
    // FR-018. Was `#[ignore]`d at full strength while eleven sites chose implementations; T049
    // assembled them in `shell/capabilities.rs`, so it now passes as written rather than having
    // been softened to fit. What it holds from here is that the assembly point stays single.
    let sources = client_sources();
    let mut wrong = Vec::new();

    for ty in real_implementations() {
        let sites: Vec<String> = sources
            .iter()
            .filter(|(path, _)| is_shell(path))
            .filter_map(|(path, code)| match names(code, &ty) {
                0 => None,
                n => Some(format!("{path}×{n}")),
            })
            .collect();
        let total: usize = sources
            .iter()
            .filter(|(path, _)| is_shell(path))
            .map(|(_, code)| names(code, &ty))
            .sum();

        if total != 1 {
            wrong.push(format!("`{ty}` is chosen in {total} places: {sites:?}"));
        }
    }

    assert!(
        wrong.is_empty(),
        "the shell chooses some implementation more than once, so there is no single assembly \
         point (FR-018):\n  - {}",
        wrong.join("\n  - ")
    );
}
