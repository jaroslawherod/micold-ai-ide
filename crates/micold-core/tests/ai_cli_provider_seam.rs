//! The AI-CLI seam is substitutable — and nothing in the application takes it (feature 021, T048 —
//! FR-019, SC-005).
//!
//! # The finding this file exists to record
//!
//! `AiCliProvider` is described as the seam that lets another provider be added "without touching
//! the session model, persistence, sidebar, or terminal wiring". It is not, yet. Every consumer in
//! the workspace names [`ClaudeProvider`] concretely — `terminal::launch_args`, the daemon's
//! `catalog`, `supervisor` and `state`, and the client's `main.rs`. Not one of them accepts the
//! port as a parameter, so there is no call site a second provider could be substituted at, and no
//! consumer a fake can be handed to.
//!
//! That makes SC-005 weaker for this capability than for the other nine, and the difference is
//! worth stating rather than papering over: for `Git`, `FolderScanner` or `ProjectStore` the fake
//! drives a real consumer (`Workspace`, `Catalog`) and the assertion is about that consumer's
//! behaviour. Here the strongest available claim is that the *trait* is usable as a boundary at
//! all — which is what these tests make, through `&dyn AiCliProvider` rather than a concrete type,
//! so they would stop compiling if the port stopped being object-safe or a consumer-facing
//! operation moved onto the concrete provider.
//!
//! T049 is where this is answerable: the `Capabilities` struct assembled once at boot is exactly
//! the place a provider would be chosen, and once consumers take it from there rather than naming
//! `ClaudeProvider`, this file should grow the consumer-driven test it cannot have today.
//!
//! # Why the fake overrides the trait's two default methods
//!
//! `has_recorded_conversation` and `read_title` are provided by the trait, and both reach the real
//! filesystem — `.exists()`, `read_to_string`. Inheriting them would give a fake that answers
//! plausibly (`false`, `None`) while making a syscall, which satisfies "zero real filesystem
//! access" on paper and not in fact. So [`FakeAiCliProvider`] serves both from its in-memory
//! transcripts, and the tests below drive them.

use micold_core::provider::{AiCliProvider, FakeAiCliProvider};
use micold_core::terminal::LaunchMode;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// A consumer written the way one *should* be: against the port, not a provider.
///
/// Stands in for the call sites that name `ClaudeProvider` today. If T049 converts them, this
/// helper is what they will look like, and the tests below already cover the shape.
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
    // The trait's own default reaches for `.exists()`. This drives the same question through a
    // provider that has no disk at all.
    let config = PathBuf::from("/fake/config");
    let cwd = Path::new("/fake/project");
    let session = Uuid::from_u128(7);

    let bare = FakeAiCliProvider::new();
    assert!(
        !bare.has_recorded_conversation(&config, cwd, session),
        "nothing was recorded"
    );

    let path = bare.transcript_path(&config, cwd, session);
    let provider = FakeAiCliProvider::new().with_transcript(&path, "{\"title\":\"Login page\"}");
    assert!(provider.has_recorded_conversation(&config, cwd, session));
}

#[test]
fn a_title_is_read_from_the_transcript_the_provider_named() {
    // Composed behaviour: `read_title` resolves a path through `transcript_path`, fetches those
    // contents, and hands them to `parse_title`. A fake that only answered `parse_title` would
    // leave the composition untested — the step where a consumer could look in the wrong place.
    let config = PathBuf::from("/fake/config");
    let cwd = Path::new("/fake/project");
    let session = Uuid::from_u128(7);

    let path = FakeAiCliProvider::new().transcript_path(&config, cwd, session);
    let provider = FakeAiCliProvider::new()
        .with_transcript(&path, "raw transcript")
        .with_title("raw transcript", "Login page");

    assert_eq!(
        provider.read_title(&config, cwd, session),
        Some("Login page".to_string())
    );
    assert_eq!(
        provider.read_title(&config, Path::new("/somewhere/else"), session),
        None,
        "a different cwd resolves to a different transcript, and there is none there"
    );
}

#[test]
fn each_session_gets_its_own_transcript_within_a_project() {
    // The property every consumer depends on and none of them states: two sessions in one folder
    // must not share a transcript path, or one session's title would be read for another.
    let provider = FakeAiCliProvider::new();
    let config = PathBuf::from("/fake/config");
    let cwd = Path::new("/fake/project");

    let one = provider.transcript_path(&config, cwd, Uuid::from_u128(1));
    let two = provider.transcript_path(&config, cwd, Uuid::from_u128(2));

    assert_ne!(one, two);
    assert_eq!(
        one.parent(),
        two.parent(),
        "and both sit in the project's transcript directory, which is what `transcript_dir` \
         exists to let a caller list"
    );
    assert_eq!(
        one.parent(),
        Some(provider.transcript_dir(&config, cwd).as_path())
    );
}
