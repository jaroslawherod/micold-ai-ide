//! What the transcript draws as text, render-free (feature 1550, research R2): the strings the view
//! passes to its text leaves, moved here from `ui/transcript/` and `ui/material/inline_diff.rs` so
//! that copying a selection ([`super::units`]) reads the same strings the view draws, without a
//! renderer. The view re-exports each name where it lived.

use crate::icons::Icon;
use micold_core::protocol::messages::Resolution;
use micold_core::structured::edit_diff::HunkNote;
use micold_core::structured::insights::figures::{turn_details, turn_figures, ESTIMATE_NOTE};
use micold_core::structured::insights::TurnUsage;
use micold_core::structured::transcript::{
    footer_figures, CommandChip, Outcome, ToolStatus, TurnFooter,
};

/// What an empty transcript says (FR-006).
pub const NO_OUTPUT: &str = "No output yet";
/// The label a raw row leads with (research R4), so it reads apart from the conversation.
pub const RAW_LABEL: &str = "Raw event";
/// A thinking card's title.
pub const THINKING_TITLE: &str = "Thinking";
/// What a user message is headed with.
pub const USER_LABEL: &str = "You";
/// What stands where a turn's footer would, when the process ended mid-turn (FR-004).
pub const ENDED_WITHOUT_RESULT: &str = "The turn ended without a result";
/// What stands where a turn's footer would, when the user interrupted it (feature 941, FR-014).
pub const INTERRUPTED: &str = "You interrupted this turn";
/// What a message sent with pictures names them under (feature 941, FR-030).
pub const ATTACHED: &str = "Attached";
/// What heads a result whose call is not in the transcript (FR-020): shown, never as an error.
pub const ORPHAN_RESULT: &str = "Result of a tool call not shown";

/// Which colour role a status takes beside its icon and label (FR-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Supporting: the muted foreground.
    Muted,
    /// A failure: the error role, on the icon and the label.
    Error,
}

/// A status as an icon, a label and a tone: failure never rests on colour alone (FR-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    pub icon: Icon,
    pub label: &'static str,
    pub tone: Tone,
}

/// A tool card's status.
pub fn tool_status(status: ToolStatus) -> Status {
    let (icon, label, tone) = match status {
        ToolStatus::Running => (Icon::ToolRunning, "Running", Tone::Muted),
        ToolStatus::Done => (Icon::ToolDone, "Done", Tone::Muted),
        ToolStatus::Failed => (Icon::ToolFailed, "Failed", Tone::Error),
        ToolStatus::NoResult => (Icon::ToolNoResult, "No result", Tone::Muted),
    };
    Status { icon, label, tone }
}
/// `"1 sub-agent message"`, `"3 sub-agent messages"`.
pub fn subagent_count(n: u64) -> String {
    if n == 1 {
        "1 sub-agent message".to_owned()
    } else {
        format!("{n} sub-agent messages")
    }
}
/// A turn footer as it is drawn (FR-005, FR-012).
#[derive(Debug, Clone, PartialEq)]
pub struct FooterView {
    pub status: Status,
    /// Duration, the session's running cost, turns: each only when the result reports it.
    pub figures: Vec<String>,
    /// An error turn's text.
    pub error_text: Option<String>,
    /// What its tooltip says, one line each (feature 944, FR-006, FR-026); none: no tooltip.
    pub details: Vec<String>,
}

/// `footer`'s view.
pub fn footer_view(footer: &TurnFooter) -> FooterView {
    let status = match footer.outcome {
        Outcome::Success => Status {
            icon: Icon::ToolDone,
            label: "Done",
            tone: Tone::Muted,
        },
        Outcome::Error => Status {
            icon: Icon::ToolFailed,
            label: "Failed",
            tone: Tone::Error,
        },
    };
    FooterView {
        status,
        figures: footer_figures(footer),
        error_text: footer.error_text.clone(),
        details: Vec::new(),
    }
}

/// `"1 earlier event not shown"`, `"40 earlier events not shown"` (FR-004).
pub fn omitted_text(n: u64) -> String {
    if n == 1 {
        "1 earlier event not shown".to_owned()
    } else {
        format!("{n} earlier events not shown")
    }
}

/// What an event too large to send says (FR-004).
pub fn oversized_text(original_bytes: u64) -> String {
    format!("An event of {original_bytes} bytes was too large to show")
}

/// What a `/clear` marker says (#943 FR-005).
pub const CLEARED: &str = "Conversation cleared: the turns above are from before the clear";

/// What a compaction marker says (#943 FR-004).
pub fn compacted_text(trigger: Option<&str>) -> String {
    match trigger {
        Some("auto") => "Conversation compacted automatically".to_owned(),
        _ => "Conversation compacted".to_owned(),
    }
}

/// What a model marker says (#943 FR-015, FR-016).
pub fn model_changed_text(model: &str, refused: Option<&str>) -> String {
    match refused {
        None => format!("Model changed to {model}"),
        Some(reason) => format!("Model {model} was refused: {reason}"),
    }
}

/// What a turn from before a `/clear` is marked with (#943 FR-005, US1 scenario 5).
pub const BEFORE_THE_CLEAR: &str = "Before the clear";

/// What heads a built-in command's answer (#943 FR-006).
pub const COMMAND_OUTPUT_TITLE: &str = "Command output";

/// What a command chip says with no description behind it (#943 FR-003).
pub const NO_COMMAND_DESCRIPTION: &str = "The session's command list gave no description";

/// [`BEFORE_THE_CLEAR`] for an entry of an earlier conversation than the transcript's latest
/// (#943 FR-005): the turns above a `/clear` stay, marked.
pub fn era_tag(entry_era: u32, latest_era: u32) -> Option<&'static str> {
    (entry_era < latest_era).then_some(BEFORE_THE_CLEAR)
}

/// A command chip's header: the command as the user typed it (#943 FR-003), never the
/// instructions the CLI expanded it into.
pub fn command_chip_title(chip: &CommandChip) -> String {
    let args = chip.invocation.args.trim();
    if args.is_empty() {
        format!("/{}", chip.invocation.name)
    } else {
        format!("/{} {args}", chip.invocation.name)
    }
}

/// What stands behind a command chip: the description the session's command list gave when the
/// command was sent, with its argument hint (#943 FR-003, decision 4).
pub fn command_chip_description(chip: &CommandChip) -> String {
    let mut parts: Vec<&str> = Vec::new();
    parts.extend(
        chip.description
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty()),
    );
    parts.extend(
        chip.argument_hint
            .as_deref()
            .map(str::trim)
            .filter(|h| !h.is_empty()),
    );
    if parts.is_empty() {
        NO_COMMAND_DESCRIPTION.to_owned()
    } else {
        parts.join(" · ")
    }
}

/// What a notice of sub-agent messages naming no card says (FR-024).
pub fn subagent_notice_text(count: u64) -> String {
    format!("{} not shown", subagent_count(count))
}
/// The first line of `text` that holds anything: a folded thinking card's summary.
pub fn first_line(text: &str) -> &str {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
}
/// The line naming a user message's pictures, in order (FR-030); `None` without any.
pub fn attachments_line(names: &[String]) -> Option<String> {
    (!names.is_empty()).then(|| format!("{ATTACHED}: {}", names.join(", ")))
}

/// `footer`'s view with the turn's own figures after #939's (X2, FR-006): its cost and tokens,
/// and in its details the cached input, the per-model rows and the estimate note (FR-026). A turn
/// without figures adds nothing but the note, when #939's session total is a cost.
pub fn footer_view_with(footer: &TurnFooter, turn: Option<&TurnUsage>) -> FooterView {
    let mut view = footer_view(footer);
    let Some(turn) = turn else {
        if footer.cost_usd.is_some() {
            view.details.push(ESTIMATE_NOTE.to_owned());
        }
        return view;
    };
    view.figures.extend(turn_figures(turn));
    view.details = turn_details(turn);
    if footer.cost_usd.is_some() && !view.details.iter().any(|l| l == ESTIMATE_NOTE) {
        view.details.push(ESTIMATE_NOTE.to_owned());
    }
    view
}

/// How many columns apart the tab stops are when a tab is laid out as spaces.
const TAB_STOP: usize = 4;

/// `text` as plain text draws it: each tab expanded to spaces up to the next tab stop, since the
/// text stack draws a tab as a missing glyph (`Read` results are `cat -n` lines: `1\tline`).
pub fn plain_text(text: &str) -> String {
    if !text.contains('\t') {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len() + 8);
    let mut column = 0;
    for c in text.chars() {
        match c {
            '\t' => {
                let pad = TAB_STOP - column % TAB_STOP;
                out.extend(std::iter::repeat_n(' ', pad));
                column += pad;
            }
            '\n' => {
                out.push(c);
                column = 0;
            }
            _ => {
                out.push(c);
                column += 1;
            }
        }
    }
    out
}

/// `[image not shown]`: what a result part the transcript cannot show reads as (FR-011).
pub fn unshown_caption(kind: &str) -> String {
    format!("[{kind} not shown]")
}

/// What a hunk's note says.
pub fn note_text(note: HunkNote) -> &'static str {
    match note {
        HunkNote::ReplaceAll => "Replaces every occurrence",
        HunkNote::NoChange => "No change",
        HunkNote::EmptyWrite => "Writes an empty file",
        HunkNote::NewContent => "New content",
    }
}

/// What the row under a hunk whose final newline changed says.
pub const NO_NEWLINE: &str = "No newline at end";

/// The name of hunk `ordinal` of `of`.
pub fn hunk_title(ordinal: u32, of: u32) -> String {
    format!("Edit {ordinal} of {of}")
}

/// What a cut input says under its preview.
pub const INPUT_CUT: &str = "Only the start of the input is shown";
/// A plan approval's title.
pub const PLAN_TITLE: &str = "Claude's plan";
/// A question request's title.
pub const QUESTION_TITLE: &str = "Claude asks";
/// The label a request leaves on its card once it has ended (research R11).
pub fn resolution_label(resolution: Resolution) -> &'static str {
    match resolution {
        Resolution::Allowed => "Allowed",
        Resolution::AllowedAlways => "Always allowed",
        Resolution::Denied => "Denied",
        Resolution::PlanApproved { .. } => "Plan approved",
        Resolution::KeptPlanning => "Kept planning",
        Resolution::Answered => "Answered",
        Resolution::Declined => "Declined",
        Resolution::Withdrawn { .. } => "Withdrawn",
    }
}
/// What a tool request's note says Always allow does: it names the rule it adds.
pub fn always_allow_note(rule: &str) -> String {
    format!("Always allow also allows {rule} for the rest of this session")
}
/// `text` as structured text: indented JSON when it parses, else as it came (a cut preview).
pub fn structured_input(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| text.to_owned())
}
