//! Feature 944 — the session insights model over the recorded stream-json fixtures and synthetic
//! events (contracts/insights-model.md: guarantees I1–I6).

use micold_core::structured::insights::{
    ContextReading, ContextState, Owner, SessionInsights, SubagentState, Task, TaskKind, TaskState,
};
use micold_core::structured::transcript::{ToolStatus, Transcript};
use micold_core::structured::{
    parse_line, ContentBlock, SequencedEvent, StructuredEvent, MAX_EVENTS,
};
use serde_json::{json, Value};
use std::collections::HashMap;

#[path = "support/stream_json.rs"]
mod stream_json;
use stream_json::{events_of, lines_of, tool_use, user_text, with_parent, RECORDED, RECORDED_944};

/// Every fixture recorded from a real `claude`.
fn recorded() -> impl Iterator<Item = &'static str> {
    RECORDED.into_iter().chain(RECORDED_944)
}

/// Insights built from `events` in one history message after `omitted` earlier events.
fn built(omitted: u64, events: &[SequencedEvent]) -> SessionInsights {
    let mut insights = SessionInsights::new();
    insights.apply(omitted, events, false);
    insights
}

/// `events` numbered from `from` in order.
fn numbered(events: impl IntoIterator<Item = StructuredEvent>, from: u64) -> Vec<SequencedEvent> {
    (from..)
        .zip(events)
        .map(|(seq, event)| SequencedEvent { seq, event })
        .collect()
}

fn fixture(name: &str) -> Vec<SequencedEvent> {
    events_of(&lines_of(name))
}

/// The fixtures' events one after the other, numbered from `0`, as one session's history.
fn joined(names: &[&str]) -> Vec<SequencedEvent> {
    numbered(
        names.iter().flat_map(|name| fixture(name)).map(|e| e.event),
        0,
    )
}

/// `count` user messages numbered from `from`.
fn fillers(from: u64, count: usize) -> Vec<SequencedEvent> {
    numbered((0..count).map(|_| user_text("filler")), from)
}

fn line(value: Value) -> StructuredEvent {
    parse_line(&value.to_string())
}

fn raw_of(event: &StructuredEvent) -> Option<Value> {
    match event {
        StructuredEvent::Assistant { raw, .. }
        | StructuredEvent::Result { raw, .. }
        | StructuredEvent::Unrecognised { raw } => serde_json::from_str(raw).ok(),
        _ => None,
    }
}

/// A main-scope (or, with `parent`, a subagent's) model response reporting `usage`.
fn response(model: &str, usage: Value, parent: Option<&str>) -> StructuredEvent {
    line(json!({
        "type": "assistant",
        "message": {
            "model": model, "id": "msg_x", "type": "message", "role": "assistant",
            "content": [{ "type": "text", "text": "ok" }],
            "usage": usage,
        },
        "parent_tool_use_id": parent,
        "session_id": "<SESSION_ID>",
    }))
}

fn usage(input: u64, cache_creation: u64, cache_read: u64, output: u64) -> Value {
    json!({
        "input_tokens": input,
        "cache_creation_input_tokens": cache_creation,
        "cache_read_input_tokens": cache_read,
        "output_tokens": output,
    })
}

/// One `modelUsage` entry.
fn model_usage(input: u64, output: u64, cost: f64, window: u64) -> Value {
    json!({
        "inputTokens": input, "outputTokens": output,
        "cacheReadInputTokens": 0, "cacheCreationInputTokens": 0,
        "costUSD": cost, "contextWindow": window,
    })
}

/// A turn result with these running figures; `usage` `None` leaves the field out.
fn result(cost_cum: f64, usage: Option<Value>, models: Value) -> StructuredEvent {
    let mut value = json!({
        "type": "result", "subtype": "success", "is_error": false, "num_turns": 1,
        "result": "ok", "session_id": "<SESSION_ID>",
        "total_cost_usd": cost_cum, "modelUsage": models,
    });
    if let Some(usage) = usage {
        value["usage"] = usage;
    }
    line(value)
}

fn autocompact(value: Value) -> StructuredEvent {
    line(json!({ "type": "autocompact_state", "value": value, "session_id": "<SESSION_ID>" }))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn result_seqs(events: &[SequencedEvent]) -> Vec<u64> {
    events
        .iter()
        .filter(|e| matches!(e.event, StructuredEvent::Result { .. }))
        .map(|e| e.seq)
        .collect()
}

// ---------------------------------------------------------------------------------------
// T007 — I1: incremental, yet equal to a fresh build (FR-023).

#[test]
fn every_fixture_applied_in_any_cut_equals_one_history_message() {
    for name in recorded().chain(["insights-tolerance"]) {
        let events = fixture(name);
        let fresh = built(0, &events);

        let mut one_by_one = SessionInsights::new();
        one_by_one.apply(0, &[], false);
        for event in &events {
            one_by_one.apply(0, std::slice::from_ref(event), true);
        }
        assert_eq!(one_by_one, fresh, "{name}: one event per live message");

        let mut chunked = SessionInsights::new();
        let (history, rest) = events.split_at(events.len() / 3);
        chunked.apply(0, history, false);
        for chunk in rest.chunks(3) {
            chunked.apply(0, chunk, true);
        }
        assert_eq!(chunked, fresh, "{name}: history, then chunks of 3");

        let mut continued = SessionInsights::new();
        let (first, second) = events.split_at(events.len() / 2);
        continued.apply(0, first, false);
        continued.apply(0, second, false);
        assert_eq!(continued, fresh, "{name}: a history continued");
    }
}

#[test]
fn a_history_message_that_does_not_continue_replaces_the_state() {
    let mut insights = built(0, &fixture("tool-turn"));
    let other = fixture("text-turn");
    insights.apply(5, &other, false);
    assert_eq!(insights, built(5, &other));
    assert_eq!(insights.omitted(), 5);
}

#[test]
fn past_the_bound_the_oldest_leave_and_the_value_equals_a_fresh_build() {
    for name in recorded() {
        let mut events = fixture(name);
        let len = events.len();
        events.extend(fillers(len as u64, MAX_EVENTS));
        let mut insights = SessionInsights::new();
        insights.apply(3, &events[..len], false);
        for chunk in events[len..].chunks(700) {
            insights.apply(0, chunk, true);
        }
        let dropped = events.len() - MAX_EVENTS;
        assert_eq!(insights.held(), MAX_EVENTS, "{name}");
        assert_eq!(insights.omitted(), 3 + dropped as u64, "{name}");
        assert_eq!(
            insights,
            built(3 + dropped as u64, &events[dropped..]),
            "{name}: equal to a fresh build of the held events"
        );
    }
}

#[test]
fn a_bound_crossed_inside_one_message_equals_a_fresh_build() {
    let mut events = joined(&["text-turn", "resume-init"]);
    let len = events.len();
    events.extend(fillers(len as u64, MAX_EVENTS - 10));
    let mut insights = SessionInsights::new();
    insights.apply(0, &events, false);
    let dropped = events.len() - MAX_EVENTS;
    assert_eq!(insights, built(dropped as u64, &events[dropped..]));
}

#[test]
fn past_a_seq_gap_the_bound_drops_the_events_actually_held_first() {
    let history = fixture("tool-turn");
    let gap = 10_000;
    let mut live = numbered(turns(2).into_iter().map(|e| e.event), gap);
    live.extend(fillers(gap + live.len() as u64, MAX_EVENTS));
    let mut insights = built(0, &history);
    insights.apply(0, &live, true);
    let dropped = history.len() + live.len() - MAX_EVENTS;
    assert_eq!(insights.held(), MAX_EVENTS);
    assert_eq!(insights.turn(gap + 1), None, "the turns past the gap left");
    assert_eq!(
        insights,
        built(dropped as u64, &live[live.len() - MAX_EVENTS..]),
        "equal to a fresh build of the held events"
    );
}

// ---------------------------------------------------------------------------------------
// T009 — context (I2, FR-001–FR-005).

/// The last main-scope response's model and usage sum, and that model's window in the latest
/// result listing it, read straight from the fixture's lines.
fn expected_context(events: &[SequencedEvent]) -> (String, u64, Option<u64>) {
    let (model, used) = events
        .iter()
        .rev()
        .filter_map(|e| match &e.event {
            StructuredEvent::Assistant {
                parent_tool_use_id: None,
                ..
            } => raw_of(&e.event),
            _ => None,
        })
        .map(|v| {
            let usage = &v["message"]["usage"];
            let sum = [
                "input_tokens",
                "cache_creation_input_tokens",
                "cache_read_input_tokens",
                "output_tokens",
            ]
            .iter()
            .map(|k| usage[*k].as_u64().unwrap())
            .sum::<u64>();
            (v["message"]["model"].as_str().unwrap().to_string(), sum)
        })
        .next()
        .expect("a main response");
    let window = events
        .iter()
        .rev()
        .filter(|e| matches!(e.event, StructuredEvent::Result { .. }))
        .find_map(|e| raw_of(&e.event)?["modelUsage"][&model]["contextWindow"].as_u64());
    (model, used, window)
}

#[test]
fn every_fixtures_reading_is_its_last_main_response_and_its_models_window() {
    for name in recorded() {
        let events = fixture(name);
        let (model, used, window) = expected_context(&events);
        let reading = built(0, &events).context().clone();
        assert_eq!(reading.used, Some(used), "{name}");
        assert_eq!(reading.model.as_deref(), Some(model.as_str()), "{name}");
        assert_eq!(reading.window, window, "{name}");
        assert_eq!(reading.threshold, Some(384_000), "{name}");
    }
}

#[test]
fn the_tool_turn_reading_is_measured_with_its_threshold() {
    let events = fixture("tool-turn");
    let insights = built(0, &events);
    let used = insights.context().used.unwrap();
    // The last main response of `tool-turn`: its four usage figures summed.
    let (_, expected, _) = expected_context(&events);
    assert_eq!(used, expected);
    assert_eq!(
        insights.context().state(),
        ContextState::Measured {
            used,
            window: 1_000_000,
            percent: 4,
            threshold: Some(384_000),
            warning: false,
        }
    );
}

#[test]
fn the_first_response_of_tool_turn_sums_its_four_usage_figures() {
    let events = fixture("tool-turn");
    let first = events
        .iter()
        .position(|e| matches!(e.event, StructuredEvent::Assistant { .. }))
        .unwrap();
    let insights = built(0, &events[..=first]);
    assert_eq!(insights.context().used, Some(2 + 8515 + 28624 + 16));
}

#[test]
fn no_main_response_is_no_data() {
    assert_eq!(
        SessionInsights::new().context().state(),
        ContextState::NoData
    );
    let events = fixture("text-turn");
    let first = events
        .iter()
        .position(|e| matches!(e.event, StructuredEvent::Assistant { .. }))
        .unwrap();
    assert_eq!(
        built(0, &events[..first]).context().state(),
        ContextState::NoData
    );
}

#[test]
fn before_the_first_result_the_reading_is_tokens_only() {
    let events = fixture("text-turn");
    let at = result_seqs(&events)[0] as usize;
    let reading = built(0, &events[..at]).context().clone();
    assert_eq!(
        reading.state(),
        ContextState::TokensOnly {
            used: reading.used.unwrap()
        }
    );
}

#[test]
fn a_model_the_results_do_not_list_is_tokens_only() {
    let events = numbered(
        [
            result(
                0.01,
                Some(usage(1, 0, 0, 1)),
                json!({ "a": model_usage(1, 1, 0.01, 1000) }),
            ),
            response("b", usage(10, 0, 0, 5), None),
        ],
        0,
    );
    assert_eq!(
        built(0, &events).context().state(),
        ContextState::TokensOnly { used: 15 }
    );
}

fn reading(used: u64, window: u64, threshold: Option<u64>) -> ContextState {
    ContextReading {
        used: Some(used),
        model: Some("m".into()),
        window: Some(window),
        threshold,
    }
    .state()
}

fn percent_and_warning(state: ContextState) -> (u64, bool) {
    match state {
        ContextState::Measured {
            percent, warning, ..
        } => (percent, warning),
        other => panic!("not measured: {other:?}"),
    }
}

#[test]
fn the_percent_is_whole_and_rounded_half_up() {
    assert_eq!(percent_and_warning(reading(4, 1000, None)).0, 0);
    assert_eq!(percent_and_warning(reading(5, 1000, None)).0, 1);
    assert_eq!(percent_and_warning(reading(14, 1000, None)).0, 1);
    assert_eq!(percent_and_warning(reading(15, 1000, None)).0, 2);
    assert_eq!(percent_and_warning(reading(1000, 1000, None)).0, 100);
}

#[test]
fn the_warning_starts_at_the_threshold_or_at_90_percent_without_one() {
    assert!(percent_and_warning(reading(384_000, 1_000_000, Some(384_000))).1);
    assert!(!percent_and_warning(reading(383_999, 1_000_000, Some(384_000))).1);
    assert!(
        !percent_and_warning(reading(950_000, 1_000_000, Some(960_000))).1,
        "with a threshold, only the threshold counts"
    );
    assert!(percent_and_warning(reading(900, 1000, None)).1);
    assert!(!percent_and_warning(reading(899, 1000, None)).1);
}

#[test]
fn the_threshold_is_read_only_when_compaction_is_enabled() {
    let threshold = |value: Value| {
        built(0, &numbered([autocompact(value)], 0))
            .context()
            .threshold
    };
    assert_eq!(
        threshold(json!({ "enabled": true, "threshold": 384_000 })),
        Some(384_000)
    );
    assert_eq!(
        threshold(json!({ "enabled": false, "threshold": 384_000 })),
        None
    );
    assert_eq!(
        threshold(json!({ "enabled": true, "threshold": "384000" })),
        None
    );
    assert_eq!(threshold(json!({ "threshold": 384_000 })), None);
    assert_eq!(threshold(json!(null)), None);
    assert_eq!(SessionInsights::new().context().threshold, None);
}

#[test]
fn the_reading_follows_each_main_response_both_ways() {
    let mut insights = SessionInsights::new();
    insights.apply(
        0,
        &numbered(
            [result(
                0.01,
                Some(usage(1, 0, 0, 1)),
                json!({ "m": model_usage(1, 1, 0.01, 1000) }),
            )],
            0,
        ),
        false,
    );
    insights.apply(
        0,
        &numbered([response("m", usage(100, 0, 0, 0), None)], 1),
        true,
    );
    assert_eq!(insights.context().used, Some(100));
    insights.apply(
        0,
        &numbered([response("m", usage(500, 0, 300, 20), None)], 2),
        true,
    );
    assert_eq!(insights.context().used, Some(820), "it grew");
    insights.apply(
        0,
        &numbered([response("m", usage(40, 0, 0, 2), None)], 3),
        true,
    );
    assert_eq!(insights.context().used, Some(42), "compacted: it shrank");
    assert_eq!(
        percent_and_warning(insights.context().state()).0,
        4,
        "measured at once against the window"
    );
}

#[test]
fn a_subagents_response_never_moves_the_reading() {
    let mut insights = built(0, &numbered([response("m", usage(10, 0, 0, 0), None)], 0));
    insights.apply(
        0,
        &numbered([response("m", usage(90_000, 0, 0, 0), Some("toolu_1"))], 1),
        true,
    );
    assert_eq!(insights.context().used, Some(10));
    for name in ["subagent-turn", "nested-subagent", "background-subagent"] {
        let events = fixture(name);
        assert_eq!(
            built(0, &events).context().used,
            Some(expected_context(&events).1),
            "{name}"
        );
    }
}

// ---------------------------------------------------------------------------------------
// T011 — turn usage and totals (I2, I3, FR-006–FR-009).

#[test]
fn each_turns_tokens_are_its_results_usage() {
    for name in recorded() {
        let events = fixture(name);
        let insights = built(0, &events);
        for event in &events {
            let StructuredEvent::Result { usage, .. } = &event.event else {
                continue;
            };
            let usage = usage.as_ref().expect("a recorded result reports usage");
            let tokens = insights.turn(event.seq).unwrap().tokens.unwrap();
            assert_eq!(
                (
                    tokens.input,
                    tokens.output,
                    tokens.cache_read,
                    tokens.cache_creation
                ),
                (
                    usage.input_tokens,
                    usage.output_tokens,
                    usage.cache_read_input_tokens,
                    usage.cache_creation_input_tokens
                ),
                "{name} seq {}",
                event.seq
            );
        }
    }
}

#[test]
fn a_resumed_turns_cost_and_model_figures_are_the_difference_of_the_running_figures() {
    let events = joined(&["text-turn", "resume-init"]);
    let insights = built(0, &events);
    let seqs = result_seqs(&events);
    let first = insights.turn(seqs[0]).unwrap();
    assert!(
        close(first.cost_usd.unwrap(), 0.0371549),
        "nothing before it, nothing omitted"
    );
    let second = insights.turn(seqs[1]).unwrap();
    assert!(!second.restarted);
    assert!(close(second.cost_usd.unwrap(), 0.0954221 - 0.0371549));
    let models = second.models.as_ref().unwrap();
    let sonnet = &models
        .iter()
        .find(|(m, _)| m == "claude-sonnet-5-5")
        .unwrap()
        .1;
    assert_eq!(sonnet.output, 20 - 14);
    assert!(
        models.iter().all(|(m, _)| m != "claude-haiku-5-5"),
        "a model the turn did not use is not listed"
    );
    let totals = insights.totals();
    assert_eq!((totals.turns_counted, totals.turns_left_out), (2, 0));
    assert!(close(totals.cost_usd, 0.0954221), "no turn counted twice");
    assert!(!totals.partial);
}

/// `n` turns with running cost `k × 0.01` and running model output `k × 10` after turn `k`.
fn turns(n: u64) -> Vec<SequencedEvent> {
    numbered(
        (1..=n).flat_map(|k| {
            [
                response("m", usage(k, 0, 0, 1), None),
                result(
                    k as f64 * 0.01,
                    Some(usage(k, 0, 0, 1)),
                    json!({ "m": model_usage(k * 3, k * 10, k as f64 * 0.01, 1000) }),
                ),
            ]
        }),
        0,
    )
}

#[test]
fn totals_are_the_sum_of_the_counted_turns_for_1_2_and_50_turns() {
    for n in [1u64, 2, 50] {
        let events = turns(n);
        let insights = built(0, &events);
        let totals = insights.totals();
        assert_eq!(totals.turns_counted, n);
        assert!(close(totals.cost_usd, n as f64 * 0.01), "{n} turns");
        assert_eq!(totals.tokens.input, Some((1..=n).sum::<u64>()));
        assert_eq!(totals.tokens.output, Some(n));
        let (model, figures) = &totals.models[0];
        assert_eq!((model.as_str(), figures.output), ("m", n * 10));
        for seq in result_seqs(&events) {
            let turn = insights.turn(seq).unwrap();
            assert!(close(turn.cost_usd.unwrap(), 0.01) && turn.counted);
        }
    }
}

#[test]
fn a_running_figure_that_falls_is_a_restart_taken_as_it_is() {
    let events = numbered(
        [
            result(
                0.5,
                Some(usage(1, 0, 0, 1)),
                json!({ "m": model_usage(9, 90, 0.5, 1000) }),
            ),
            result(
                0.2,
                Some(usage(1, 0, 0, 1)),
                json!({ "m": model_usage(2, 20, 0.2, 1000) }),
            ),
        ],
        0,
    );
    let insights = built(0, &events);
    let turn = insights.turn(1).unwrap();
    assert!(turn.restarted && close(turn.cost_usd.unwrap(), 0.2));
    assert_eq!(turn.models.as_ref().unwrap()[0].1.output, 20);
    assert!(close(insights.totals().cost_usd, 0.7));
}

#[test]
fn a_first_held_result_after_omitted_events_has_no_cost() {
    let events = turns(2);
    let complete = built(0, &events);
    assert!(close(complete.turn(1).unwrap().cost_usd.unwrap(), 0.01));
    let partial = built(7, &events);
    let first = partial.turn(1).unwrap();
    assert_eq!((first.cost_usd, first.models.as_ref()), (None, None));
    assert!(!first.counted);
    assert!(
        partial.turn(3).unwrap().counted,
        "the next one has its previous"
    );
    let totals = partial.totals();
    assert_eq!((totals.turns_counted, totals.turns_left_out), (1, 1));
    assert!(totals.partial);
    assert!(!complete.totals().partial);
}

#[test]
fn evicting_the_previous_result_leaves_the_turn_out() {
    let mut events = turns(1);
    events.extend(fillers(2, 10));
    events.extend(numbered(turns(2).into_iter().map(|e| e.event).skip(2), 12));
    let second = 13;
    let mut insights = built(0, &events);
    assert!(insights.turn(second).unwrap().counted);
    insights.apply(0, &fillers(14, MAX_EVENTS - 12), true);
    assert_eq!(insights.turn(1), None, "the first result left");
    let turn = insights.turn(second).unwrap();
    assert_eq!(turn.cost_usd, None);
    assert!(!turn.counted && insights.totals().partial);
}

#[test]
fn a_result_without_usage_has_no_tokens_and_is_left_out() {
    let events = numbered(
        [
            result(0.01, None, json!({ "m": model_usage(1, 1, 0.01, 1000) })),
            StructuredEvent::TurnEndedWithoutResult,
            result(
                0.02,
                Some(usage(1, 0, 0, 1)),
                json!({ "m": model_usage(2, 2, 0.02, 1000) }),
            ),
        ],
        0,
    );
    let insights = built(0, &events);
    let turn = insights.turn(0).unwrap();
    assert_eq!(turn.tokens, None);
    assert!(!turn.counted);
    let totals = insights.totals();
    assert_eq!((totals.turns_counted, totals.turns_left_out), (1, 2));
    assert!(close(totals.cost_usd, 0.01));
}

#[test]
fn per_model_totals_sum_the_counted_turns() {
    let events = numbered(
        [
            result(
                0.03,
                Some(usage(1, 0, 0, 1)),
                json!({ "a": model_usage(1, 10, 0.01, 1000), "b": model_usage(2, 20, 0.02, 1000) }),
            ),
            result(
                0.04,
                None,
                json!({ "a": model_usage(5, 50, 0.02, 1000), "b": model_usage(2, 20, 0.02, 1000) }),
            ),
            result(
                0.07,
                Some(usage(1, 0, 0, 1)),
                json!({ "a": model_usage(5, 50, 0.02, 1000), "b": model_usage(6, 60, 0.05, 1000) }),
            ),
        ],
        0,
    );
    let totals = built(0, &events).totals().clone();
    let of = |name: &str| totals.models.iter().find(|(m, _)| m == name).unwrap().1;
    assert_eq!(
        (of("a").output, of("b").output),
        (10, 60),
        "the left-out turn's a: 40 is not in"
    );
    assert!(close(of("b").cost_usd, 0.05));
}

// ---------------------------------------------------------------------------------------
// T015 — tolerance (I6, SC-006, FR-024).

#[test]
fn malformed_figures_are_left_out_and_the_well_formed_ones_still_count() {
    let events = fixture("insights-tolerance");
    let insights = built(0, &events);
    let reading = insights.context();
    assert_eq!(
        reading.used,
        Some(3 + 100 + 1000 + 7),
        "the one well-formed response"
    );
    assert_eq!(
        reading.window,
        Some(1_000_000),
        "read apart from the entry's bad figure"
    );
    assert_eq!(
        reading.threshold, None,
        "malformed, then absent, then disabled"
    );

    let seqs = result_seqs(&events);
    assert_eq!(
        seqs.len(),
        4,
        "the result with a string token count is not a result"
    );
    let first = insights.turn(seqs[0]).unwrap();
    let models = first.models.as_ref().unwrap();
    assert_eq!(
        models.iter().map(|(m, _)| m.as_str()).collect::<Vec<_>>(),
        ["claude-haiku-5-5"],
        "the entry with a string figure and the number are skipped"
    );
    assert!(first.counted && close(first.cost_usd.unwrap(), 0.02));
    let no_usage = insights.turn(seqs[1]).unwrap();
    assert_eq!((no_usage.tokens, no_usage.models.as_ref()), (None, None));
    assert!(!no_usage.counted);
    let list_models = insights.turn(seqs[2]).unwrap();
    assert_eq!(list_models.models, None);
    assert!(list_models.counted && close(list_models.cost_usd.unwrap(), 0.02));
    let negative_cost = insights.turn(seqs[3]).unwrap();
    assert_eq!(negative_cost.cost_usd, None);
    assert!(!negative_cost.counted);

    let totals = insights.totals();
    assert_eq!((totals.turns_counted, totals.turns_left_out), (2, 2));
    assert!(close(totals.cost_usd, 0.04));
}

#[test]
fn a_malformed_response_usage_leaves_the_reading_without_a_figure() {
    for bad in [
        json!({ "input_tokens": "2", "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0, "output_tokens": 1 }),
        json!({ "input_tokens": 2, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0, "output_tokens": -1 }),
        json!({ "input_tokens": 2 }),
        json!("lots"),
    ] {
        let events = numbered(
            [
                response("m", usage(10, 0, 0, 0), None),
                response("m", bad.clone(), None),
            ],
            0,
        );
        let used = built(0, &events).context().used;
        if bad.is_object() {
            assert_eq!(used, None, "{bad}");
        } else {
            assert_eq!(used, Some(10), "{bad}: no usage object, no reading");
        }
    }
}

// ---------------------------------------------------------------------------------------
// T023 — I5: background tasks (FR-018, FR-020–FR-022).

/// A `system` task line of `subtype` with `fields`.
fn system(subtype: &str, fields: Value) -> StructuredEvent {
    let mut value = json!({ "type": "system", "subtype": subtype, "session_id": "<SESSION_ID>" });
    for (key, field) in fields.as_object().expect("fields are an object") {
        value[key] = field.clone();
    }
    line(value)
}

fn task_started(
    task_id: &str,
    tool_use_id: &str,
    task_type: &str,
    background: bool,
) -> StructuredEvent {
    system(
        "task_started",
        json!({
            "task_id": task_id, "tool_use_id": tool_use_id, "description": format!("task {task_id}"),
            "task_type": task_type, "is_backgrounded": background,
        }),
    )
}

fn live_set(task_ids: &[&str]) -> StructuredEvent {
    let tasks: Vec<Value> = task_ids
        .iter()
        .map(|id| json!({ "task_id": id, "task_type": "local_bash", "description": format!("live {id}") }))
        .collect();
    system("background_tasks_changed", json!({ "tasks": tasks }))
}

/// The CLI's `init` line when its process starts, as `text-turn` recorded it.
fn init() -> StructuredEvent {
    fixture("text-turn")
        .into_iter()
        .map(|e| e.event)
        .find(|e| matches!(e, StructuredEvent::Init { .. }))
        .expect("text-turn starts with an init")
}

/// The `init` line the same CLI process writes at the start of a later turn, as
/// `background-command` recorded it (no `startup_timing`).
fn turn_init() -> StructuredEvent {
    fixture("background-command")
        .into_iter()
        .map(|e| e.event)
        .filter(|e| matches!(e, StructuredEvent::Init { .. }))
        .nth(1)
        .expect("background-command's second turn starts with an init")
}

/// The task `task_id` of insights built from `events`.
fn task_of<'a>(insights: &'a SessionInsights, task_id: &str) -> &'a Task {
    insights
        .tasks()
        .find(|t| t.task_id == task_id)
        .unwrap_or_else(|| panic!("task {task_id} is listed"))
}

/// The seq of the fixture's first `system` line of `subtype`.
fn seq_of_subtype(events: &[SequencedEvent], subtype: &str) -> u64 {
    events
        .iter()
        .find(|e| raw_of(&e.event).is_some_and(|v| v["subtype"] == subtype))
        .map(|e| e.seq)
        .unwrap_or_else(|| panic!("a {subtype} line"))
}

/// The fixture's first line of `subtype`, as JSON.
fn first_line_of(events: &[SequencedEvent], subtype: &str) -> Value {
    raw_of(&events[seq_of_subtype(events, subtype) as usize].event).expect("JSON")
}

#[test]
fn a_background_command_runs_then_finishes_with_its_summary() {
    let events = fixture("background-command");
    let started = first_line_of(&events, "task_started");
    let notification = first_line_of(&events, "task_notification");
    let id = started["task_id"].as_str().unwrap();

    let until_started = seq_of_subtype(&events, "task_started") as usize + 1;
    let running = built(0, &events[..until_started]);
    let task = task_of(&running, id);
    assert_eq!(task.state, TaskState::Running);
    assert_eq!(task.kind, Some(TaskKind::Command));
    assert!(task.background);
    assert_eq!(task.description, started["description"].as_str().unwrap());
    assert_eq!(task.tool_use_id.as_deref(), started["tool_use_id"].as_str());
    assert_eq!(task.outcome, None);
    assert_eq!(running.running_background(true), 1);
    assert_eq!(running.background_tasks().count(), 1);

    let finished = built(0, &events);
    let task = task_of(&finished, id);
    assert_eq!(task.state, TaskState::Finished);
    assert_eq!(task.outcome.as_deref(), notification["summary"].as_str());
    assert_eq!(finished.running_background(true), 0);
    assert_eq!(
        finished.tasks().count(),
        1,
        "one task, however many lines name it"
    );
}

#[test]
fn a_background_subagent_is_a_subagent_background_task() {
    let events = fixture("background-subagent");
    let started = first_line_of(&events, "task_started");
    let insights = built(0, &events);
    let task = task_of(&insights, started["task_id"].as_str().unwrap());
    assert_eq!(task.kind, Some(TaskKind::Subagent));
    assert!(task.background);
    assert_eq!(task.state, TaskState::Finished);
    assert_eq!(insights.background_tasks().count(), 1);
}

#[test]
fn the_call_that_started_a_task_finds_it() {
    for name in ["background-command", "background-subagent", "subagent-turn"] {
        let events = fixture(name);
        let started = first_line_of(&events, "task_started");
        let insights = built(0, &events);
        let tool_use_id = started["tool_use_id"].as_str().unwrap();
        assert_eq!(
            insights
                .task_for_tool(tool_use_id)
                .map(|t| t.task_id.as_str()),
            started["task_id"].as_str(),
            "{name}"
        );
    }
    assert!(built(0, &fixture("background-command"))
        .task_for_tool("toolu_none")
        .is_none());
}

#[test]
fn an_init_ends_every_running_task_without_a_report() {
    let events = numbered(
        [
            task_started("bg", "toolu_1", "local_bash", true),
            task_started("fg", "toolu_2", "local_agent", false),
            init(),
        ],
        0,
    );
    let insights = built(0, &events);
    assert_eq!(
        task_of(&insights, "bg").state,
        TaskState::EndedWithoutReport
    );
    assert_eq!(
        task_of(&insights, "fg").state,
        TaskState::EndedWithoutReport
    );
    assert_eq!(insights.running_background(true), 0);
}

#[test]
fn a_later_turns_init_in_the_same_process_leaves_tasks_running() {
    let events = numbered(
        [
            task_started("bg", "toolu_1", "local_bash", true),
            turn_init(),
        ],
        0,
    );
    let insights = built(0, &events);
    assert_eq!(task_of(&insights, "bg").state, TaskState::Running);
    assert_eq!(insights.running_background(true), 1);
}

#[test]
fn a_report_after_an_inferred_end_sets_the_reported_state() {
    for report in [
        system(
            "task_updated",
            json!({ "task_id": "bg", "patch": { "status": "completed" } }),
        ),
        system(
            "task_notification",
            json!({ "task_id": "bg", "status": "completed", "summary": "done" }),
        ),
    ] {
        let events = numbered(
            [
                task_started("bg", "toolu_1", "local_bash", true),
                live_set(&[]),
                report,
            ],
            0,
        );
        assert_eq!(task_of(&built(0, &events), "bg").state, TaskState::Finished);
    }
    let events = numbered(
        [
            task_started("bg", "toolu_1", "local_bash", true),
            system(
                "task_notification",
                json!({ "task_id": "bg", "status": "failed" }),
            ),
            system(
                "task_notification",
                json!({ "task_id": "bg", "status": "completed" }),
            ),
        ],
        0,
    );
    assert_eq!(
        task_of(&built(0, &events), "bg").state,
        TaskState::Failed,
        "a reported end stays"
    );
}

#[test]
fn no_background_task_counts_as_running_while_the_cli_is_not_running() {
    let insights = built(
        0,
        &numbered([task_started("bg", "toolu_1", "local_bash", true)], 0),
    );
    assert_eq!(insights.running_background(true), 1);
    assert_eq!(insights.running_background(false), 0);
}

#[test]
fn a_live_set_without_a_running_background_task_ends_it() {
    let events = numbered(
        [
            task_started("bg", "toolu_1", "local_bash", true),
            task_started("kept", "toolu_2", "local_bash", true),
            task_started("fg", "toolu_3", "local_agent", false),
            live_set(&["kept"]),
        ],
        0,
    );
    let insights = built(0, &events);
    assert_eq!(
        task_of(&insights, "bg").state,
        TaskState::EndedWithoutReport
    );
    assert_eq!(task_of(&insights, "kept").state, TaskState::Running);
    assert_eq!(
        task_of(&insights, "fg").state,
        TaskState::Running,
        "not a background task"
    );
    assert_eq!(insights.running_background(true), 1);
}

#[test]
fn a_live_set_lists_a_task_not_seen_before() {
    let insights = built(0, &numbered([live_set(&["new"])], 0));
    let task = task_of(&insights, "new");
    assert_eq!(task.state, TaskState::Running);
    assert!(task.background);
    assert_eq!(task.kind, Some(TaskKind::Command));
    assert_eq!(task.description, "live new");
}

#[test]
fn a_turn_ended_without_result_ends_running_foreground_tasks_only() {
    let events = numbered(
        [
            task_started("bg", "toolu_1", "local_bash", true),
            task_started("fg", "toolu_2", "local_agent", false),
            StructuredEvent::TurnEndedWithoutResult,
        ],
        0,
    );
    let insights = built(0, &events);
    assert_eq!(
        task_of(&insights, "fg").state,
        TaskState::EndedWithoutReport
    );
    assert_eq!(task_of(&insights, "bg").state, TaskState::Running);
}

#[test]
fn a_report_of_an_unknown_task_lists_it() {
    let notification = system(
        "task_notification",
        json!({ "task_id": "lost", "tool_use_id": "toolu_9", "status": "failed", "summary": "Command failed" }),
    );
    let insights = built(0, &numbered([notification], 0));
    let task = task_of(&insights, "lost");
    assert_eq!(task.state, TaskState::Failed);
    assert!(task.background);
    assert_eq!(task.outcome.as_deref(), Some("Command failed"));
    assert_eq!(task.description, "Command failed");
    assert_eq!(task.tool_use_id.as_deref(), Some("toolu_9"));
    assert_eq!(task.kind, None);
}

#[test]
fn each_reported_status_sets_its_state() {
    for (subtype, status, state) in [
        ("task_updated", "completed", TaskState::Finished),
        ("task_updated", "failed", TaskState::Failed),
        ("task_updated", "killed", TaskState::Stopped),
        ("task_notification", "completed", TaskState::Finished),
        ("task_notification", "failed", TaskState::Failed),
        ("task_notification", "stopped", TaskState::Stopped),
    ] {
        let report = if subtype == "task_updated" {
            system(
                subtype,
                json!({ "task_id": "t", "patch": { "status": status } }),
            )
        } else {
            system(subtype, json!({ "task_id": "t", "status": status }))
        };
        let events = numbered(
            [task_started("t", "toolu_1", "local_bash", true), report],
            0,
        );
        assert_eq!(
            task_of(&built(0, &events), "t").state,
            state,
            "{subtype} {status}"
        );
    }
}

#[test]
fn a_later_start_of_the_same_task_reopens_it() {
    let events = numbered(
        [
            task_started("t", "toolu_1", "local_agent", true),
            system(
                "task_notification",
                json!({ "task_id": "t", "status": "completed", "summary": "done" }),
            ),
            task_started("t", "toolu_1", "local_agent", true),
        ],
        0,
    );
    let insights = built(0, &events);
    assert_eq!(task_of(&insights, "t").state, TaskState::Running);
    assert_eq!(insights.tasks().count(), 1);
}

#[test]
fn a_tasks_activity_is_its_progress_summary_else_description_else_last_tool() {
    for (progress, activity) in [
        (
            json!({ "summary": "Summing up", "description": "Reading", "last_tool_name": "Read" }),
            "Summing up",
        ),
        (
            json!({ "description": "Reading", "last_tool_name": "Read" }),
            "Reading",
        ),
        (json!({ "last_tool_name": "Read" }), "Read"),
    ] {
        let mut fields = progress.clone();
        fields["task_id"] = json!("t");
        let events = numbered(
            [
                task_started("t", "toolu_1", "local_agent", false),
                system("task_progress", fields),
            ],
            0,
        );
        assert_eq!(
            task_of(&built(0, &events), "t").activity.as_deref(),
            Some(activity),
            "{progress}"
        );
    }
    let insights = built(0, &fixture("subagent-turn"));
    let task = insights.tasks().next().expect("the subagent's task");
    assert_eq!(task.activity.as_deref(), Some("Reading notes.txt"));
}

#[test]
fn a_running_task_shows_ended_while_the_cli_is_not_running_and_the_table_keeps_it() {
    let events = numbered([task_started("t", "toolu_1", "local_bash", true)], 0);
    let insights = built(0, &events);
    let task = task_of(&insights, "t");
    assert_eq!(task.display_state(false), TaskState::EndedWithoutReport);
    assert_eq!(task.display_state(true), TaskState::Running);
    assert_eq!(task.state, TaskState::Running, "the table is unchanged");
    assert_eq!(insights, built(0, &events));
    let finished = Task {
        state: TaskState::Finished,
        ..task.clone()
    };
    assert_eq!(finished.display_state(false), TaskState::Finished);
}

#[test]
fn a_session_without_background_tasks_lists_none() {
    for name in ["text-turn", "tool-turn", "subagent-turn", "nested-subagent"] {
        let insights = built(0, &fixture(name));
        assert_eq!(insights.background_tasks().count(), 0, "{name}");
        assert_eq!(insights.running_background(true), 0, "{name}");
    }
}

#[test]
fn malformed_task_lines_are_skipped() {
    let insights = built(0, &fixture("insights-tolerance"));
    assert_eq!(insights.tasks().count(), 0);
}

// ---------------------------------------------------------------------------------------
// T024 — I4: subagent threads (SC-003, FR-014–FR-018).

/// Per parent tool use id, read straight from the events: the parented messages, the tool calls
/// they hold and the latest tool's name.
fn expected_threads(events: &[SequencedEvent]) -> HashMap<String, (u64, u64, Option<String>)> {
    let mut threads: HashMap<String, (u64, u64, Option<String>)> = HashMap::new();
    for e in events {
        let (parent, blocks) = match &e.event {
            StructuredEvent::Assistant {
                parent_tool_use_id: Some(p),
                blocks,
                ..
            }
            | StructuredEvent::User {
                parent_tool_use_id: Some(p),
                blocks,
                ..
            } => (p, blocks),
            _ => continue,
        };
        let entry = threads.entry(parent.clone()).or_default();
        entry.0 += 1;
        for block in blocks {
            if let ContentBlock::ToolUse { name, .. } = block {
                entry.1 += 1;
                entry.2 = Some(name.clone());
            }
        }
    }
    threads
}

fn main_of(omitted: u64, events: &[SequencedEvent]) -> Transcript {
    let mut main = Transcript::new();
    main.apply(omitted, events, false);
    main
}

/// The id of the first `Agent` call that `parent`'s thread (or, with `None`, the main scope)
/// holds.
fn agent_call(events: &[SequencedEvent], parent: Option<&str>) -> String {
    events
        .iter()
        .find_map(|e| match &e.event {
            StructuredEvent::Assistant {
                parent_tool_use_id,
                blocks,
                ..
            } if parent_tool_use_id.as_deref() == parent => blocks.iter().find_map(|b| match b {
                ContentBlock::ToolUse { id, name, .. } if name == "Agent" => Some(id.clone()),
                _ => None,
            }),
            _ => None,
        })
        .expect("an Agent call")
}

#[test]
fn every_parented_message_is_counted_in_its_parents_thread_and_no_other() {
    for name in ["subagent-turn", "nested-subagent", "background-subagent"] {
        let events = fixture(name);
        let expected = expected_threads(&events);
        assert!(!expected.is_empty(), "{name} has a subagent");
        let insights = built(0, &events);
        for (parent, (messages, tool_calls, latest)) in &expected {
            let thread = insights
                .thread(parent)
                .unwrap_or_else(|| panic!("{name}: a thread for {parent}"));
            assert_eq!(&thread.parent, parent);
            assert_eq!(thread.messages, *messages, "{name} {parent}: messages");
            assert_eq!(
                thread.tool_calls, *tool_calls,
                "{name} {parent}: tool calls"
            );
            assert_eq!(&thread.latest_tool, latest, "{name} {parent}: latest tool");
        }
        let main = main_of(0, &events);
        let mut every: Vec<_> = insights.threads_owned_by(&Owner::Main, &main);
        let mut queue: Vec<String> = every.iter().map(|t| t.parent.clone()).collect();
        while let Some(parent) = queue.pop() {
            let inner = insights.threads_owned_by(&Owner::Thread(parent), &main);
            queue.extend(inner.iter().map(|t| t.parent.clone()));
            every.extend(inner);
        }
        assert_eq!(
            every.len(),
            expected.len(),
            "{name}: every thread is placed once"
        );
        assert!(insights.unknown_threads(&main).is_empty(), "{name}");
    }
}

#[test]
fn a_tool_call_held_again_is_counted_once() {
    let call = with_parent(tool_use("toolu_r", "Read", "{}"), "toolu_a");
    let insights = built(0, &numbered([call.clone(), call], 0));
    let thread = insights.thread("toolu_a").unwrap();
    assert_eq!((thread.messages, thread.tool_calls), (2, 1));
}

#[test]
fn a_thread_is_placed_under_the_card_of_its_parent_at_every_depth() {
    let events = fixture("nested-subagent");
    let main = main_of(0, &events);
    let insights = built(0, &events);
    let outer = agent_call(&events, None);
    let inner = agent_call(&events, Some(&outer));
    assert_eq!(insights.owner(&outer, &main), Owner::Main);
    assert_eq!(insights.owner(&inner, &main), Owner::Thread(outer.clone()));
    let owned: Vec<&str> = insights
        .threads_owned_by(&Owner::Thread(outer.clone()), &main)
        .iter()
        .map(|t| t.parent.as_str())
        .collect();
    assert_eq!(owned, [inner.as_str()]);
    let at_main: Vec<&str> = insights
        .threads_owned_by(&Owner::Main, &main)
        .iter()
        .map(|t| t.parent.as_str())
        .collect();
    assert_eq!(at_main, [outer.as_str()]);
}

#[test]
fn a_thread_whose_parent_is_held_nowhere_is_unknown() {
    let mut events = fixture("subagent-turn");
    let next = events.len() as u64;
    events.extend(numbered(
        [with_parent(user_text("orphan"), "toolu_never")],
        next,
    ));
    let main = main_of(0, &events);
    let insights = built(0, &events);
    assert_eq!(insights.owner("toolu_never", &main), Owner::Unknown);
    let unknown: Vec<&str> = insights
        .unknown_threads(&main)
        .iter()
        .map(|t| t.parent.as_str())
        .collect();
    assert_eq!(unknown, ["toolu_never"]);

    // The parent's card evicted: the history starts after the `Agent` call.
    let events = fixture("nested-subagent");
    let outer = agent_call(&events, None);
    let inner = agent_call(&events, Some(&outer));
    let call_seq = events
        .iter()
        .position(|e| {
            matches!(&e.event, StructuredEvent::Assistant { parent_tool_use_id: None, blocks, .. }
            if blocks.iter().any(|b| matches!(b, ContentBlock::ToolUse { id, .. } if *id == outer)))
        })
        .unwrap();
    let held = &events[call_seq + 1..];
    let (main, insights) = (
        main_of(call_seq as u64 + 1, held),
        built(call_seq as u64 + 1, held),
    );
    assert_eq!(insights.owner(&outer, &main), Owner::Unknown);
    assert_eq!(insights.owner(&inner, &main), Owner::Thread(outer.clone()));
    let unknown: Vec<&str> = insights
        .unknown_threads(&main)
        .iter()
        .map(|t| t.parent.as_str())
        .collect();
    assert_eq!(unknown, [outer.as_str()]);
}

#[test]
fn a_threads_state_comes_from_its_subagent_task_else_from_its_card() {
    let events = fixture("subagent-turn");
    let parent = agent_call(&events, None);
    let insights = built(0, &events);
    let view = insights
        .thread_view(&parent, Some(ToolStatus::Running))
        .unwrap();
    assert_eq!(view.state, SubagentState::Finished, "the task's state wins");
    assert_eq!(
        view.activity.as_deref(),
        Some("Reading notes.txt"),
        "the task's activity"
    );

    let until_progress = seq_of_subtype(&events, "task_progress") as usize + 1;
    let running = built(0, &events[..until_progress]);
    assert_eq!(
        running
            .thread_view(&parent, Some(ToolStatus::Done))
            .unwrap()
            .state,
        SubagentState::Running
    );

    let untasked = numbered(
        [
            tool_use("toolu_a", "Agent", "{}"),
            with_parent(tool_use("toolu_r", "Read", "{}"), "toolu_a"),
        ],
        0,
    );
    let insights = built(0, &untasked);
    for (card, state) in [
        (ToolStatus::Running, SubagentState::Running),
        (ToolStatus::Done, SubagentState::Finished),
        (ToolStatus::Failed, SubagentState::Failed),
        (ToolStatus::NoResult, SubagentState::EndedWithoutResult),
    ] {
        let view = insights.thread_view("toolu_a", Some(card)).unwrap();
        assert_eq!(view.state, state, "{card:?}");
        assert_eq!(
            view.activity.as_deref(),
            Some("Read"),
            "the latest tool's name"
        );
    }
    assert!(insights.thread_view("toolu_none", None).is_none());
}

#[test]
fn a_subagent_task_that_ended_without_a_report_ended_without_a_result() {
    let events = fixture("subagent-turn");
    let parent = agent_call(&events, None);
    let until_progress = seq_of_subtype(&events, "task_progress") as usize + 1;
    let mut cut = events[..until_progress].to_vec();
    cut.extend(numbered(
        [StructuredEvent::TurnEndedWithoutResult],
        until_progress as u64,
    ));
    let insights = built(0, &cut);
    assert_eq!(
        insights
            .thread_view(&parent, Some(ToolStatus::NoResult))
            .unwrap()
            .state,
        SubagentState::EndedWithoutResult
    );
}
