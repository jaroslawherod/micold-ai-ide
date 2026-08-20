//! The per-session activity FSM (data-model §ActivitySignal, contracts/hooks.md).
//!
//! Tracks a session's derived "activity" from three input classes:
//! claude-CLI lifecycle hooks (delivered by the loopback HTTP receiver), terminal-derived spinner
//! evidence (an OSC 0 title carrying a braille spinner glyph), and process exit. It is a pure state
//! machine over [`ActivitySignal`] — no I/O, no clock, no persistence (activity resets to `Unknown`
//! on daemon restart, invariant H3/A4).
//!
//! The wire enum [`ActivitySignal`] is reused verbatim as the machine's current-state
//! representation, so no domain↔wire mapping is needed for this field.
//!
//! # Invariants upheld here
//! - **H1 / A1** — with no hooks ever delivered the state stays `Unknown`. `Unknown` is a
//!   first-class value and MUST NEVER become `AwaitingInput` from terminal evidence or its absence.
//! - **H1a / A1a** — terminal-derived evidence is *monotone toward `Working` only*:
//!   [`ActivityEvent::SpinnerObserved`] can only ever produce `Working`, and only from `Unknown`.
//!   It can never move a session toward `AwaitingInput`, never move it out of `Working`, and never
//!   revive an `Ended` session.
//! - `Ended` is absorbing — once ended, later events do not resurrect it.

use micold_core::protocol::messages::ActivitySignal;

/// Which claude-CLI lifecycle hook fired (contracts/hooks.md state-transition table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookKind {
    /// The user submitted a prompt — a turn is starting.
    UserPromptSubmit,
    /// A tool call is about to run.
    PreToolUse,
    /// A tool call finished — the turn continues (no state change until `Stop`).
    PostToolUse,
    /// The model turn ended (a strong hint the user is needed, not a guarantee — H4).
    Stop,
    /// The agent surfaced a notification (permission/idle/needs-input prompt).
    Notification,
}

/// An input event to the activity FSM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityEvent {
    /// A claude-CLI lifecycle hook fired (from the loopback receiver).
    Hook(HookKind),
    /// An OSC 0 title carrying a braille spinner glyph was observed on the terminal
    /// (from the `Event::Title` handler; see [`is_spinner_title`]).
    SpinnerObserved,
    /// The session's process exited or the supervisor gave up (terminal).
    Ended {
        /// Why it ended — mirrors [`ActivitySignal::Ended`]'s `reason` field.
        reason: String,
    },
}

/// A session's activity state machine over [`ActivitySignal`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activity {
    current: ActivitySignal,
}

impl Activity {
    /// A fresh machine in the initial `Unknown` state.
    pub fn new() -> Self {
        Self {
            current: ActivitySignal::Unknown,
        }
    }

    /// The current derived signal.
    pub fn signal(&self) -> &ActivitySignal {
        &self.current
    }

    /// Apply an input event, advancing the state per the transition table.
    ///
    /// Transitions (contracts/hooks.md):
    /// - `UserPromptSubmit`, `PreToolUse` → `Working`
    /// - `PostToolUse` → no change (stays `Working` until `Stop`)
    /// - `Stop`, `Notification` → `AwaitingInput`
    /// - `Ended { reason }` → `Ended { reason }` (absorbing)
    /// - `SpinnerObserved` → only `Unknown → Working`; a no-op from any other state (H1a/A1a)
    pub fn apply(&mut self, event: ActivityEvent) {
        // `Ended` is absorbing: once ended, nothing resurrects it.
        if matches!(self.current, ActivitySignal::Ended { .. }) {
            return;
        }

        match event {
            ActivityEvent::Hook(HookKind::UserPromptSubmit)
            | ActivityEvent::Hook(HookKind::PreToolUse) => {
                self.current = ActivitySignal::Working;
            }
            // Turn continues after a tool call — no change until `Stop`.
            ActivityEvent::Hook(HookKind::PostToolUse) => {}
            ActivityEvent::Hook(HookKind::Stop) | ActivityEvent::Hook(HookKind::Notification) => {
                self.current = ActivitySignal::AwaitingInput;
            }
            ActivityEvent::Ended { reason } => {
                self.current = ActivitySignal::Ended { reason };
            }
            // Terminal-derived evidence is monotone toward `Working` only (H1a/A1a): it may lift
            // `Unknown` to `Working` and do nothing else. From `Working`/`AwaitingInput` it is a
            // no-op (and `Ended` was already handled above).
            ActivityEvent::SpinnerObserved => {
                if matches!(self.current, ActivitySignal::Unknown) {
                    self.current = ActivitySignal::Working;
                }
            }
        }
    }
}

impl Default for Activity {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether an OSC 0 title carries a braille spinner glyph — the positive evidence that claude is
/// actively working. Used by the `Event::Title` handler to decide whether to emit
/// [`ActivityEvent::SpinnerObserved`].
///
/// Detection matches **any** codepoint in the Unicode Braille Patterns block (U+2800..=U+28FF)
/// rather than a fixed frame list: the spinner frames (`⠂ ⠐ …`) are a cosmetic upstream detail and
/// new frames MUST NOT go undetected (contracts/hooks.md). Note the idle `✳` glyph (U+2733) is
/// deliberately *not* matched — its presence carries no information (H1a/A1a).
pub fn is_spinner_title(title: &str) -> bool {
    title
        .chars()
        .any(|c| ('\u{2800}'..='\u{28FF}').contains(&c))
}

// ---------------------------------------------------------------------------------------
// Copilot's event log → the same vocabulary (feature 026, T063 — FR-018)
// ---------------------------------------------------------------------------------------

/// Map one line of a Copilot `events.jsonl` to an [`ActivityEvent`], or `None` to ignore it.
///
/// **Typed at `ActivityEvent`, not at [`HookKind`]**, and that is not a stylistic choice: most of
/// Copilot's turn events do land on a hook — `user.message` is a `UserPromptSubmit`,
/// `assistant.turn_end` is a `Stop` — but `session.shutdown` and `session.error` land on
/// `Ended { reason }`, which is a *sibling* variant of `ActivityEvent` with no `HookKind` to
/// express it. A mapping returning `HookKind` could not report a session ending at all.
///
/// The state machine above is untouched by this feature. A second source feeds it the events it
/// already consumes; nothing about the transitions changes.
///
/// # Everything it does not know is ignored, never rejected
///
/// This is another tool's internal log and it gains event types between releases — the 1.0.80
/// capture alone added three that the 1.0.62 contract does not list. An unknown `type`, a line that
/// is not JSON, a blank line and a line with no `type` at all all yield `None`, so a tail is never
/// ended by something Copilot started writing.
///
/// # The two sources cannot contradict each other
///
/// A Copilot session has two, not one: this log, and the shared braille-spinner path in
/// `terminal.rs`, which scans **every** PTY session's OSC-0 titles and is not provider-conditional.
/// A Copilot TUI drawing a spinner will trip it. That is harmless by construction —
/// `SpinnerObserved` only ever moves `Unknown → Working` and is a no-op from every other state
/// (H1a/A1a) — so it can add nothing this mapping would have contradicted.
pub fn copilot_event(line: &str) -> Option<ActivityEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    let kind = value.get("type")?.as_str()?;
    match kind {
        // A turn is starting or continuing.
        "user.message" => Some(ActivityEvent::Hook(HookKind::UserPromptSubmit)),
        "assistant.turn_start" | "tool.execution_start" => {
            Some(ActivityEvent::Hook(HookKind::PreToolUse))
        }
        // The turn continues — no state change until it ends. Mapped rather than ignored so the
        // vocabulary stays complete and a reader can see the decision was made.
        "tool.execution_complete" => Some(ActivityEvent::Hook(HookKind::PostToolUse)),
        // The model is done, or is waiting on the user's answer to a permission prompt.
        "assistant.turn_end" => Some(ActivityEvent::Hook(HookKind::Stop)),
        "permission.requested" => Some(ActivityEvent::Hook(HookKind::Notification)),
        // Terminal. `shutdownType` is Copilot's own word for how it went ("routine"); the error
        // form carries a message. Either way the reason is best-effort — a missing one is not a
        // reason to miss the ending.
        "session.shutdown" => Some(ActivityEvent::Ended {
            reason: value
                .get("data")
                .and_then(|d| d.get("shutdownType"))
                .and_then(|v| v.as_str())
                .unwrap_or("session ended")
                .to_string(),
        }),
        "session.error" => Some(ActivityEvent::Ended {
            reason: value
                .get("data")
                .and_then(|d| d.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("session error")
                .to_string(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hook(kind: HookKind) -> ActivityEvent {
        ActivityEvent::Hook(kind)
    }

    #[test]
    fn starts_unknown() {
        let a = Activity::new();
        assert_eq!(*a.signal(), ActivitySignal::Unknown);
        assert_eq!(Activity::default(), Activity::new());
    }

    #[test]
    fn hooks_drive_happy_path() {
        let mut a = Activity::new();

        a.apply(hook(HookKind::UserPromptSubmit));
        assert_eq!(*a.signal(), ActivitySignal::Working);

        a.apply(hook(HookKind::PreToolUse));
        assert_eq!(*a.signal(), ActivitySignal::Working);

        // PostToolUse is a no-op: still Working until Stop.
        a.apply(hook(HookKind::PostToolUse));
        assert_eq!(*a.signal(), ActivitySignal::Working);

        a.apply(hook(HookKind::Stop));
        assert_eq!(*a.signal(), ActivitySignal::AwaitingInput);
    }

    #[test]
    fn notification_awaits_input() {
        let mut a = Activity::new();
        a.apply(hook(HookKind::UserPromptSubmit));
        a.apply(hook(HookKind::Notification));
        assert_eq!(*a.signal(), ActivitySignal::AwaitingInput);
    }

    #[test]
    fn ended_from_working() {
        let mut a = Activity::new();
        a.apply(hook(HookKind::UserPromptSubmit));
        a.apply(ActivityEvent::Ended {
            reason: "process exited".into(),
        });
        assert_eq!(
            *a.signal(),
            ActivitySignal::Ended {
                reason: "process exited".into()
            }
        );
    }

    // H1 / A1: with no hooks ever delivered, the state never becomes AwaitingInput.
    #[test]
    fn h1_no_hooks_never_awaits_input() {
        let mut a = Activity::new();
        // No hooks at all: stays Unknown.
        assert_eq!(*a.signal(), ActivitySignal::Unknown);

        // Only spinner evidence: lifts to Working, but NEVER AwaitingInput.
        a.apply(ActivityEvent::SpinnerObserved);
        assert_eq!(*a.signal(), ActivitySignal::Working);
        assert_ne!(*a.signal(), ActivitySignal::AwaitingInput);

        // More spinner evidence never moves it toward AwaitingInput.
        a.apply(ActivityEvent::SpinnerObserved);
        assert_eq!(*a.signal(), ActivitySignal::Working);
    }

    // H1a / A1a: SpinnerObserved only acts from Unknown; from every other state it is a no-op.
    #[test]
    fn h1a_spinner_from_unknown_only() {
        // From Unknown → Working.
        let mut a = Activity::new();
        a.apply(ActivityEvent::SpinnerObserved);
        assert_eq!(*a.signal(), ActivitySignal::Working);

        // From Working → no-op (stays Working, never leaves it).
        let mut a = Activity::new();
        a.apply(hook(HookKind::PreToolUse));
        a.apply(ActivityEvent::SpinnerObserved);
        assert_eq!(*a.signal(), ActivitySignal::Working);

        // From AwaitingInput → no-op (does NOT move to Working).
        let mut a = Activity::new();
        a.apply(hook(HookKind::Stop));
        assert_eq!(*a.signal(), ActivitySignal::AwaitingInput);
        a.apply(ActivityEvent::SpinnerObserved);
        assert_eq!(*a.signal(), ActivitySignal::AwaitingInput);

        // From Ended → no-op (does not revive).
        let mut a = Activity::new();
        a.apply(ActivityEvent::Ended {
            reason: "gave up".into(),
        });
        a.apply(ActivityEvent::SpinnerObserved);
        assert_eq!(
            *a.signal(),
            ActivitySignal::Ended {
                reason: "gave up".into()
            }
        );
    }

    // Ended is absorbing: later events of every class leave it Ended.
    #[test]
    fn ended_is_absorbing() {
        let mut a = Activity::new();
        a.apply(ActivityEvent::Ended {
            reason: "exit 0".into(),
        });
        let ended = ActivitySignal::Ended {
            reason: "exit 0".into(),
        };

        a.apply(hook(HookKind::Stop));
        assert_eq!(*a.signal(), ended);

        a.apply(ActivityEvent::SpinnerObserved);
        assert_eq!(*a.signal(), ended);

        a.apply(hook(HookKind::UserPromptSubmit));
        assert_eq!(*a.signal(), ended);

        // A second Ended does not overwrite the first reason.
        a.apply(ActivityEvent::Ended {
            reason: "exit 1".into(),
        });
        assert_eq!(*a.signal(), ended);
    }

    #[test]
    fn is_spinner_title_detects_braille() {
        // Braille spinner frame present.
        assert!(is_spinner_title("⠋ Working"));
        assert!(is_spinner_title("⠂ my-project"));
        // Boundary codepoints of the block.
        assert!(is_spinner_title("\u{2800}"));
        assert!(is_spinner_title("\u{28FF}"));
    }

    #[test]
    fn is_spinner_title_rejects_plain_and_idle() {
        // Plain title.
        assert!(!is_spinner_title("my-project"));
        assert!(!is_spinner_title(""));
        // The idle `✳` glyph (U+2733) is deliberately NOT spinner evidence.
        assert!(!is_spinner_title("✳ my-project"));
    }
}
