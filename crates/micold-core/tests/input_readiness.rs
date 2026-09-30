//! When a new session is ready for its first prompt, and how a prompt is typed (feature 034,
//! FR-017, research R12; U38–U40, U118–U123).
//!
//! A fresh session never reports "awaiting input" before its first prompt, so each CLI names its
//! own ready-for-input signal. A CLI with none is ready once its terminal has produced output and
//! then gone quiet for 1.5 s. The prompt is then typed as one submission: bracketed when the
//! terminal asked for bracketed paste, so a multi-line prompt does not submit at its first line.

use std::time::Duration;

use micold_core::clock::Uptime;
use micold_core::mcp::submission::{encode_submission, OutputSettled, SETTLE_AFTER};
use micold_core::provider::InputReadiness;
use micold_core::session::AiCli;

fn at(millis: u64) -> Uptime {
    Uptime::from_nanos(millis * 1_000_000)
}

/// Claude Code posts no `SessionStart` over an HTTP hook (quickstart §B3,
/// `specs/034-daemon-mcp-server/evidence/m3-real-cli.md`), so it has no ready signal of its own.
#[test]
fn claude_is_ready_once_its_output_has_settled() {
    assert_eq!(
        AiCli::ClaudeCode.provider().input_readiness(),
        InputReadiness::OutputSettled
    );
}

#[test]
fn pi_is_ready_on_its_components_session_start_event() {
    assert_eq!(
        AiCli::Pi.provider().input_readiness(),
        InputReadiness::ExtensionEvent("session_start")
    );
}

#[test]
fn copilot_is_ready_once_its_output_has_settled() {
    assert_eq!(
        AiCli::Copilot.provider().input_readiness(),
        InputReadiness::OutputSettled
    );
}

#[test]
fn a_bracketed_submission_is_wrapped_in_paste_markers_then_submitted() {
    assert_eq!(
        encode_submission("print the branch name", true),
        b"\x1b[200~print the branch name\x1b[201~\r".to_vec()
    );
}

#[test]
fn an_unbracketed_submission_is_the_text_then_a_carriage_return() {
    assert_eq!(
        encode_submission("print the branch name", false),
        b"print the branch name\r".to_vec()
    );
}

#[test]
fn a_multi_line_prompt_is_submitted_exactly_once() {
    let encoded = encode_submission("line one\nline two\n", true);
    assert_eq!(
        encoded.iter().filter(|&&b| b == b'\r').count(),
        1,
        "one submission, however many lines: {encoded:?}"
    );
    assert!(
        encoded.ends_with(b"\x1b[201~\r"),
        "the carriage return follows the paste: {encoded:?}"
    );
}

#[test]
fn with_no_output_yet_the_terminal_is_never_settled() {
    let rule = OutputSettled::new();
    assert!(
        !rule.is_ready(at(3_600_000)),
        "a CLI that has drawn nothing has not started"
    );
}

#[test]
fn a_second_and_a_half_of_silence_after_output_is_settled() {
    assert_eq!(SETTLE_AFTER, Duration::from_millis(1500));
    let mut rule = OutputSettled::new();
    rule.output(at(0));
    assert!(!rule.is_ready(at(1499)), "1.499 s is not yet settled");
    assert!(rule.is_ready(at(1500)), "1.5 s of silence is settled");
}

#[test]
fn new_output_during_the_silence_restarts_it() {
    let mut rule = OutputSettled::new();
    rule.output(at(0));
    rule.output(at(1000));
    assert!(
        !rule.is_ready(at(2000)),
        "the silence counts from the latest output"
    );
    assert!(rule.is_ready(at(2500)));
}
