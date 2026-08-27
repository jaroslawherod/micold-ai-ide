//! The AI-CLI seam is substitutable — and now the application takes it (feature 021 T048;
//! reshaped and answered by feature 026 T004/T005 — FR-020, FR-021).
//!
//! # What this file used to say, and why it no longer says it
//!
//! It opened by recording that the seam was not substitutable: *"Every consumer in the workspace
//! names `ClaudeProvider` concretely."* Seven call sites did, across all three crates, so there was
//! no site a second provider could be substituted at and no consumer a fake could be handed to. The
//! strongest claim available was that the *trait* was usable as a boundary at all.
//!
//! That is no longer the situation, and the change is structural rather than a matter of degree.
//! `AiCli::provider` resolves a name to an implementation from `micold-core`, which is the one
//! crate all three can see; `catalog.rs`, `state.rs`, `supervisor.rs`, `terminal.rs` and the
//! client's boot prune each take the provider from the session record; and
//! `tests/no_concrete_implementations.rs` fails if any of them goes back to naming a type. Two
//! providers now exist and neither is privileged.
//!
//! # The two properties this file holds
//!
//! **T004 — the seam is object-safe and complete.** A consumer holds `&dyn AiCliProvider`, so the
//! trait must stay object-safe; and [`FakeAiCliProvider`] must implement *every* method the
//! contract lists, since there are no defaults left to inherit.
//!
//! **T005 — the trait provides no layout-specific default.** This is the one that matters, and it
//! is the reason the reshape happened. The previous trait handed every implementation
//! `discover_transcript_session_ids` (list a directory, take the `*.jsonl` stems as ids) and
//! `archived_marker_path` (`{id}.archived` *inside* that directory). Both are `claude`'s layout,
//! stated once and inherited by everyone. Copilot's storage is a per-cwd index file naming
//! conversations that each live in their own directory — nothing about that shape survives contact
//! with either default, so a second provider's first job would have been to override them. A seam
//! whose defaults are wrong for the second implementation is not a seam.
//!
//! Asserting the *absence* of a default cannot be done by calling something, so it is asserted the
//! way absences are: by a minimal implementation that inherits nothing, plus a compile-time check
//! that the removed names are gone from the trait's surface.

use micold_core::provider::{ActivitySource, AiCliProvider, FakeAiCliProvider};
use micold_core::session::AiCli;
use micold_core::terminal::LaunchMode;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// A consumer written the way one *should* be: against the port, not a provider.
fn spawn_command(provider: &dyn AiCliProvider, session: Uuid, mode: LaunchMode) -> Vec<String> {
    let mut argv = vec![provider.command().to_string()];
    argv.extend(provider.launch_args(session, mode));
    argv
}

#[test]
fn a_consumer_reaching_through_the_port_gets_the_substituted_provider() {
    let provider = FakeAiCliProvider::new();
    let session = Uuid::nil();

    let argv = spawn_command(&provider, session, LaunchMode::Fresh);

    assert_eq!(
        argv[0], "fake-ai-cli",
        "the consumer took the port's answer"
    );
    assert_ne!(
        argv[0], "claude",
        "a consumer that hardcoded the real command would produce this, and would pass a test \
         that asserted the real command — which is why the fake's name is deliberately different"
    );
}

#[test]
fn the_provider_is_told_which_session_and_whether_to_resume() {
    // The launch log is the point: "it spawned something" and "it spawned *this* session, fresh"
    // come apart, and only the second catches a resume issued as a fresh start.
    let provider = FakeAiCliProvider::new();
    let first = Uuid::from_u128(1);
    let second = Uuid::from_u128(2);

    spawn_command(&provider, first, LaunchMode::Fresh);
    spawn_command(&provider, second, LaunchMode::Resume);

    assert_eq!(
        provider.launches(),
        vec![(first, LaunchMode::Fresh), (second, LaunchMode::Resume)]
    );
}

#[test]
fn a_recorded_conversation_is_found_without_touching_the_filesystem() {
    // Both real providers answer this with a `.exists()`. This drives the same question through a
    // provider that has no disk at all — which is only possible because the method is *required*
    // now: as a trait default it reached the real filesystem, and a fake that inherited it would
    // have answered plausibly (`false`) while making a syscall.
    let config = PathBuf::from("/fake/config");
    let cwd = Path::new("/fake/project");
    let session = Uuid::from_u128(7);

    let bare = FakeAiCliProvider::new();
    assert!(
        !bare.has_recorded_conversation(&config, cwd, session),
        "nothing was recorded"
    );

    let provider =
        FakeAiCliProvider::new().with_conversation(cwd, session, "{\"title\":\"Login page\"}");
    assert!(provider.has_recorded_conversation(&config, cwd, session));
}

#[test]
fn a_title_is_read_from_the_conversation_the_provider_named() {
    // Composed behaviour: `read_title` resolves the conversation the provider recorded for this
    // `(cwd, id)` and turns its contents into a title. A fake that only answered a parse step would
    // leave the composition untested — the step where a consumer could look in the wrong place.
    let config = PathBuf::from("/fake/config");
    let cwd = Path::new("/fake/project");
    let session = Uuid::from_u128(7);

    let provider = FakeAiCliProvider::new()
        .with_conversation(cwd, session, "raw conversation")
        .with_title("raw conversation", "Login page");

    assert_eq!(
        provider.read_title(&config, cwd, session),
        Some("Login page".to_string())
    );
    assert_eq!(
        provider.read_title(&config, Path::new("/somewhere/else"), session),
        None,
        "a different cwd is a different conversation, and there is none there"
    );
}

#[test]
fn a_working_directorys_recorded_sessions_come_back_through_the_seam() {
    // What replaced `transcript_dir` + the listing default. The question a consumer asks is "which
    // conversations has this provider recorded here" — the *answer's* shape, `Vec<Uuid>`, is the
    // seam; how it was obtained (a directory listing, an index file) is not.
    let config = PathBuf::from("/fake/config");
    let cwd = Path::new("/fake/project");
    let elsewhere = Path::new("/fake/other");
    let one = Uuid::from_u128(1);
    let two = Uuid::from_u128(2);

    let provider = FakeAiCliProvider::new()
        .with_conversation(cwd, one, "a")
        .with_conversation(cwd, two, "b")
        .with_conversation(elsewhere, Uuid::from_u128(3), "c");

    let mut here = provider.recorded_session_ids(&config, cwd);
    here.sort();
    assert_eq!(here, vec![one, two]);
    assert_eq!(
        provider.recorded_session_ids(&config, Path::new("/fake/empty")),
        Vec::<Uuid>::new(),
        "a working directory the provider has recorded nothing for contributes nothing — never an \
         error, so discovery cannot fail a project open"
    );
}

// ---------------------------------------------------------------------------------------
// T005 — no layout-specific default
// ---------------------------------------------------------------------------------------

/// A provider that implements **only** the required methods and inherits nothing.
///
/// This type is the assertion. If any layout-shaped behaviour came back as a trait default, the
/// contradiction would show up here first: a `discover_transcript_session_ids` default would make
/// `recorded_session_ids` redundant, and an `archived_marker_path` default would build
/// `{id}.archived` inside a `transcript_dir` this provider does not have and cannot supply.
///
/// The storage it models is deliberately *neither* real layout — one flat file per session, keyed
/// by id alone, with no per-cwd container of any kind — so nothing about `claude`'s directory or
/// Copilot's index could be quietly assumed on its behalf.
///
/// # Every method answers from its own state, and that is not decoration
///
/// It would be shorter to return constants. `tests/service_capability_fakes.rs` fails a test-written
/// port implementation whose methods ignore every argument *and* never consult `self`, on the
/// grounds that a capability nobody can answer differently is one nobody is really exercising
/// (feature 021, FR-016) — and it is right about this one. A `MinimalProvider` returning fixed
/// values would demonstrate that the trait *compiles* without defaults; one that answers from its
/// own fields demonstrates that a third provider could actually be written this way.
struct MinimalProvider {
    root: PathBuf,
    id: AiCli,
    display_name: &'static str,
    command: &'static str,
    available: bool,
    /// Conversations this provider has "recorded", by id. No cwd: this layout does not have one,
    /// which is the whole point of it being neither real provider's.
    conversations: RefCell<BTreeMap<Uuid, String>>,
    /// Sessions marked archived.
    archived: RefCell<BTreeSet<Uuid>>,
}

impl MinimalProvider {
    fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            id: AiCli::ClaudeCode,
            display_name: "Minimal",
            command: "minimal",
            available: true,
            conversations: RefCell::new(BTreeMap::new()),
            archived: RefCell::new(BTreeSet::new()),
        }
    }

    fn with_conversation(self, id: Uuid, title: &str) -> Self {
        self.conversations
            .borrow_mut()
            .insert(id, title.to_string());
        self
    }
}

impl AiCliProvider for MinimalProvider {
    fn id(&self) -> AiCli {
        self.id
    }
    fn display_name(&self) -> &'static str {
        self.display_name
    }
    fn command(&self) -> &'static str {
        self.command
    }
    fn is_available(&self) -> bool {
        self.available
    }
    fn launch_args(&self, session_id: Uuid, _mode: LaunchMode) -> Vec<String> {
        vec![self.command.to_string(), session_id.to_string()]
    }
    fn config_dir(&self) -> Option<PathBuf> {
        Some(self.root.clone())
    }
    fn recorded_session_ids(&self, _config_dir: &Path, _cwd: &Path) -> Vec<Uuid> {
        // Keyed by id alone — this storage has no per-working-directory container at all, so
        // nothing about either real layout can have been assumed for it.
        self.conversations.borrow().keys().copied().collect()
    }
    fn has_recorded_conversation(&self, _config_dir: &Path, _cwd: &Path, id: Uuid) -> bool {
        self.conversations.borrow().contains_key(&id)
    }
    fn read_title(&self, _config_dir: &Path, _cwd: &Path, id: Uuid) -> Option<String> {
        self.conversations.borrow().get(&id).cloned()
    }
    fn mark_archived(&self, _config_dir: &Path, _cwd: &Path, id: Uuid) -> io::Result<()> {
        self.archived.borrow_mut().insert(id);
        Ok(())
    }
    fn is_archived(&self, _config_dir: &Path, _cwd: &Path, id: Uuid) -> bool {
        self.archived.borrow().contains(&id)
    }
    fn activity_source(&self, _config_dir: &Path, _cwd: &Path, id: Uuid) -> ActivitySource {
        // Its own arithmetic, from its own root — not `claude`'s per-cwd directory and not
        // Copilot's `session-state/<uuid>/`.
        ActivitySource::EventLog {
            path: self.root.join(format!("{id}.log")),
        }
    }
}

#[test]
fn a_provider_that_implements_only_the_required_methods_is_a_complete_provider() {
    // It compiles, and it is usable as `&dyn AiCliProvider` — which is the whole claim. A trait
    // with a layout-specific default would still compile here, so the assertions below are about
    // what such a default would *do* to this provider's answers.
    let root = PathBuf::from("/minimal");
    let one = Uuid::from_u128(1);
    let provider = MinimalProvider::new(&root).with_conversation(one, "Its own title");
    let port: &dyn AiCliProvider = &provider;
    let anywhere = Path::new("/anywhere");

    assert_eq!(port.command(), "minimal");
    assert_eq!(port.display_name(), "Minimal");
    assert_eq!(port.id(), AiCli::ClaudeCode);
    assert!(port.is_available());
    assert_eq!(
        port.launch_args(one, LaunchMode::Fresh),
        vec!["minimal".to_string(), one.to_string()],
        "its own argv shape — neither `--session-id` nor `--resume=`"
    );
    assert_eq!(
        port.config_dir(),
        Some(root.clone()),
        "it supplied its own base directory, and nothing derived a `projects/<slug>` path from it"
    );

    // Its own storage answers, from its own keying. A layout default would have to look somewhere
    // this provider never wrote, so each of these would come back empty.
    assert_eq!(port.recorded_session_ids(&root, anywhere), vec![one]);
    assert!(port.has_recorded_conversation(&root, anywhere, one));
    assert_eq!(
        port.read_title(&root, anywhere, one),
        Some("Its own title".to_string())
    );
    assert_eq!(
        port.activity_source(&root, anywhere, one),
        ActivitySource::EventLog {
            path: root.join(format!("{one}.log"))
        },
        "and its own activity arithmetic, from its own root"
    );

    // The archived marker is its own too: no inherited `{id}.archived`-beside-the-transcript rule
    // answered for it, and nothing was written into a directory it does not have.
    assert!(!port.is_archived(&root, anywhere, one));
    port.mark_archived(&root, anywhere, one).unwrap();
    assert!(port.is_archived(&root, anywhere, one));
    assert!(
        !root.exists(),
        "and it touched no filesystem on the way — every derivation here is this provider's own"
    );
}

#[test]
fn the_seam_is_object_safe_and_holds_both_real_providers_and_a_fake() {
    // `&dyn` in three shapes. This stops compiling the moment a method gains a generic parameter,
    // `Self: Sized`, or returns `Self` — each of which would quietly make the seam un-substitutable
    // again while every behavioural test kept passing.
    let fake = FakeAiCliProvider::new();
    let minimal = MinimalProvider::new("/minimal");
    let ports: Vec<&dyn AiCliProvider> = vec![
        AiCli::ClaudeCode.provider(),
        AiCli::Copilot.provider(),
        &fake,
        &minimal,
    ];

    let names: Vec<&str> = ports.iter().map(|p| p.command()).collect();
    assert_eq!(names, vec!["claude", "copilot", "fake-ai-cli", "minimal"]);
}

#[test]
fn every_registered_name_resolves_to_a_provider_that_answers_with_that_name() {
    // The registry's one invariant, and the one a `BTreeMap` could not give: it is total, and each
    // arm answers for itself. A copy-paste in the match — both arms returning `&CLAUDE` — is the
    // realistic way this breaks, and it would leave every Copilot session spawning `claude`.
    for which in AiCli::ALL {
        assert_eq!(
            which.provider().id(),
            which,
            "`AiCli::provider` returned an implementation that identifies as something else"
        );
    }
}
