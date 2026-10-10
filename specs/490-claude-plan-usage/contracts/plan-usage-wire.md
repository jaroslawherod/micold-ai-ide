# Contract: plan-usage wire and settings

Additive changes to `crates/micold-core/src/protocol/messages.rs`. The schema hash moves, as with
every message change; client and daemon ship together.

## W1. `DaemonMsg::Welcome` gains `plan_usage: Option<UsageReading>`

The stored reading at connect time, after dropping it if every window has passed (research R7).
`None` while the switch is off.

## W2. `DaemonMsg::PlanUsageChanged { reading: Option<UsageReading> }`

Broadcast to every connected client when `note_plan_usage` returns true (research R6), and with
`None` when the switch turns off. Every window shows the same reading (FR-018).

## W3. Settings fields

- `DaemonSettings` gains `plan_usage_enabled: bool` and `plan_usage_warn_percent: u8`.
- `ClientMsg::SettingsSet` gains `plan_usage_enabled: Option<bool>` and
  `plan_usage_warn_percent: Option<u8>`. The daemon clamps the percent (50–100) before storing,
  as it does `long_task_threshold_secs`; the client refuses out-of-range input before sending
  (FR-005).
- A change is persisted and broadcast as `SettingsChanged` (existing), so every window applies it
  without a restart (FR-018). A threshold change needs no new reading: the client recomputes the
  look from the stored reading (US2 s4).

## W4. Examples

```json
{"PlanUsageChanged":{"reading":{"windows":[
  {"key":"five_hour","used_tenths":420,"resets_at":1791552600},
  {"key":"seven_day","used_tenths":180,"resets_at":1791849600}],
  "obtained_at":1791547320}}}
```

`settings_contract_examples.rs` (core tests) gains a stored-settings example with both fields and
one without them (defaults `true`, `80`).
