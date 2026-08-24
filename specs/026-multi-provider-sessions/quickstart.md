# Quickstart: Choose which AI CLI a session runs on

Two parts. **§A** is what the machine checks. **§B** is what needs a real Copilot CLI, a real
worktree, and a real conversation — the parts no test in this repository can reach, because they
depend on another vendor's binary and on its on-disk formats being what they were on 2026-08-14.

**Prerequisites for §B**: both CLIs installed and logged in (`claude --version`,
`copilot --version`), and a project registered in the application with at least one worktree.

> **Never run §B against your own `~/.copilot`.** Export `COPILOT_HOME` to a scratch directory
> first — it relocates Copilot's entire store, verified in research R2 — so a failed step cannot
> lose or pollute real conversation history.

---

## §A — The automated suite

```bash
mise run test        # whole workspace, matching CI
mise run test-core   # provider + store + settings only; much faster while iterating
```

Green is the gate. What each gate is watching, for this feature:

| Gate | Watching |
|---|---|
| `micold-core/tests/ai_cli_provider.rs` | `ClaudeProvider`'s existing path arithmetic, title parsing and marker behaviour are **unchanged** by the reshape — the regression lock on not breaking provider one while adding provider two |
| `micold-core/tests/copilot_provider.rs` *(new)* | pure path derivation: base dir from `COPILOT_HOME`/`~/.copilot`, `sidebar-sessions-state/<sha256(cwd)>.json`, `session-state/<uuid>/{events.jsonl,workspace.yaml,micold.archived}` — all without the CLI installed |
| `micold-core/tests/copilot_provider.rs` *(new)* | the SHA-256 of a known cwd matches the byte-for-byte value recorded in `contracts/copilot-cli.md`, so a change to the hashing helper cannot silently orphan every session |
| `micold-core/tests/copilot_provider.rs` *(new)* | index parsing: ids listed, a `schemaVersion` other than `1` contributes nothing, a truncated or absent file contributes nothing — **never an error** |
| `micold-core/tests/copilot_provider.rs` *(new)* | title: `name:` present → `Named`, absent → `Pending`, quoted and colon-containing values parse, an unreadable file yields `None` |
| `micold-core/tests/terminal_backend.rs` | the launch argv comes from the **spec's own provider** — `LaunchSpec` carries one, and `claude_args` no longer hard-codes `ClaudeProvider`. The one gate here that fails because a struct is the wrong shape rather than because a call site names a type |
| `micold-core/tests/ai_cli_provider_seam.rs` | the seam is still object-safe and substitutable, and the fake still drives a consumer — extended so the consumer is a **real** one now that `Capabilities` holds a registry (the file's own module docs already name this as the work T049 left open) |
| `micold-core/tests/store_roundtrip.rs` | `provider` survives save → load, **and** a session file written without the field loads as `ClaudeCode` — the claim that lets this ship with no `schema_version` bump (research R8, FR-013) |
| `micold-core/tests/store_roundtrip.rs` | an unknown provider string is a load error, not a silent fallback to `ClaudeCode` (data-model round-trip table) |
| `micold-core/tests/settings_ai_cli.rs` *(new)* | `default_ai_cli` round-trips; a settings file without it loads as `ClaudeCode`; a default naming an uninstalled CLI is **kept**, not rewritten; `settings_version` does not move |
| the service-owned-settings test T023a adds *(new)* | a `SettingsSet` changing the **scrollback limit** leaves `default_ai_cli` intact — the reason the preference is service-owned rather than sitting with `theme`, since the daemon persists its whole boot-time `Settings` struct on every set |
| `micold-core/tests/schema_hash.rs` | the protocol hash **moves**, once for the whole feature, with a single version bump — `SessionCreate` and `CatalogSnapshot` change together. If it moves without the bump, or moves a second time later, something else reached for the wire |
| `micold-daemon/tests/session_start.rs` | a missing CLI reports `WireLifecycle::Failed { reason, .. }` naming it, **without** spending restart attempts — the domain `SessionLifecycle::Failed` is a payload-free "crash-loop exhausted" and is the wrong home for it |
| `micold-daemon/tests/session_start.rs` | a Copilot session is spawned with Copilot's argv and **no `--settings` file** — the hook settings file follows `activity_source`, so it is written for a `Hooks` provider and withheld from an `EventLog` one. The launch half of the seam, and the failure a rename-only pass through `spawn_claude` leaves behind |
| `micold-daemon/tests/activity_pipeline.rs` | the `Activity` state machine is byte-identical in behaviour; what changes is where its events come from. Most Copilot event names map onto the existing `HookKind` vocabulary, but the mapping is typed at `ActivityEvent`, since `session.shutdown`/`session.error` land on `Ended { reason }` — a sibling variant `HookKind` cannot express |
| `micold-daemon/tests/copilot_activity.rs` *(new)* | tailing a fixture `events.jsonl`: `user.message`/`turn_start` → `Working`, `turn_end` → `AwaitingInput`, `permission.requested` → `AwaitingInput`, `session.shutdown` → `Ended`; **unknown event types are ignored, not rejected** |
| `micold-daemon/tests/copilot_activity.rs` *(new)* | a dangling `assistant.turn_start` from a dead process does not leave the badge `Working` forever |
| `micold-core/tests/session_reconciliation.rs` | the *rules* of discovery: ids from the index, a `micold.archived` marker suppressing one permanently, ~250 ids with nothing capped or aged out (FR-014, FR-015). **This file is a mirror** — its own module doc says it hand-copies a `reconcile_sessions_from_transcripts` that no longer exists in `micold-client/src/main.rs`. It pins the rules; it gates nothing |
| the discovery test T042a adds *(new)*, in `micold-daemon/tests/` | the same rules against the **real** entry point: the function called from the `AttachProject` arm (R15). This is the gate on FR-014/FR-015; the row above is not. It also holds the two properties R15's cost argument rests on — the catalog's own ids are subtracted **before** any `micold.archived` stat, so a location with hundreds of known conversations does no per-conversation I/O; and a reopen adds nothing, because a discovered session's `SessionId` is the CLI's own uuid |
| `micold-daemon/tests/set_wide_provider_decisions.rs` *(new)* | the two decisions in `state.rs` that judge a **set** of sessions — `prune_empty_sessions` (which archives) and `present_interrupted_resumable_at_startup` — consult **each session's own** provider. One provider's unresolvable config dir must not condemn or spare another's. This is the feature's only silent-data-loss path |
| the client boot-prune test T008b adds *(new)* | the same rule on the client side: `main.rs::prune_empty_sessions`, reached from `shell/startup.rs`, judges each session by **its own** provider. It drops sessions from the workspace, and it passes `no_concrete_implementations.rs` while doing so, because it names nothing concrete |
| `micold-daemon/tests/session_survival.rs` | a Copilot session survives a **daemon** restart on the right provider — the leg of FR-012 neither the store round-trip nor §B B3 reaches |
| `micold-daemon/tests/copilot_activity_is_event_driven.rs` *(new)* | *this application* schedules no polling timer, no periodic wakeup and no per-idle-session work, and no debouncer sits in the path. Explicitly **not** an assertion about the watch crate's internals (FR-019, SC-006) |
| `micold-client/tests/terminal_bar_stability.rs` | the bar's pinned AI tab carries the session's **command name** (`claude`, `copilot`) as text beside its glyph, in both of a session's panes (FR-016a) |
| `micold-client/src/main.rs`'s own `#[cfg(test)]` module | a `SessionSummary` carrying `Copilot` becomes a **Copilot** session in `reconcile_catalog` — the one path every daemon-reported session takes into the client, which is every discovered session and every session after a client restart. `Session::restored` is the constructor involved, and it is the one T012 has to change alongside `start_new`; miss it and the provider defaults silently while every other test stays green |
| `micold-client/tests/features_sidebar.rs` | the CLI label changes a row's **content, not its height**. `features/sidebar.rs::row_heights` hardcodes one line per session row and the scroll target is computed from it — the one place in the sidebar where a wrong answer is silent rather than visible |
| `micold-client/tests/features_sidebar.rs` + `features_settings.rs` | the two naming registers stay apart: rows and the terminal bar use `command()`, the Settings select / override list / failure messages use `display_name()`. Both strings hang off the same provider, which is what makes the drift likely |
| `micold-daemon/tests/copilot_activity.rs` | the badge's **scope** — a discovered but unsupervised session reads `Unknown`, and **no watch is opened for it**, however many a project holds (FR-018, SC-006) |
| `micold-daemon/tests/session_start.rs` | resuming a conversation another process holds is attempted like any other and reported if the CLI refuses — with **no liveness probe of our own** (FR-008) |
| `micold-client/tests/layout_snapshot.rs` | the committed geometry fixture, **regenerated deliberately** in T067a once the row label and the terminal-bar label land. The gate cannot heal itself by design (`layout_snapshot_regeneration.rs`), so an unregenerated fixture is a red gate, not a stale one |
| `micold-client/tests/layout_text_overflow.rs` | a narrow sidebar row ellipsizes the **title** first; the CLI label is never what disappears, since FR-016 makes it the identification |
| `micold-client/tests/sidebar_tree.rs` | **SC-009** — ~250 discovered sessions cost no more per row than three: no per-session I/O, no per-session watcher, nothing growing faster than the list. Structural, not timed |
| `micold-client/tests/material_builder_api.rs` + `showcase_completeness.rs` + `showcase_captions.rs` | the split start affordance and any new row/bar component are shared components with chainable builders, registered in the showcase **with their live states declared** — not one-offs, and not bare entries (Principle VIII) |
| `micold-client/tests/no_concrete_implementations.rs` | **extended to the daemon and `core/terminal.rs`** — the four call sites that name `ClaudeProvider` today. This is FR-022, and it is the gate that stops a third provider being wired in by hand. `AiCli::provider` in `micold-core/src/provider.rs` is an **explicitly listed** exemption: it is the definition site and names both types by necessity. `capabilities.rs` should name none at all once T014 delegates |
| the daemon/core lookup (T011a) | that `AiCli::provider` exists at all, in the one crate `micold-core`, `micold-daemon` and `micold-client` can each see. `Capabilities` cannot serve this: the daemon depends on the client only as a dev-dependency, and core not at all — so a registry on `Capabilities` leaves `catalog.rs`, `state.rs`, `supervisor.rs` and `claude_args` with nothing to read |
| `micold-client/tests/features_session.rs` | a session's provider is set at creation and no message changes it (FR-001, FR-005); and both branches live here, render-free — the split affordance's decision (primary press vs secondary, against the availability set) **and** default-vs-override. `main.rs` handles `SessionStartRequested` at the I/O boundary and no integration test can link it, so a branch that drifts in there becomes untestable |
| `micold-daemon/tests/activity_pipeline.rs` + `copilot_activity.rs` | a Copilot session has **two** activity sources — the event log and the shared braille-spinner title path, which is not provider-conditional. They cannot contradict each other (`SpinnerObserved` only ever moves `Unknown → Working`), but SC-005's one-second gate must prove the **log** moved the badge, not the TUI's own animation |
| `micold-core/tests/ai_cli_provider_seam.rs` | `CopilotProvider` satisfies **all twelve** required methods the moment T027 registers it as `Arc<dyn AiCliProvider>`. The trait has no defaults by design, so the five-method type an earlier task list described does not compile; the six bodies staged to later stories must be conservative values — empty, `false`, `None`, `Ok(())`, `ActivitySource::None` — and never `todo!()`, which a registered provider makes reachable from the sidebar |
| `micold-client/tests/features_session.rs` + `service_capability_fakes.rs` | the availability set reaches the render-free layer **as state**, not as `Capabilities`: `features/` imports nothing from `shell::`, and `settings_form`'s view is dispatched through one shared `DialogView` fn-pointer, so a filter written against `Capabilities` has nowhere to live. The snapshot refreshes when the choice is offered and never per frame — a `PATH` probe per render is the scheduled work SC-006 forbids |
| `micold-core/tests/copilot_provider.rs` *(new)* | **FR-011**: a fresh launch and a resume leave the base config directory byte-identical apart from our own `micold.archived` — no `config.json` write, no `trustedFolders` edit |
| `micold-client/tests/service_capability_fakes.rs` | `Capabilities::available_providers()` reflects availability, and an unavailable CLI is not offered (FR-006) |

**What §A cannot tell you**: whether Copilot CLI still writes what `contracts/copilot-cli.md` says it
writes. Every gate above runs against fixtures. That is §B.

---

## §B — The manual pass

```bash
export COPILOT_HOME=/tmp/micold-copilot-probe    # NOT your real store
mise run run
```

This pass is automatable with the repository's `visual-pass` skill for the steps that are about what
the sidebar *shows* (B2, B5, B7); the steps that need a real conversation (B3, B4) need a prompt
actually sent to Copilot.

### B1 — A Copilot session starts, in the right place (US1, FR-007)

Start a session in a worktree, choosing GitHub Copilot. **Expect**: a Copilot TUI in the session's
terminal, its working directory the worktree. Confirm the id is ours:
`ls $COPILOT_HOME/session-state/` names the session id the sidebar shows.

### B2 — The default is honoured, overriding does not change it, and neither costs a click (FR-002, FR-003, FR-004, SC-001)

With the default at Claude Code, press the row's `+` — **one press, no menu** — → a `claude` session.
This is the click budget, and B2 is the only place it can be checked: if starting on the default
takes two interactions, SC-001 has failed however good the rest looks.

Then use the adjacent secondary control → the CLI list → Copilot → a Copilot session, and Settings
still reads Claude Code. Change the default to Copilot → the next unqualified session is Copilot, and
**every session already open is untouched** (FR-005).

### B3 — The choice survives a restart, with the conversation (US2, SC-002)

Have a short exchange in the Copilot session. Quit the application entirely. Reopen, select the
session. **Expect**: a Copilot session, resumed, with that exchange visible — not a fresh one and
not a `claude` session. This is the feature; it cannot be automated, because every test in this
repository runs in one process.

### B4 — The sidebar and the session both tell the truth (US3, FR-016, FR-016a, FR-017, FR-018)

With one session of each CLI open:

- **Each row names its CLI in text**, as the command name — `claude`, `copilot` — not by colour, not
  by a glyph you have to recognise, not in a tooltip. Check it at a narrow sidebar width too: the
  title ellipsizes, the CLI name does not go.
- **Each open session names its CLI on its own terminal bar**, again as the command name, as text
  beside the sparkle glyph on the pinned AI tab at the bottom-right (FR-016a). Switch that session to
  regular-terminal mode: the tab and its name stay exactly where they are — only the mark moves.
- **The menus disagree with the labels, on purpose.** Settings and the override list read "Claude
  Code" and "GitHub Copilot"; the rows and the bar read `claude` and `copilot`. If they have
  converged, one register has leaked into the other.
- The Copilot row picks up Copilot's own title once it has generated one
  (`grep name: $COPILOT_HOME/session-state/<id>/workspace.yaml`).
- Sending a prompt turns the badge to working **within a second** (SC-005), back to awaiting-input
  when it finishes. A permission prompt should read as awaiting input, not as working.
- **The two badges look identical.** Copilot's is not styled as less certain — that distinction was
  withdrawn on 2026-08-16 when the event log turned out to be reported rather than inferred.

### B5 — Sessions started outside the application (FR-014, FR-018)

With the application closed, run `copilot` by hand in one of the project's worktrees and send one
message. Reopen the application and open that project. **Expect**: the session listed, as a Copilot
session, in that worktree, with its activity reading **unknown** — the application is not supervising
it and does not watch its log to find out (FR-018, Clarifications 2026-08-18). Then run a *second*
`copilot` by hand and reopen the project again: it appears too. Discovery runs on every open, not
only the first (FR-014).

Leave that hand-started `copilot` **running**, and select its session in the application. **Expect**:
the resume is attempted like any other, and if `copilot` refuses or exits immediately the session
says so and starts nothing. The application makes no attempt to detect the conflict in advance
(FR-008). Then close it in the application, quit, reopen — it must **not** come
back (FR-015; `ls $COPILOT_HOME/session-state/<id>/micold.archived` should exist).

### B6 — A worktree Copilot has not trusted (research R12, unverified)

Create a brand-new worktree and start a Copilot session in it. **Expect**: if Copilot asks about
trusting the folder, the question appears in the session's own terminal and answering it there
continues normally — and while it waits, the sidebar does **not** report the session as failed.
Record what actually happened; this is the one behaviour in the contract that no probe could
confirm.

### B7 — Only one CLI installed (US4, FR-002, FR-006, FR-010)

Temporarily remove `copilot` from `PATH` and restart. **Expect**: Copilot is not offered as a
choice, and with only one CLI available the **secondary control is gone entirely** — the row looks
exactly as it did before this feature (FR-006). The existing Copilot session is still listed and
still identified as Copilot; selecting it reports that its CLI is missing rather than appearing to
start.

Then set the default to Copilot **while it is still off `PATH`** and press `+`. **Expect**: the
application says the default is unavailable and offers the CLIs that are available; nothing starts
until you pick one; and reopening Settings shows the default still reads Copilot — a temporary
`PATH` problem must not erase the preference (FR-002, Clarifications 2026-08-16).

### B8 — Nothing else moved

A project with only `claude` sessions behaves exactly as it did before this feature: same rows, same
labels, same badges, same resume behaviour. The upgrade path (FR-013) is only real if this is
boring.

---

## Recording the pass

§B is evidence, recorded the way features 006, 010, 020, 021, 022, 024 and 025 recorded theirs: the
date, the platform, and any step that did not behave as written. A step that fails is a defect, not
a note. If §B was not run, say so here rather than leaving the table blank and implying it was.

B3 and B5 are the two that cannot be substituted for by anything in §A — one needs the process
restarted, the other needs a session this application did not create. **B2 is a third**: SC-001's
click budget is a claim about an interaction, and no test in this repository counts clicks.

| Recorded | |
|---|---|
| Date | *(not yet run)* |
| Platform | |
| B1 — a Copilot session starts in the right worktree | |
| B2 — one press starts the default; secondary control overrides; open sessions untouched | |
| B3 — the choice and the conversation survive a restart | |
| B4 — row names the CLI in text; terminal bar names it too; title; badge within 1s; both badges identical | **terminal-bar half only, 2026-08-24** — see T067b. The pinned AI tab names both CLIs correctly in both schemes, marked and unmarked, and the sidebar rows carry their labels in the same frames. Run on Xvfb + lavapipe, so the badge's one-second claim and the title are **not** covered here. Evidence: `evidence/FR-016a-ai-tab-names-its-cli.png` |
| B5 — externally-started session discovered on every open; unknown activity; a still-attached one is attempted and reported, not pre-checked; closing sticks | |
| B6 — untrusted worktree | |
| B7 — only one CLI installed; unavailable default offers the available ones and is not rewritten | |
| B8 — `claude`-only projects unchanged | |
