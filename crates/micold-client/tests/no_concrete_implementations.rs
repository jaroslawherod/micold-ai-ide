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
//!
//! # The AI CLI providers are held to a different, wider rule (feature 026 — FR-022)
//!
//! Everything above scans the **client**. That was enough while every capability's real
//! implementation was chosen there. It is not enough for the provider seam, and the gap was not
//! marginal: of the seven places that named `ClaudeProvider`, **four were outside this crate** —
//! `micold-daemon/src/{catalog,supervisor}.rs` once each, `state.rs` twice, and
//! `micold-core/src/terminal.rs`. A guard that would not catch a `CodexProvider` wired into the
//! supervisor is not the guard FR-022 asks for.
//!
//! So [`no_provider_type_is_named_outside_cores_definition_site`] scans all three crates, and the
//! "one place" for a provider is **not** the shell. It is `micold-core/src/provider.rs`, where
//! `AiCli::provider` resolves a persisted name to an implementation. That location is forced
//! rather than preferred: `micold-daemon` depends on `micold-client` only as a dev-dependency and
//! `micold-core` cannot depend on it at all, while the daemon's catalog, state and supervisor and
//! the core's own `terminal.rs` all need the lookup.
//!
//! That module is an **explicitly listed exemption**, not one the scan happens not to reach. It is
//! where both types are *defined*, so it names them by necessity — and an exemption a test states
//! is a decision, while one it merely misses is a hole.
//!
//! # What this guard structurally cannot catch, and where that is covered instead
//!
//! A name-based guard finds names. The client's boot prune named nothing concrete — it took one
//! `&dyn AiCliProvider` from `Capabilities` and applied it to every session in every project — so
//! it passed this check while being the same defect, and would have dropped every Copilot session
//! at startup. Only a test that mixes providers can see that, which is why
//! `shell/persist.rs`'s own `boot_judges_each_session_by_its_own_provider` and the daemon's
//! `set_wide_provider_decisions.rs` exist. Recorded here so a reader does not mistake a green run
//! of this file for the whole of FR-022.

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

/// The AI CLI provider implementations, derived the same way everything else here is.
///
/// They are excluded from the client's FR-018 count and held to the wider rule below instead. The
/// derivation rather than a list, for the reason the module doc gives: a hardcoded list of real
/// implementations was already incomplete on the day it was written.
fn provider_implementations() -> BTreeSet<String> {
    inventory::port_impls_under(&inventory::core_src())
        .into_iter()
        .filter(|found| found.port == "AiCliProvider" && !found.is_fake())
        .map(|found| found.ty)
        .collect()
}

/// `micold-core/src/provider.rs` — where both provider types are defined and where `AiCli::provider`
/// resolves a name to one. The single listed exemption.
const PROVIDER_DEFINITION_SITE: &str = "provider.rs";

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

    // Feature 026's own vacuity guard. `no_provider_type_is_named_outside_cores_definition_site`
    // iterates this derivation, so a scan that stopped finding provider implementations would let
    // a `CodexProvider` be wired into the supervisor with every test still green.
    let providers = provider_implementations();
    for expected in ["ClaudeProvider", "CopilotProvider"] {
        assert!(
            providers.contains(expected),
            "the derivation did not find `{expected}`, so FR-022's guard below is vacuous.              Found: {providers:?}"
        );
    }
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
    let providers = provider_implementations();
    let mut wrong = Vec::new();

    // The providers are exempt from *this* count and not from the requirement: since feature 026
    // the client chooses no provider at all: every consumer resolves one from the session record
    // through `AiCli::provider`, so a "named exactly once in the shell" rule would demand a line
    // that no longer has any reason to exist. Their single site is asserted below, across all
    // three crates.
    for ty in real_implementations().difference(&providers) {
        let sites: Vec<String> = sources
            .iter()
            .filter(|(path, _)| is_shell(path))
            .filter_map(|(path, code)| match names(code, ty) {
                0 => None,
                n => Some(format!("{path}×{n}")),
            })
            .collect();
        let total: usize = sources
            .iter()
            .filter(|(path, _)| is_shell(path))
            .map(|(_, code)| names(code, ty))
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

// ---------------------------------------------------------------------------------------
// FR-022 (feature 026) — the provider rule, across all three crates
// ---------------------------------------------------------------------------------------

/// Every source file this guard is responsible for, keyed by a display path that names its crate.
///
/// Three crates rather than one, because that is where the problem was. `micold-core` is included
/// whole — not just `terminal.rs` — so a provider named from `store.rs` or `session.rs` is caught
/// too; the definition site is subtracted by name rather than by never being looked at.
fn workspace_sources() -> BTreeMap<String, String> {
    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/");
    let mut out = BTreeMap::new();
    for crate_name in ["micold-core", "micold-client", "micold-daemon"] {
        let src = crates_dir.join(crate_name).join("src");
        for (path, text) in inventory::sources_under(&src) {
            // `sources_under` strips this crate's `src/`, which only matches for the client, so
            // rebuild a path that names the crate either way.
            let relative = path.rsplit("src/").next().unwrap_or(&path).to_string();
            out.insert(
                format!("{crate_name}/src/{relative}"),
                inventory::code_only(&text),
            );
        }
    }
    out
}

#[test]
fn the_workspace_scan_reaches_the_files_the_old_one_missed() {
    // Vacuity again, and pointed at the exact gap: four of the seven pre-feature mentions lived in
    // these files, and a scan that silently resolved no path would make the guard below pass by
    // looking at nothing.
    let sources = workspace_sources();
    for expected in [
        "micold-core/src/provider.rs",
        "micold-core/src/terminal.rs",
        "micold-daemon/src/catalog.rs",
        "micold-daemon/src/state.rs",
        "micold-daemon/src/supervisor.rs",
        "micold-client/src/main.rs",
        "micold-client/src/shell/capabilities.rs",
    ] {
        assert!(
            sources.contains_key(expected),
            "the workspace scan did not reach `{expected}`; found {} files",
            sources.len()
        );
    }
}

#[test]
fn no_provider_type_is_named_outside_cores_definition_site() {
    // FR-022, SC-007. A third CLI must be a one-file change: add an implementation beside the
    // other two and an arm to `AiCli::provider`. It must NOT be possible to wire one in by naming
    // it in the supervisor, in the catalog, or in the launch path — which is exactly how the
    // second one would have been added before this feature.
    let sources = workspace_sources();
    let mut leaked = Vec::new();

    for ty in provider_implementations() {
        for (path, code) in &sources {
            if path.ends_with(&format!("micold-core/src/{PROVIDER_DEFINITION_SITE}")) {
                continue;
            }
            let count = names(code, &ty);
            if count > 0 {
                leaked.push(format!(
                    "`{ty}` is named {count}× in `{path}`. Since feature 026 a session records                      *which* CLI it runs and every consumer resolves that name through                      `AiCli::provider`; naming an implementation here re-decides it (FR-022)"
                ));
            }
        }
    }

    assert!(
        leaked.is_empty(),
        "a concrete AI CLI provider escaped core's definition site:\n  - {}",
        leaked.join("\n  - ")
    );
}

#[test]
fn the_definition_site_is_the_one_place_and_it_does_name_them() {
    // The other half of an exemption: it has to be load-bearing. If `provider.rs` stopped naming
    // the implementations — because the registry moved, or was replaced by something that resolves
    // a provider some other way — the exemption above would be silently protecting nothing, and
    // the test that says "named nowhere else" would be describing a workspace where they are named
    // nowhere at all.
    let sources = workspace_sources();
    let definition = sources
        .get(&format!("micold-core/src/{PROVIDER_DEFINITION_SITE}"))
        .expect("the definition site is in the scan");

    for ty in provider_implementations() {
        assert!(
            names(definition, &ty) > 0,
            "`{ty}` is not named at the definition site, so the exemption protects nothing"
        );
    }
}
