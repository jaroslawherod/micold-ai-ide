# Contract: Claude Code Hook Receiver

**Feature**: `specs/010-daemon-session-persistence` | **Date**: 2026-07-20

How the daemon derives each session's activity signal (FR-016a–d). This replaces the
output-quiescence mechanism FR-016b currently mandates — see [plan.md](../plan.md) amendment A1.

---

## Why not PTY scraping

Measured against a live `claude` v2.1.215 in a PTY (research R4):

| Mechanism | Result |
|---|---|
| Output quiescence | **Dead.** Idle-at-prompt max gap 6.02 s; *working* on a 25 s tool call max gap 20.50 s. No threshold separates them. |
| OSC 133 semantic prompts | **Zero occurrences.** Open upstream feature request. |
| Bracketed paste toggling | Sent once at startup, never toggled. |
| Terminal bell | All 16 BEL bytes were OSC-title terminators. A `\a` scan is 100% false positives. |
| **OSC 0 title glyph** | **Dead as an idle signal — but valuable for titles. See below.** |
| `tcgetpgrp` on the master | Linux-only; macOS XNU returns `ENOTTY`; no ConPTY equivalent. |
| Process state (`/proc`, `sysinfo`) | `state=S, wchan=ep_poll` identically whether busy or idle. |

The spinner does **not** repaint continuously during tool calls, which is the assumption quiescence
detection would need. Hooks are an authoritative application-level signal instead of a heuristic,
and behave identically on all three platforms — so this mechanism does **not** sit behind the
FR-036 platform abstraction.

---

## OSC 0 title: a real signal, for a different problem

`claude` continuously rewrites the terminal title as `OSC 0 ; <glyph> <session title> BEL`. Measured
against v2.1.216 (raw capture in scratchpad `title/`):

```text
  t=1.257   '0;✳ Claude Code'                              idle at prompt
  t=13.425  '0;⠐ Run sleep command and echo marker'        working, spinner frame
  t=21.089  '0;✳ Run sleep command and echo marker'        STILL mid-`sleep 30`
  t=47.121  '0;⠐ Run sleep command and echo marker'        26.03 s of silence
```

**Two independent conclusions, and they point opposite ways.**

**1. As an activity signal it fails, for exactly the reason quiescence failed.** At t=21.089 the glyph
became `✳` — byte-identical to the idle-at-prompt glyph — while the session was 5 s into a 30 s tool
call, then emitted nothing for 26.03 s. The braille spinner frames (`⠂ ⠐ …`) stop when the agent stops
animating, not when it stops working, and when idle the title is not rewritten at all. So:

- **A spinner frame arriving is positive evidence of `Working`** and MAY be used to corroborate.
- **Absence of spinner frames, and the `✳` glyph, carry no information.** They MUST NOT be used to
  conclude `AwaitingInput` (H1). Over the same interval, hooks fired precisely at the tool
  boundaries — `PreToolUse` at 15.7 s, `PostToolUse` at 45.9 s, `Stop` at 47.7 s — while the title was
  dark. That contrast is the whole argument for hooks.

**2. As a session-title source it is excellent, and should be adopted regardless.** The title text
after the glyph is the agent's own generated session name, pushed the instant it changes. Today the
app rescans the transcript JSONL **every 120 ms on the UI thread** (`src/main.rs:754`) to get the same
string. The daemon already parses every byte through `alacritty_terminal`, which surfaces this as an
`Event::Title` with no extra work — so this replaces a polling loop with an event, and removes the
lossy path-slug computation (`src/provider.rs:361-373`) from the title path entirely.

Strip the leading glyph before display: the set observed is `✳` (U+2733) plus braille spinner frames
(U+2800–U+28FF). Match on the codepoint ranges, not a fixed list — new frames are a cosmetic upstream
change and MUST NOT corrupt a title.

⚠️ Titles are user- and agent-generated text on a PTY. Treat as untrusted: bound the length, strip
control characters, and never interpret as markup.

---

## Configuration

The daemon writes a per-session settings file and launches `claude --settings <file>`, so **user
configuration is never modified**.

```jsonc
{
  "hooks": {
    "SessionStart":      [{ "matcher": "", "hooks": [{ "type": "http", "url": "$URL", "headers": { "Authorization": "Bearer $TOKEN" } }] }],
    "UserPromptSubmit":  [{ "matcher": "", "hooks": [{ "type": "http", "url": "$URL", "headers": { "Authorization": "Bearer $TOKEN" } }] }],
    "PreToolUse":        [{ "matcher": "", "hooks": [{ "type": "http", "url": "$URL", "headers": { "Authorization": "Bearer $TOKEN" } }] }],
    "PostToolUse":       [{ "matcher": "", "hooks": [{ "type": "http", "url": "$URL", "headers": { "Authorization": "Bearer $TOKEN" } }] }],
    "Stop":              [{ "matcher": "", "hooks": [{ "type": "http", "url": "$URL", "headers": { "Authorization": "Bearer $TOKEN" } }] }],
    "SubagentStop":      [{ "matcher": "", "hooks": [{ "type": "http", "url": "$URL", "headers": { "Authorization": "Bearer $TOKEN" } }] }],
    "Notification":      [{ "matcher": "", "hooks": [{ "type": "http", "url": "$URL", "headers": { "Authorization": "Bearer $TOKEN" } }] }]
  }
}
```

`$URL` is `http://127.0.0.1:<port>/hook/<session-uuid>`; `$TOKEN` is a per-session random secret.

**Bugfix (BUG-001, 2026-07-27)**: the example above previously flattened the hook object directly
into each event's array (`[{ "type": "http", ... }]`), omitting the `matcher`/`hooks` wrapper every
Claude Code hook type requires. `claude`'s settings validator rejected the generated file for every
event (`hooks.<Event>.0.hooks: Expected array, but received undefined`), so no session's hooks ever
reached the daemon. Corrected here; see `bugs/BUG-001.md` and `tasks.md` Phase 11. The same pass
also added the previously-missing `SubagentStop` entry: `activity.rs::classify_hook` already
grouped `"Stop" | "SubagentStop"` into the same transition, but nothing configured `claude` to ever
send a `SubagentStop` hook in the first place — found by code review of the BUG-001 fix.

⚠️ ~~**Unverified**: that `type: "http"` hooks accept a custom `headers` map. If they do not, the
token moves into the URL path — still per-session and unguessable, but it would then appear in any
hook logging. **Verify before implementing**; the fallback is a path-embedded token.~~
**Confirmed (BUG-001)**: `type: "http"` is a real, supported hook type and does accept a custom
`headers` map (confirmed against the official Claude Code hooks reference). What was *not*
verified — and turned out wrong — was the array shape: every hook type's entries, `http` included,
must be nested under a `{"matcher": ..., "hooks": [...]}` wrapper, not placed directly in the
event's top-level array.

---

## Listener requirements

This is the one HTTP listener in an otherwise socket-only, local-first application, so it is
constrained tightly (see plan.md Complexity Tracking):

1. **MUST** bind `127.0.0.1` (and `::1`) only. Never `0.0.0.0`. Nothing leaves the device.
2. **MUST** listen on an ephemeral port chosen at daemon start, never a fixed well-known port.
3. **MUST** require the per-session bearer token and reject mismatches with `403`, without
   revealing whether the session exists.
4. **MUST** expose no capability beyond reporting activity. It is not a route to project state,
   session input, or the catalog. A compromised token can lie about one session's activity signal
   and nothing else.
5. **MUST** bound request bodies and reject oversized payloads — with the bound **sized against
   measured real payloads**, not against an intuition about how large a hook "ought" to be. The bound
   exists for one reason: stop a hostile or looping sender from making the daemon buffer without
   limit. It is **not** a statement about normal payload size, and a bound that rejects normal
   payloads has failed at its own job while appearing to do it (BUG-010).

   **A hook payload is not small.** `PostToolUse` embeds the tool's entire input *and* response, so
   for `Edit`/`Write` it carries the edited file's full contents (`tool_response.originalFile`, plus
   `tool_input.old_string`/`new_string` or `content`). Payload size therefore scales with the user's
   source tree, not with the hook envelope. Measured across 3,332 real `Edit`/`Write` payloads from
   this project's own transcripts: largest **97,087 bytes (95 KiB)**, next two 68,227 B and 67,933 B.
   `PreToolUse` and `UserPromptSubmit` grow the same way (a large edit's `tool_input`, a large pasted
   prompt) and, unlike `PostToolUse`, both carry a `Working` transition — so a bound that clips them
   loses signal, not just noise.

   Size the bound in **megabytes**, and treat any future value as needing the same justification.
   Only `hook_event_name` — a few dozen bytes — is ever read from the body, so the whole bound is a
   memory guarantee and nothing else.

   A payload the receiver does refuse **MUST** degrade under **H1**: the session holds its prior
   state or reports `Unknown`, never a state the refused hook would have contradicted. See §State
   machine, "A refused or undelivered hook".

   An over-bound request **MUST** be drained before it is answered — read and discarded up to a hard
   byte cap *and* a short deadline, never accumulated. Answering and closing mid-body costs the peer
   the response entirely: it sees `ECONNRESET` on its remaining write and cannot report why the hook
   failed. Both bounds are required; a byte cap alone parks the handler forever on a peer that
   declares a large `Content-Length` and then sends nothing without closing its write half.
6. **MUST NOT** log request bodies — they carry transcript paths and prompt metadata (FR-047). This
   holds regardless of the bound in rule 5: a body rejected as oversized is not logged either, not
   even its size-bearing prefix.
7. **MUST** answer nothing before authentication beyond `403` — the size check in rule 5 included.
   Rejecting an oversized body *before* checking the token tells an unauthenticated caller which
   bound it crossed; it discloses nothing about the session, but it is a free disclosure with no
   reason to exist (BUG-010).

---

## State machine

```text
                 UserPromptSubmit, PreToolUse
       ┌──────────────────────────────────────────┐
       │                                          ▼
   AwaitingInput ◄──── Stop, Notification ──── Working
       │                                          │
       └──────────► Ended ◄───────────────────────┘
                (process exit / crash-loop give-up)

   Unknown  ── first signal ──► Working | AwaitingInput
```

| Event | Transition |
|---|---|
| `SessionStart` | → `Unknown` (session known, no turn state yet) |
| `UserPromptSubmit` | → `Working` |
| `PreToolUse` | → `Working` |
| `PostToolUse` | no change (still `Working` until `Stop`) |
| `Stop` | → `AwaitingInput` |
| `Notification` (`permission_prompt`, `idle_prompt`, `agent_needs_input`) | → `AwaitingInput` ⚠️ subtypes unverified |
| Process exit / give-up | → `Ended { reason }` |
| OSC 0 title with a braille spinner glyph | `Unknown` → `Working`. **Never** any transition *out* of `Working`. |
| OSC 0 title text (glyph stripped) | Session title update — orthogonal to activity |

**Invariants**

- **H1** — Absent, unconfigured or silently-dropped hooks yield **`Unknown`, never `AwaitingInput`**.
  A user running `--bare` or with conflicting settings loses hooks silently; guessing "idle" would
  produce exactly the false attention signal FR-016c forbids.
- **H2** — `Unknown` and `Working` are ambient. Only `AwaitingInput` and `Ended` are
  notification-grade (FR-016c).
- **H3** — Activity is **not persisted**; it resets to `Unknown` on daemon restart (A4).
- **H1a** — Terminal-derived evidence is **monotone toward `Working` only**. Nothing observed on the
  PTY may move a session toward `AwaitingInput`; only hooks and process exit may. This keeps the one
  falsified class of inference structurally impossible rather than merely discouraged.
**A refused or undelivered hook** (BUG-010). A hook the receiver answers with `4xx` — or that never
arrives at all — produces **no transition**. The session holds whatever state it was in, which by the
table above is always the *safe* direction: a lost `PreToolUse` or `UserPromptSubmit` leaves a session
looking less busy than it is, a lost `Stop` leaves it looking busier. Neither invents an
`AwaitingInput`, so **H1 holds under refusal as well as under absence**. A lost `PostToolUse` changes
nothing by definition. This is a degradation guarantee, not a licence to refuse hooks: rule 5's bound
must still admit every payload the agent legitimately sends, because the user sees the refusal as an
error inside their own agent session even when the daemon's state stays correct.

- **H4** — `Stop` means *the model turn ended*, not that a human is required. Auto-continuation or a
  blocking `Stop` hook can resume without user input, so `AwaitingInput` is a strong hint, not a
  guarantee. The UI wording should reflect that.

---

## Degraded fallback: transcript JSONL

Used only when hooks are unavailable, and **explicitly degraded** — the daemon reports `Unknown`
rather than pretending this is equivalent.

- Path: **take `transcript_path` from the hook payload.** Do **not** compute the slug — the current
  `/` and `.` → `-` transform (`src/provider.rs:361-373`) is lossy and collision-prone.
- Turn end is detectable via `stop_reason` (`end_turn` vs `tool_use`), but the official
  documentation states the transcript **"is written asynchronously and may lag the in-memory
  conversation"** — explicitly not a real-time signal.
- Scan backwards; the last line is often a `last-prompt` or `ai-title` record. Tolerate trailing
  partial lines, since reads race the writer.

This same file remains the source for session titles, which the daemon should move to an
event-driven read rather than the current full-file rescan every 120 ms on the UI thread
(`src/main.rs:754`).

---

## Verification required before implementation

| Item | Status |
|---|---|
| `type: "http"` hooks accept custom `headers` | ✅ Confirmed (BUG-001) |
| Each event's array entries need a `matcher`/`hooks` wrapper, not a bare hook object | ✅ Confirmed the hard way (BUG-001) — this was never listed as unverified and was simply wrong; see the Configuration section above |
| **Payload *size*** — that a hook body fits any assumed bound | ✅ Measured the hard way (BUG-010): it does not. `PostToolUse` embeds the edited file, so payloads reach ~95 KiB in this repo and scale with the user's files. Never assume a size; measure. |
| `Notification` subtype values | ⚠️ **Unverified** — did not fire during testing |
| Hook delivery inside a PTY session | ✅ Observed: `SessionStart → UserPromptSubmit → PreToolUse → PostToolUse → Stop` |
| `Stop` timing relative to turn end | ✅ Observed ~12 ms before the turn-end result |
| `--settings <file>` isolates from user config | ✅ Documented |
| OSC 0 title is emitted continuously and carries the session name | ✅ Measured, v2.1.216 |
| Title glyph cannot distinguish working from idle | ✅ Measured: `✳` on both; 26.03 s dark mid-tool-call |
