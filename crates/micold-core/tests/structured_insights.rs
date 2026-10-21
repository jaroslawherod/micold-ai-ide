//! Feature 944 — the session insights model over the recorded stream-json fixtures and synthetic
//! events (contracts/insights-model.md: guarantees I1, I2, I3, I6).

use micold_core::structured::insights::{ContextReading, ContextState, SessionInsights};
use micold_core::structured::{parse_line, SequencedEvent, StructuredEvent, MAX_EVENTS};
use serde_json::{json, Value};

#[path = "support/stream_json.rs"]
mod stream_json;
use stream_json::{events_of, lines_of, user_text, RECORDED, RECORDED_944};

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
