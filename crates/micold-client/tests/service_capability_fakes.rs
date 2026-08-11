//! Every capability has a fake, something exercises it, and none is wider than its consumers
//! (feature 021, T042 — FR-016, FR-019, SC-005).
//!
//! # Three obligations, all met as of T048
//!
//! **FR-019 — every capability has a fake, in the core, beside the capability it satisfies.** Three
//! of seven did when T042 was written; T048 added the other four and split a fifth capability out,
//! so all of them do. The `#[ignore]` came off with the last missing fake.
//!
//! **SC-005 — at least one test exercises real behaviour through each fake.** A fake nothing uses
//! is not evidence that a capability is fakeable; it is an unused type that happens to compile.
//!
//! **FR-016 — narrowness.** The contract states the test exactly: *"if a test must implement a
//! method it does not exercise merely to satisfy the trait, the capability is too wide and must be
//! split"* (`contracts/service-capabilities.md`). That is what
//! [`no_test_is_forced_to_supply_an_operation_it_does_not_use`] checks. It failed with six
//! violations when written; it passes now, and how it came to pass is the next section.
//!
//! # How "forced to supply, does not exercise" is detected
//!
//! A method in a test's own port implementation whose parameters are **all** `_`-prefixed and whose
//! body never mentions `self`. Such a method cannot do anything with what it is given and cannot
//! consult what it is: it returns a constant. That is not a fake behaviour, it is a signature being
//! satisfied.
//!
//! Scoped to implementations written **in tests**, which is what the contract's wording is about. A
//! shared fake in the core implementing everything once is the relief FR-019 exists to provide, not
//! a violation of FR-016 — nobody is forced to write it a second time.
//!
//! # How the six violations went away, and why only one was a real width problem
//!
//! T042's text: *"A capability failing the narrowness check MUST be split rather than the check
//! relaxed."* T048 answered both findings, and they turned out to be different problems.
//!
//! `FolderScanner` **was** too wide, in production and not just in tests: every consumer that took
//! it as a trait asked only `is_git_repo` and `is_available`, while the sole caller of
//! `list_subdirs` reached for `StdFolderScanner` concretely. It was split (`FolderBrowser`).
//!
//! `ProjectStore` was **not**. Its only trait consumer, the daemon's `Catalog`, exercises all three
//! operations. The three `FakeStore`s that had to stub `save` were hand-written because no shared
//! fake existed — the coverage gap, showing up as a width symptom. Deleting them in favour of
//! `FakeProjectStore` is what fixed it, and splitting a capability its consumer genuinely uses
//! would have made the codebase worse to score a check green.
//!
//! # The vacuity guard had to change, because T048 emptied the thing it counted
//!
//! [`the_scan_finds_the_implementations_that_exist`] originally required at least six port
//! implementations under `tests/`, so a scan blinded by a line-wrapped `impl` header
//! (`impl Port` / `    for Type` — the known blind spot, and what a long enough type name
//! produces) would fail loudly instead of narrowing the list. There are now **zero**: every
//! hand-rolled fake is gone, which is the point of T048. A count of violations-era implementations
//! cannot guard a check whose correct answer is an empty list, so the guard moved to where it
//! cannot go stale — [`the_detector_can_tell_a_stub_from_a_behaviour`] runs `methods_of` over a
//! source with one of each and asserts it classifies both. That is a stronger guard than the
//! count: it fails if the parser breaks, not merely if the corpus shrinks.

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

/// Where the `impl <port> for <ty>` block begins, tolerating a fully-qualified port path.
///
/// Reads the header the same way [`inventory::port_impls_under`] does — take what sits between
/// `impl` and `for`, keep the last `::` segment — because the two disagreeing is a hole rather than
/// a difference. It was one: `port_impls_under` reported
/// `impl micold_core::fs_scan::FolderScanner for X` as a `FolderScanner` implementation, while a
/// reconstructed `impl FolderScanner for X` needle found nothing, so the narrowness check silently
/// examined no methods and passed. Found by probing T048 with exactly that fake.
fn impl_block_start(code: &str, port: &str, ty: &str) -> Option<usize> {
    for (at, _) in code.match_indices(&format!(" for {ty}")) {
        let Some(head_start) = code[..at].rfind("impl ") else {
            continue;
        };
        let head = &code[head_start + "impl ".len()..at];
        // A `{` in between means the `impl` found is an earlier block, not this header's.
        if head.contains('{') {
            continue;
        }
        if head.rsplit("::").next().unwrap_or(head).trim() == port {
            return Some(head_start);
        }
    }
    None
}

/// The methods of the `impl <port> for <ty>` block in `code`, if it has one.
///
/// A brace-depth scan rather than a regex: a method body contains braces, and stopping at the first
/// `}` would read one method and call it the whole block.
fn methods_of(code: &str, port: &str, ty: &str) -> Vec<Method> {
    let Some(at) = impl_block_start(code, port, ty) else {
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
    // The vacuity guard for the *core* scan, which the coverage check iterates. A scan that
    // matched nothing — an `impl` reformatted, a directory moved — would pass it without looking
    // at anything.
    //
    // The corresponding claim about the `tests/` scan is no longer a count: T048 drove it to zero
    // on purpose. See `the_detector_can_tell_a_stub_from_a_behaviour`.
    let core = core_port_impls();

    assert!(
        core.iter().any(|i| i.ty == "GitCli"),
        "the core scan lost `GitCli`; every test in this file is now vacuous. Found: {core:?}"
    );
    assert!(
        core.iter().any(|i| i.ty == "FakeGit"),
        "the core scan lost `FakeGit`, the fake the others are meant to match: {core:?}"
    );
    assert!(
        !test_sources().is_empty(),
        "the tests scan read no source files at all, so the narrowness check below cannot see a \
         violation even if one is written"
    );
}

#[test]
fn the_detector_can_tell_a_stub_from_a_behaviour() {
    // What `no_test_is_forced_to_supply_an_operation_it_does_not_use` rests on, now that its
    // correct answer is an empty list. Run over a source with one of each, so it fails if the
    // parser breaks rather than only if the corpus shrinks.
    //
    // Both methods are in one `impl` block deliberately: the block scan has to walk past the first
    // method's body, and a parser that stopped at the first `}` would report only `is_git_repo`
    // and call the block finished.
    //
    // The fixture names `Capability`, not a real port, because `port_impls_under` reads this
    // file's own text — and with a real port name it found this literal and reported the fixture
    // as a live violation. Which is the scan proving it is not vacuous, on its first run.
    let code = "\
impl Capability for Probe {
    fn is_git_repo(&self, _dir: &Path) -> bool {
        self.git
    }
    fn is_available(&self, _dir: &Path) -> bool {
        true
    }
}
";
    let methods = methods_of(code, "Capability", "Probe");
    let names: Vec<&str> = methods.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["is_git_repo", "is_available"],
        "the block scan did not read both methods"
    );

    // A fully-qualified header must resolve to the same block. `port_impls_under` strips the path
    // and reports the implementation either way, so a `methods_of` that only matched the bare form
    // would examine nothing and let a wide fake through — which it did, until a probe wrote one.
    let qualified = code.replace("impl Capability", "impl some::deep::path::Capability");
    let qualified_names: Vec<String> = methods_of(&qualified, "Capability", "Probe")
        .into_iter()
        .map(|m| m.name)
        .collect();
    assert_eq!(
        qualified_names, names,
        "a fully-qualified `impl` header found no methods; the narrowness check would pass on it \
         no matter how much it stubbed"
    );

    let by_name = |want: &str| methods.iter().find(|m| m.name == want).expect(want).stub;
    assert!(
        !by_name("is_git_repo"),
        "a method that consults `self` is configurable behaviour, not a stub — flagging it would \
         make the narrowness check fire on every honest fake"
    );
    assert!(
        by_name("is_available"),
        "a method that ignores every argument and never consults `self` can only return a \
         constant; if the detector misses this it will never find a real violation either"
    );
}

#[test]
fn every_capability_has_a_fake_in_the_core() {
    // FR-019. Was `#[ignore]`d at full strength while four capabilities had no fake; T048 added
    // them, so it now passes as written rather than having been softened to fit.
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
         real thing (FR-019): {missing:?}\n\nAdd one beside the capability it satisfies, as an \
         ordinary public item matching `FakeGit` — not behind a `cfg` and not in another crate."
    );
}

#[test]
fn every_fake_that_exists_is_exercised_by_a_test() {
    // SC-005's second half. A fake nothing constructs is not evidence that a capability is
    // fakeable — it is an unused type that happens to compile.
    let fakes: BTreeSet<String> = core_port_impls()
        .into_iter()
        .filter(inventory::PortImpl::is_fake)
        .map(|i| i.ty)
        .collect();
    let sources = test_sources();
    let mut unused = Vec::new();

    for fake in &fakes {
        // Construction, not mention. A bare `contains` counts a `use` line and — as a probe
        // found — a *guard's own allow-list*: adding `"FakeAiCliProvider"` to the known-fakes
        // list in `no_concrete_implementations.rs` was enough to satisfy this check with no test
        // constructing the fake anywhere. A capability's fake being listed somewhere is not
        // evidence anyone can test through it.
        let built = |code: &String| {
            code.contains(&format!("{fake}::")) || code.contains(&format!("{fake} {{"))
        };
        if !sources.values().any(built) {
            unused.push(fake.clone());
        }
    }

    assert!(
        unused.is_empty(),
        "these fakes exist but no test constructs them, so nothing exercises the capability \
         through them (SC-005): {unused:?}\n\nNaming one in a list does not count — the check \
         looks for `Fake…::` or `Fake… {{`."
    );
    assert!(
        !fakes.is_empty(),
        "no fakes found at all — the scan has drifted"
    );
}

#[test]
fn no_test_is_forced_to_supply_an_operation_it_does_not_use() {
    // FR-016, in the contract's own words: "if a test must implement a method it does not exercise
    // merely to satisfy the trait, the capability is too wide and must be split."
    //
    // It found six violations and they were answered by splitting one capability and deleting six
    // hand-rolled fakes, not by relaxing this predicate. What it holds now is that neither comes
    // back: a test that writes its own port implementation and stubs half of it fails here.
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
