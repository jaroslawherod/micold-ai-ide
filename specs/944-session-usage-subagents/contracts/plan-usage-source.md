# Contract: structured sessions as a plan-usage source (amends feature 490 FR-012)

## Input

A line the running Claude Code CLI of a structured session writes, of the documented shape
(`SDKRateLimitEvent`, code.claude.com/docs/en/agent-sdk/typescript#sdkratelimitevent), with the
undocumented `unifiedWindows` the CLI writes (Claude Code 2.1.302, `text-turn.jsonl`):

```json
{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1792363200,
 "unifiedWindows":{"five_hour":{"utilization":0.01,"resetsAt":1792363200},
                   "seven_day":{"utilization":0.1,"resetsAt":1792904400}}}, "...": "..."}
```

## Parse — `micold_core::plan_usage::rate_limit_event::parse(raw, obtained_at)`

- Not JSON, `type` not `rate_limit_event`, no `rate_limit_info.unifiedWindows` object → `None`.
- Each entry, in key order: a usable window when its value is an object with a finite,
  non-negative numeric `utilization` and a `resetsAt` that is absent, `null`, or a time after
  `obtained_at` in range — the rule `status_line::window` applies, through the shared
  `plan_usage::limit_window`. `used_tenths = round(utilization × 1000)`; the key is the window's
  key (`five_hour` → "5-hour", `seven_day` → "Weekly", others by `LimitWindow::name`).
- No usable window → `None`. Top-level `utilization`/`resetsAt` are not read (they name no window).

## Feed (daemon)

1. `StructuredSession::spawn`'s `on_event`, for each `StructuredEvent::Unrecognised { raw }` the
   CLI writes: `parse(raw, unix_secs())`; a reading goes to the service's plan-usage feed with the
   session id. Events of a restored history never reach `on_event` (FR-013).
2. The feed task calls `DaemonState::offer_plan_usage(session, reading)`: switch off → nothing kept
   or shown (FR-011); otherwise `note_plan_usage` (newest wins, repeat suppression; feature 490
   FR-019) and, when kept, `PlanUsageChanged { reading }` to every window (FR-010: within 10 s).
3. An unreadable event: nothing is shown; one debug log line per (session, reason) until that
   session's next accepted reading, as `hooks.rs` logs a rejected status (feature 490 FR-016).
4. The event always stays in the session's event history, whatever the switch (US4 s3).

## Guarantees (tests)

- Core `tests/plan_usage_rate_limit_event.rs`: every recorded fixture's event parses to its two
  windows; malformed shapes (`utilization` a string, negative, beyond `u32` after scaling, missing
  `unifiedWindows`, `resetsAt` in the past) give `None` or drop the window.
- Daemon `tests/plan_usage_structured.rs` (stand-in CLI replaying `text-turn.jsonl`): switch on →
  a test client receives `PlanUsageChanged` with the fixture's windows; switch off → none; a
  restored history containing the event → none (SC-004, FR-013).
- `tests/plan_usage_route.rs`, `plan_usage_setting.rs`, `status_relay.rs` pass unchanged (SC-007).
