//! Every capability has a fake, something exercises it, and none is wider than its consumers
//! (feature 021, T042 — FR-016, FR-019, SC-005).
//!
//! # Three obligations, two of them not yet met
//!
//! **FR-019 — every capability has a fake, in the core, beside the capability it satisfies.** Three
//! of seven do (`FakeGit`, `FakeTerminalBackend`, `FakeHandle`). T048 adds the rest; until then
//! [`every_capability_has_a_fake_in_the_core`] is `#[ignore]`d rather than softened, so the
//! requirement is written at the strength it should pass at and `cargo test` reports it outstanding
//! on every run. Same arrangement as T041's FR-018 clause, and for the same reason.
//!
//! **SC-005 — at least one test exercises real behaviour through each fake.** A fake nothing uses
//! is not evidence that a capability is fakeable; it is an unused type that happens to compile.
//! Checked for the fakes that exist, so this one passes today and keeps passing as T048 adds more.
//!
//! **FR-016 — narrowness.** The contract states the test exactly: *"if a test must implement a
//! method it does not exercise merely to satisfy the trait, the capability is too wide and must be
//! split"* (`contracts/service-capabilities.md`). That is what
//! [`no_test_is_forced_to_supply_an_operation_it_does_not_use`] checks, and it fails today.
//!
//! # How "forced to supply, does not exercise" is detected
//!
//! A method in a test's own port implementation whose parameters are **all** `_`-prefixed and whose
//! body never mentions `self`. Such a method cannot do anything with what it is given and cannot
//! consult what it is: it returns a constant. That is not a fake behaviour, it is a signature being
//! satisfied.
//!
//! The distinction is visible in one file. `tests/support/mod.rs`'s `FakeScanner` implements
//! `is_git_repo` and `is_available` by reading `self.git` / `self.available` — configurable, so a
//! test can drive them — and `list_subdirs` as `Ok(vec![])`, which no test can influence or observe.
//! The first two are exercised; the third is the trait asking for a method nobody wanted.
//!
//! Scoped to implementations written **in tests**, which is what the contract's wording is about. A
//! shared fake in the core implementing everything once is the relief FR-019 exists to provide, not
//! a violation of FR-016 — nobody is forced to write it a second time.
//!
//! # A known blind spot
//!
//! The scan is line-based, so an `impl` header rustfmt has wrapped across lines
//! (`impl Port` / `    for Type`) is invisible to it. That is not hypothetical — it is what a long
//! enough type name produces. [`the_scan_finds_the_implementations_that_exist`] is what notices:
//! the count drops and the guard fails rather than the narrowness check quietly passing on a
//! shorter list. Verified by wrapping one, which took the count from six to five and failed.
//!
//! # What a failure means, and what it does not license
//!
//! T042's own text: *"A capability failing the narrowness check MUST be split rather than the check
//! relaxed."* The failures below are therefore findings about `ProjectStore` and `FolderScanner`,
//! for T046/T048 to answer — not a reason to loosen the predicate.

mod inventory;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Every crate's `tests/` directory, plus the client `src/` (inline `#[cfg(test)]` modules live
/// there too).
fn test_roots() -> Vec<PathBuf> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/");
    ["micold-core", "micold-client", "micold-daemon"]
        .iter()
        .map(|c| crates.join(c).join("tests"))
        .filter(|p| p.is_dir())
        .collect()
}

/// Port implementations written inside tests — the ones the narrowness rule is about.
fn test_port_impls() -> Vec<inventory::PortImpl> {
    test_roots()
        .iter()
        .flat_map(|root| inventory::port_impls_under(root))
        .collect()
}

/// Port implementations in the core: the shared reals and the shared fakes.
fn core_port_impls() -> Vec<inventory::PortImpl> {
    inventory::port_impls_under(&inventory::core_src())
}

/// The source of every test root, keyed by display path, comments stripped.
fn test_sources() -> BTreeMap<String, String> {
    let mut all = BTreeMap::new();
    for root in test_roots() {
        for (path, text) in inventory::sources_under(&root) {
            all.insert(path, inventory::code_only(&text));
        }
    }
    all
}

/// A path a reader can act on: repo-relative, not the machine's absolute one.
///
/// `sources_under` keys relative to the *client's* `src`, which is right for the guards that only
/// read the client and leaves the other crates' paths absolute. A failure message naming
/// `/home/…/crates/micold-daemon/tests/x.rs` is noise around the four characters that matter.
fn shown(path: &str) -> &str {
    path.find("crates/").map_or(path, |at| &path[at..])
}

/// The text inside the first `open`…`close` pair of `s`, matched by depth.
fn balanced(s: &str, open: char, close: char) -> String {
    let mut depth = 0usize;
    let mut out = String::new();
    for c in s.chars() {
        if c == open {
            depth += 1;
            if depth == 1 {
                continue;
            }
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        if depth >= 1 {
            out.push(c);
        }
    }
    out
}

/// A method inside a port `impl` block: its name, and whether it is a stub.
#[derive(Debug)]
struct Method {
    name: String,
    /// Ignores every parameter it is given *and* never consults `self` — so it returns a constant.
    stub: bool,
}

/// The methods of the `impl <port> for <ty>` block in `code`, if it has one.
///
/// A brace-depth scan rather than a regex: a method body contains braces, and stopping at the first
/// `}` would read one method and call it the whole block.
fn methods_of(code: &str, port: &str, ty: &str) -> Vec<Method> {
    let Some(at) = code.find(&format!("impl {port} for {ty}")).or_else(|| {
        code.find(&format!(" {port} for {ty}"))
            .and_then(|i| code[..i].rfind("impl "))
    }) else {
        return Vec::new();
    };
    let body = &code[at..];
    let Some(open) = body.find('{') else {
        return Vec::new();
    };

    let mut depth = 0usize;
    let mut end = body.len();
    for (i, c) in body.char_indices().skip(open) {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let block = &body[open..end];

    let mut methods = Vec::new();
    let mut rest = block;
    while let Some(at) = rest.find("fn ") {
        let after = &rest[at + 3..];
        let Some(paren) = after.find('(') else { break };
        let name = after[..paren].trim().to_string();

        // The parameter list, by paren depth rather than by the first `)`. A parameter typed
        // `(u16, u16)` closes a paren that is not the signature's, and cutting there would drop
        // every parameter after it — turning "one argument is used" into "all are ignored".
        let params = balanced(&after[paren..], '(', ')');

        let Some(sig_end) = after.find('{') else {
            break;
        };

        let tail = &after[sig_end..];
        let mut d = 0usize;
        let mut body_end = tail.len();
        for (i, c) in tail.char_indices() {
            match c {
                '{' => d += 1,
                '}' => {
                    d -= 1;
                    if d == 0 {
                        body_end = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        let method_body = &tail[..body_end];

        let named_params: Vec<&str> = params
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty() && !p.contains("self"))
            .collect();
        let ignores_all_inputs = named_params.iter().all(|p| p.starts_with('_'));
        let consults_self = method_body.contains("self");

        methods.push(Method {
            name,
            stub: ignores_all_inputs && !consults_self,
        });
        rest = &tail[body_end..];
    }
    methods
}

#[test]
fn the_scan_finds_the_implementations_that_exist() {
    // The vacuity guard. Every test here iterates a scan of source text, so a scan that matched
    // nothing — because an `impl` was reformatted or a directory moved — would pass them all
    // without looking at anything.
    let core = core_port_impls();
    let in_tests = test_port_impls();

    assert!(
        core.iter().any(|i| i.ty == "GitCli"),
        "the core scan lost `GitCli`; every test in this file is now vacuous. Found: {core:?}"
    );
    assert!(
        core.iter().any(|i| i.ty == "FakeGit"),
        "the core scan lost `FakeGit`, the fake the other six are meant to match: {core:?}"
    );
    let seen: Vec<String> = in_tests
        .iter()
        .map(|i| format!("{}:{} for {}", shown(&i.file), i.port, i.ty))
        .collect();
    assert!(
        in_tests.len() >= 6,
        "the tests scan found {} port implementations; there were six when T042 was written, and a \
         scan that finds fewer makes the narrowness check pass on a short list.\n\nThe known \
         blind spot is a line-wrapped `impl` header (`impl Port\\n    for Type`), which the \
         line-based scan cannot see — this count is what notices.\n  - {}",
        in_tests.len(),
        seen.join("\n  - ")
    );
}

#[test]
#[ignore = "FR-019 is not met until T048 adds the missing fakes; run with --ignored to see which"]
fn every_capability_has_a_fake_in_the_core() {
    // FR-019. Written at full strength and ignored rather than reduced to the three that pass:
    // a guard that only asks about the capabilities already satisfied is not asking anything.
    let impls = core_port_impls();
    let mut missing = Vec::new();

    for port in inventory::PORTS {
        let fakes: Vec<&str> = impls
            .iter()
            .filter(|i| i.port == *port && i.is_fake())
            .map(|i| i.ty.as_str())
            .collect();
        if fakes.is_empty() {
            missing.push(*port);
        }
    }

    assert!(
        missing.is_empty(),
        "these capabilities have no fake in the core, so a consumer cannot be tested without the \
         real thing (FR-019): {missing:?}\n\nT048 adds them beside the capability they satisfy, as \
         ordinary public items matching `FakeGit`."
    );
}

#[test]
fn every_fake_that_exists_is_exercised_by_a_test() {
    // SC-005's second half. A fake nothing constructs is not evidence that a capability is
    // fakeable — it is an unused type that happens to compile. Scoped to the fakes that exist, so
    // this passes today and keeps its grip as T048 adds the other four.
    let fakes: BTreeSet<String> = core_port_impls()
        .into_iter()
        .filter(inventory::PortImpl::is_fake)
        .map(|i| i.ty)
        .collect();
    let sources = test_sources();
    let mut unused = Vec::new();

    for fake in &fakes {
        let used = sources.values().any(|code| code.contains(fake.as_str()));
        if !used {
            unused.push(fake.clone());
        }
    }

    assert!(
        unused.is_empty(),
        "these fakes exist but no test names them, so nothing exercises the capability through \
         them (SC-005): {unused:?}"
    );
    assert!(
        !fakes.is_empty(),
        "no fakes found at all — the scan has drifted"
    );
}

#[test]
#[ignore = "FR-016 narrowness fails today; the fix is to split the capability, not this check — \
            run with --ignored to see which methods are being supplied unexercised"]
fn no_test_is_forced_to_supply_an_operation_it_does_not_use() {
    // FR-016, in the contract's own words: "if a test must implement a method it does not exercise
    // merely to satisfy the trait, the capability is too wide and must be split."
    //
    // Ignored, not relaxed. T042's text is explicit that a capability failing this must be split,
    // so softening the predicate would be answering the wrong question.
    let sources = test_sources();
    let mut forced = Vec::new();

    for found in test_port_impls() {
        let Some(code) = sources.get(&found.file) else {
            continue;
        };
        for method in methods_of(code, &found.port, &found.ty) {
            if method.stub {
                forced.push(format!(
                    "`{}` in `{}` supplies `{}::{}` unexercised — it ignores every argument and \
                     never consults `self`, so it can only return a constant",
                    found.ty,
                    shown(&found.file),
                    found.port,
                    method.name
                ));
            }
        }
    }

    assert!(
        forced.is_empty(),
        "a capability is wider than its consumers, so a test had to supply an operation it does \
         not exercise (FR-016). Split the capability; do not relax this check:\n  - {}",
        forced.join("\n  - ")
    );
}
