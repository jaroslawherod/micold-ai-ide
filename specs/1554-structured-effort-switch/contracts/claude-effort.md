# Contract: what the app sends Claude Code for effort (#1554)

Recorded against Claude Code 2.1.311 ([research.md](../research.md) P-1, P-2). M1 records the
fixtures `effort-change.jsonl` and `effort-launch.jsonl` under
`crates/micold-core/tests/fixtures/stream-json/` as the oracle for the shapes below.

## Launch (both kinds)

| Effective level (`LaunchSpec.effort`) | argv addition |
|---|---|
| `None` (choice `Default`, or a level the model is known not to support, FR-010a) | nothing |
| `Some(level)` | `--effort <level>`, after `--model` when present |

## Change while running: `apply_flag_settings`

```json
{"type":"control_request","request_id":"micold-<n>","request":{"subtype":"apply_flag_settings","settings":{"effortLevel":"high"}}}
{"type":"control_request","request_id":"micold-<n>","request":{"subtype":"apply_flag_settings","settings":{"effortLevel":null}}}
```

- `null` returns to Claude Code's default, also over a launch `--effort` (P-2).
- Answer: `{"type":"control_response","response":{"subtype":"success","request_id":"micold-<n>"}}`
  with no `response` body, or `subtype: "error"` with `error` text (e.g. "apply_flag_settings is not
  supported in this context"). The CLI does **not** validate the level (P-1: `"extreme"` →
  `success`); the app sends only a level `EffortLevel::parse` accepted and the offered list holds.
- Written only while no turn is open; a pick during a turn is written by the turn-end hook, after a
  deferred `set_model` (research R1).

## Read back (not used by this feature)

`{"subtype":"get_settings"}` → `response.applied.effort` is the level in use (`"medium"` when
nothing is set, in P-1). Kept for the follow-up in research R10.

## Reported support

`initialize`'s `response.models[]`: `supportsEffort: bool`, `supportedEffortLevels: [string]`
(recorded in `command-initialize.jsonl`); either absent → support unknown.
