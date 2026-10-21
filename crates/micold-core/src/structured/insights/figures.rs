//! The insights as text (data-model.md *Formatting*, research R9, FR-026). An absent figure
//! yields no text (FR-004, FR-008).

use super::{
    ContextState, ModelFigures, SessionTotals, SubagentState, Task, TaskState, ThreadView, Tokens,
    TurnUsage,
};
use crate::format::group_digits;
use crate::structured::transcript::{format_cost, format_turns};

/// What every cost's details say in words (FR-026).
pub const ESTIMATE_NOTE: &str =
    "Cost estimated at API list prices; a subscription plan does not bill this amount.";

/// `"$0.0052 est."`: a cost is always labelled an estimate (FR-026).
fn estimate(usd: f64) -> String {
    format!("{} est.", format_cost(usd))
}

/// `"in 4 · out 106"`, then `"cached 28,624 read · 8,515 written"` when `cached`; each figure
/// only when reported.
fn token_parts(tokens: &Tokens, cached: bool) -> Vec<String> {
    let part = |label: &str, n: Option<u64>| n.map(|n| format!("{label}{}", group_digits(n)));
    let mut parts: Vec<String> = [part("in ", tokens.input), part("out ", tokens.output)]
        .into_iter()
        .flatten()
        .collect();
    if cached {
        parts.extend(cached_text(tokens));
    }
    parts
}

/// `"cached 28,624 read · 8,515 written"`, or the half reported.
fn cached_text(tokens: &Tokens) -> Option<String> {
    let read = tokens
        .cache_read
        .map(|n| format!("{} read", group_digits(n)));
    let written = tokens
        .cache_creation
        .map(|n| format!("{} written", group_digits(n)));
    let both: Vec<String> = [read, written].into_iter().flatten().collect();
    (!both.is_empty()).then(|| format!("cached {}", both.join(" · ")))
}

/// `"claude-sonnet-5-5: in 2 · out 16 · cached 28,624 read · 8,515 written · $0.0372 est."`
fn model_line(model: &str, figures: &ModelFigures) -> String {
    let tokens = Tokens {
        input: Some(figures.input),
        output: Some(figures.output),
        cache_read: Some(figures.cache_read),
        cache_creation: Some(figures.cache_creation),
    };
    let mut parts = token_parts(&tokens, true);
    parts.push(estimate(figures.cost_usd));
    format!("{model}: {}", parts.join(" · "))
}

/// The turn footer's figures (FR-006): its cost, then its input and output tokens.
pub fn turn_figures(turn: &TurnUsage) -> Vec<String> {
    let tokens = turn
        .tokens
        .map(|t| token_parts(&t, false).join(" · "))
        .filter(|t| !t.is_empty());
    [turn.cost_usd.map(estimate), tokens]
        .into_iter()
        .flatten()
        .collect()
}

/// The turn footer's details (FR-006, FR-026): cached input, one line per model, and the
/// estimate note when a cost is shown.
pub fn turn_details(turn: &TurnUsage) -> Vec<String> {
    let mut lines: Vec<String> = turn
        .tokens
        .as_ref()
        .and_then(cached_text)
        .map(|c| format!("Input {c}"))
        .into_iter()
        .collect();
    lines.extend(
        turn.models
            .iter()
            .flatten()
            .map(|(model, figures)| model_line(model, figures)),
    );
    if turn.restarted {
        lines.push("The CLI's running figures started again: shown as reported.".to_string());
    }
    if turn.cost_usd.is_some() || turn.models.as_ref().is_some_and(|m| !m.is_empty()) {
        lines.push(ESTIMATE_NOTE.to_string());
    }
    lines
}

/// `"$0.14 est. · 3 turns"`: the session totals and how many turns they cover (FR-007); none
/// while no turn is counted, since no cost was reported (FR-008).
pub fn totals_line(totals: &SessionTotals) -> Option<String> {
    (totals.turns_counted > 0).then(|| {
        format!(
            "{} · {}",
            estimate(totals.cost_usd),
            format_turns(totals.turns_counted)
        )
    })
}

/// The totals' details (FR-007, FR-008, FR-026): each token figure, one line per model, the
/// turns left out, whether earlier turns are not held, and the estimate note when a cost is shown.
pub fn totals_details(totals: &SessionTotals) -> Vec<String> {
    let tokens = token_parts(&totals.tokens, true);
    let mut lines: Vec<String> = (!tokens.is_empty())
        .then(|| format!("Tokens: {}", tokens.join(" · ")))
        .into_iter()
        .collect();
    lines.extend(
        totals
            .models
            .iter()
            .map(|(model, figures)| model_line(model, figures)),
    );
    if totals.turns_left_out > 0 {
        lines.push(format!(
            "Leaves out {} without usage figures",
            format_turns(totals.turns_left_out)
        ));
    }
    if totals.partial {
        lines.push("Partial: covers the turns still held".to_string());
    }
    if totals.turns_counted > 0 {
        lines.push(ESTIMATE_NOTE.to_string());
    }
    lines
}

/// The meter's label (FR-001, FR-003, FR-004): `"37,141 / 1,000,000 · 4%"`.
pub fn context_label(state: &ContextState) -> String {
    match *state {
        ContextState::NoData => "No data yet".to_string(),
        ContextState::TokensOnly { used } => format!("{} tokens", group_digits(used)),
        ContextState::Measured {
            used,
            window,
            percent,
            warning,
            ..
        } => {
            let label = format!(
                "{} / {} · {percent}%",
                group_digits(used),
                group_digits(window)
            );
            if warning {
                format!("{label} · Near compaction")
            } else {
                label
            }
        }
    }
}

/// The meter's details (FR-001, FR-002): used, window and threshold, each when known.
pub fn context_details(state: &ContextState) -> Vec<String> {
    let tokens = |label: &str, n: u64| format!("{label}: {} tokens", group_digits(n));
    match *state {
        ContextState::NoData => Vec::new(),
        ContextState::TokensOnly { used } => vec![tokens("Context used", used)],
        ContextState::Measured {
            used,
            window,
            threshold,
            warning,
            ..
        } => {
            let mut lines = vec![
                tokens("Context used", used),
                tokens("Context window", window),
            ];
            lines.extend(threshold.map(|t| tokens("Compaction threshold", t)));
            if warning {
                lines.push("Near compaction".to_string());
            }
            lines
        }
    }
}

/// A task's state in words, with its outcome when the CLI reported one (FR-020):
/// `"Finished · Background command … completed (exit code 0)"`.
pub fn task_label(task: &Task, cli_running: bool) -> String {
    let state = match task.display_state(cli_running) {
        TaskState::Running => "Running",
        TaskState::Finished => "Finished",
        TaskState::Failed => "Failed",
        TaskState::Stopped => "Stopped",
        TaskState::EndedWithoutReport => "Ended without a report",
    };
    match &task.outcome {
        Some(outcome) => format!("{state} · {outcome}"),
        None => state.to_string(),
    }
}

/// A thread's folded summary (FR-015): `"Subagent · running · 3 tool calls · Reading notes.txt"`;
/// the activity only while it runs.
pub fn thread_summary_line(view: &ThreadView) -> String {
    let state = match view.state {
        SubagentState::Running => "running",
        SubagentState::Finished => "finished",
        SubagentState::Failed => "failed",
        SubagentState::Stopped => "stopped",
        SubagentState::EndedWithoutResult => "ended without a result",
    };
    let calls = match view.summary.tool_calls {
        1 => "1 tool call".to_string(),
        n => format!("{n} tool calls"),
    };
    let mut parts = vec!["Subagent", state, &calls];
    if view.state == SubagentState::Running {
        parts.extend(view.activity.as_deref());
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(input: u64, output: u64, cache_read: u64, cache_creation: u64) -> Tokens {
        Tokens {
            input: Some(input),
            output: Some(output),
            cache_read: Some(cache_read),
            cache_creation: Some(cache_creation),
        }
    }

    fn sonnet() -> (String, ModelFigures) {
        let figures = ModelFigures {
            input: 2,
            output: 16,
            cache_read: 28_624,
            cache_creation: 8_515,
            cost_usd: 0.0372,
            context_window: Some(1_000_000),
        };
        ("claude-sonnet-5-5".to_string(), figures)
    }

    fn turn() -> TurnUsage {
        TurnUsage {
            result_seq: 9,
            tokens: Some(tokens(4, 106, 28_624, 8_515)),
            cost_usd: Some(0.0052),
            models: Some(vec![sonnet()]),
            restarted: false,
            counted: true,
        }
    }

    #[test]
    fn a_turns_figures_are_its_estimated_cost_and_its_tokens() {
        assert_eq!(turn_figures(&turn()), ["$0.0052 est.", "in 4 · out 106"]);
        let big = TurnUsage {
            tokens: Some(tokens(37_141, 1_200, 0, 0)),
            ..turn()
        };
        assert_eq!(turn_figures(&big)[1], "in 37,141 · out 1,200");
    }

    #[test]
    fn a_turns_details_are_cached_input_each_model_and_the_estimate_note() {
        assert_eq!(
            turn_details(&turn()),
            [
                "Input cached 28,624 read · 8,515 written",
                "claude-sonnet-5-5: in 2 · out 16 · cached 28,624 read · 8,515 written · $0.0372 est.",
                ESTIMATE_NOTE,
            ]
        );
        assert!(ESTIMATE_NOTE.contains(
            "estimated at API list prices; a subscription plan does not bill this amount"
        ));
    }

    #[test]
    fn an_absent_figure_yields_no_text() {
        let bare = TurnUsage {
            tokens: None,
            cost_usd: None,
            models: None,
            counted: false,
            ..turn()
        };
        assert!(turn_figures(&bare).is_empty());
        assert!(turn_details(&bare).is_empty());
        let input_only = TurnUsage {
            tokens: Some(Tokens {
                input: Some(4),
                ..Tokens::default()
            }),
            cost_usd: None,
            ..bare
        };
        assert_eq!(turn_figures(&input_only), ["in 4"]);
        assert!(turn_details(&input_only).is_empty());
    }

    #[test]
    fn the_totals_line_is_the_estimated_cost_and_the_turns_counted() {
        let totals = SessionTotals {
            cost_usd: 0.14,
            turns_counted: 3,
            ..SessionTotals::default()
        };
        assert_eq!(totals_line(&totals).unwrap(), "$0.1400 est. · 3 turns");
        let dear = SessionTotals {
            cost_usd: 1.244,
            turns_counted: 1,
            ..SessionTotals::default()
        };
        assert_eq!(totals_line(&dear).unwrap(), "$1.24 est. · 1 turn");
        assert_eq!(
            totals_line(&SessionTotals::default()),
            None,
            "no turn counted"
        );
    }

    #[test]
    fn the_totals_details_name_each_figure_the_left_out_turns_and_partial() {
        let totals = SessionTotals {
            cost_usd: 0.14,
            tokens: tokens(12, 3_400, 56_000, 7_800),
            models: vec![sonnet()],
            turns_counted: 3,
            turns_left_out: 2,
            partial: true,
        };
        assert_eq!(
            totals_details(&totals),
            [
                "Tokens: in 12 · out 3,400 · cached 56,000 read · 7,800 written",
                "claude-sonnet-5-5: in 2 · out 16 · cached 28,624 read · 8,515 written · $0.0372 est.",
                "Leaves out 2 turns without usage figures",
                "Partial: covers the turns still held",
                ESTIMATE_NOTE,
            ]
        );
        let whole = SessionTotals::default();
        assert!(
            totals_details(&whole).is_empty(),
            "no cost, so no estimate note"
        );
    }

    #[test]
    fn the_context_label_is_used_over_window_and_percent() {
        let measured = ContextState::Measured {
            used: 37_141,
            window: 1_000_000,
            percent: 4,
            threshold: Some(384_000),
            warning: false,
        };
        assert_eq!(context_label(&measured), "37,141 / 1,000,000 · 4%");
        assert_eq!(context_label(&ContextState::NoData), "No data yet");
        assert_eq!(
            context_label(&ContextState::TokensOnly { used: 37_141 }),
            "37,141 tokens"
        );
        let near = ContextState::Measured {
            used: 390_000,
            window: 1_000_000,
            percent: 39,
            threshold: Some(384_000),
            warning: true,
        };
        assert_eq!(
            context_label(&near),
            "390,000 / 1,000,000 · 39% · Near compaction"
        );
        assert_eq!(
            context_details(&near),
            [
                "Context used: 390,000 tokens",
                "Context window: 1,000,000 tokens",
                "Compaction threshold: 384,000 tokens",
                "Near compaction",
            ]
        );
    }

    #[test]
    fn the_context_details_name_only_the_figures_known() {
        assert!(context_details(&ContextState::NoData).is_empty());
        assert_eq!(
            context_details(&ContextState::TokensOnly { used: 5 }),
            ["Context used: 5 tokens"]
        );
        let no_threshold = ContextState::Measured {
            used: 5,
            window: 1_000,
            percent: 1,
            threshold: None,
            warning: false,
        };
        assert_eq!(
            context_details(&no_threshold),
            ["Context used: 5 tokens", "Context window: 1,000 tokens"]
        );
    }

    fn task(state: TaskState, outcome: Option<&str>) -> Task {
        Task {
            task_id: "b4zmc5hrw".to_string(),
            tool_use_id: Some("toolu_1".to_string()),
            description: "Sleep briefly then print finished".to_string(),
            kind: Some(super::super::TaskKind::Command),
            background: true,
            state,
            outcome: outcome.map(str::to_string),
            activity: None,
        }
    }

    #[test]
    fn a_tasks_label_names_its_state_and_its_outcome_when_reported() {
        for (state, label) in [
            (TaskState::Running, "Running"),
            (TaskState::Finished, "Finished"),
            (TaskState::Failed, "Failed"),
            (TaskState::Stopped, "Stopped"),
            (TaskState::EndedWithoutReport, "Ended without a report"),
        ] {
            assert_eq!(task_label(&task(state, None), true), label);
        }
        assert_eq!(
            task_label(&task(TaskState::Finished, Some("exit code 0")), true),
            "Finished · exit code 0"
        );
    }

    #[test]
    fn a_running_task_of_a_cli_that_is_not_running_ended_without_a_report() {
        assert_eq!(
            task_label(&task(TaskState::Running, None), false),
            "Ended without a report"
        );
        assert_eq!(task_label(&task(TaskState::Failed, None), false), "Failed");
    }

    fn summary(tool_calls: u64) -> super::super::ThreadSummary {
        super::super::ThreadSummary {
            parent: "toolu_1".to_string(),
            first_seq: 33,
            messages: 4,
            tool_calls,
            latest_tool: Some("Read".to_string()),
        }
    }

    #[test]
    fn a_running_threads_summary_shows_its_tool_calls_and_activity() {
        let summary = summary(3);
        let view = ThreadView {
            summary: &summary,
            state: SubagentState::Running,
            activity: Some("Reading notes.txt".to_string()),
        };
        assert_eq!(
            thread_summary_line(&view),
            "Subagent · running · 3 tool calls · Reading notes.txt"
        );
    }

    #[test]
    fn an_ended_threads_summary_leaves_out_the_activity() {
        let summary = summary(1);
        for (state, words) in [
            (SubagentState::Finished, "finished"),
            (SubagentState::Failed, "failed"),
            (SubagentState::Stopped, "stopped"),
            (SubagentState::EndedWithoutResult, "ended without a result"),
        ] {
            let view = ThreadView {
                summary: &summary,
                state,
                activity: Some("Reading notes.txt".to_string()),
            };
            assert_eq!(
                thread_summary_line(&view),
                format!("Subagent · {words} · 1 tool call")
            );
        }
    }

    #[test]
    fn a_running_thread_without_tool_calls_or_activity_says_so_plainly() {
        let summary = summary(0);
        let view = ThreadView {
            summary: &summary,
            state: SubagentState::Running,
            activity: None,
        };
        assert_eq!(
            thread_summary_line(&view),
            "Subagent · running · 0 tool calls"
        );
    }
}
