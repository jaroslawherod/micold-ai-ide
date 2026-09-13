# Phase 0 Research: Run a session on the Pi coding agent

**Feature**: 029-pi-cli-provider | **Date**: 2026-09-12 | **Spec**: [spec.md](./spec.md)

Every item below is Decision / Rationale / Alternatives considered. Sources are Pi's own published
documentation and its published source at `earendil-works/pi` (branch `main`, read 2026-09-12) —
named per item, because this is an external dependency contract and the next person needs to be able
to re-check it.

Pi version of record: **`@earendil-works/pi-coding-agent` 0.85.1** (npm `dist-tags.latest`,
2026-09-12). `engines.node >= 22.19.0`; `os` and `cpu` are unrestricted — it is a JavaScript bundle,
not a per-platform binary.

---

## R1 — Can the application choose a conversation's identity for a conversation that does not exist yet?

**This was the one question the spec left open** (FR-005b exists to settle the behaviour either way).

**Decision**: Yes. `pi --session-id <id>` is the exact analogue of `claude --session-id` and
`copilot --session-id`: *"Use exact project session ID, creating it if missing"*. It is **idempotent
across both launch modes** — `main.ts` looks the id up among the project's own sessions and opens it
if found, otherwise creates a new session carrying that id (printing a one-line warning to stderr).
So `launch_args` is the same vector for `LaunchMode::Fresh` and `LaunchMode::Resume`.

FR-005b is therefore **not exercised**: no application-side correspondence store is needed, the
identity is carried by where the conversation is stored, and Pi's name field is left alone as
FR-005a requires.

Two constraints come with it:

- The id must satisfy Pi's `assertValidSessionId`: `^[A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9])?$`.
  A hyphenated UUID v4 passes unchanged, so the application's existing session id is usable as-is.
- `--session-id` may not be combined with `--continue`, `--resume`, or `--session`; it *may* be
  combined with `--fork`. None of those are in our launch vector.

**Rationale**: it removes the only unresolved mechanism question in the spec, and it does so by the
route the seam already has a shape for. The flag is absent from pi.dev's sessions page and appears
only in `pi --help` and in `src/cli/args.ts` — which is why the earlier documentation pass could not
answer this and the source could.

**Alternatives considered**: `--session <path|id>` (resolves an *existing* file or partial UUID and
does not create; wrong for a fresh launch). `--session-dir <dir>` per session (would give every
conversation its own directory and split Pi's own per-cwd picker, which FR-005a forbids). Recording
the correspondence in the application's own store (FR-005b's fallback — unnecessary now, and it
would make the correspondence remembered rather than derived).

**Source**: `packages/coding-agent/src/cli/args.ts` (flag table, line ~288);
`packages/coding-agent/src/main.ts` `createSessionManager` (lines ~311–443);
`packages/coding-agent/src/core/session-manager.ts` `assertValidSessionId` (line ~212).

---

## R2 — Does Pi leave any liveness indicator the FR-006b check can read?

**Decision**: **No.** Pi takes no lock and writes no pid/marker for a session. `proper-lockfile` is a
dependency, but it is used only for the trust store, auth storage, settings and the experimental
server — never for the session store. Two `pi` processes opened on one conversation simply both
append to the same JSONL file.

So for Pi, FR-006b's check is **unanswerable**, and the operative arm is **FR-006c: silence**. The
application makes no probe, shows no warning, and proceeds. FR-006a is unaffected — a collision
between two of *this application's own* sessions is known from its own registry, not inferred, and
stays a refusal.

**Rationale**: FR-006c already specifies exactly this outcome, so nothing in the spec changes. What
changes is the honest expectation: on Pi the advisory warning of FR-006b will never fire, and
FR-022's documentation obligation ("the warning is advisory rather than a guarantee") is the place
that has to say so.

**Alternatives considered**: scanning the process table for a `pi` whose cwd matches — a probe, and a
per-platform one, which FR-003a's "no probe" posture and Principle VI both push back on. Writing our
*own* lock beside Pi's session file — it would detect only our own sessions, which FR-006a already
detects without it, and it would put a second app artifact in the user's store for no gain.
Inferring from the file's mtime — FR-012 forbids inferring liveness from file metadata, and the same
reasoning applies here.

**Source**: `gh api search/code` for `proper-lockfile` across `earendil-works/pi` (no hit in
`core/session-manager.ts`); `session-manager.ts` contains no lock of any kind.

---

## R3 — Does `pi` ship for Windows, and where is its base directory?

**Decision**: Yes, all three platforms; the base directory is `$PI_CODING_AGENT_DIR` when set,
otherwise `~/.pi/agent` — **home-relative on every platform including Windows**, exactly like
`ClaudeProvider` and `CopilotProvider`. `config_dir()` therefore needs no `cfg!(windows)` arm, and
Principle VI is satisfied by the same one-code-path argument `resolves_on_path` already makes.

The npm package declares no `os`/`cpu` restriction and ships a single JS bundle (`bin.pi =
dist/bundle/cli.js`); Pi's own usage documentation describes Windows Terminal, Notepad-as-editor and
a `powershell` tool beside the `bash` one.

**Rationale**: keeps the third provider's cross-platform story identical to the first two, which is
what FR-020 asks for — no Pi-only branch anywhere.

**Alternatives considered**: none; this is a fact, not a choice. Recorded because the Constitution
Check needs it and because an incorrect assumption here would surface only on Windows CI.

**Source**: npm registry metadata for 0.85.1; `packages/coding-agent/docs/environment-variables.md`
(`PI_CODING_AGENT_DIR` — *"Override the config directory; default is `~/.pi/agent`"*);
`packages/coding-agent/docs/usage.md`.

---

## R4 — How is the activity component supplied per launch, and what runtime does it need?

**Decision**: `pi -e <path-to-file>` (`--extension`, repeatable). It accepts a relative or absolute
path to a `.ts` or `.js` file **without installation**, Pi's own documentation calling it the
quick-test form. The session service materialises the component as a file in its own data directory
and passes that path at spawn.

This satisfies FR-012a end to end:

- **Scoped to our launches** — a per-launch flag, nothing written into `~/.pi/agent/extensions/` or
  `.pi/extensions/`, so the user's own `pi` is untouched (FR-007 unrelaxed).
- **No project trust required** — Pi classifies `-e` extensions as *CLI scope*, alongside
  user/global; only `.pi/extensions/` (project-local) waits for the project to be trusted.
- **Nothing added to the image (FR-017a)** — the runtime is `jiti`, a direct dependency of
  `pi-coding-agent` itself, so TypeScript runs without compilation and without a toolchain. An image
  that carries `pi` carries everything the component needs, and a substituted image cannot lose the
  badge while keeping Pi working.
- **Travels with the session service** — the service writes the file and spawns Pi, and it is
  present wherever sessions run, so host and sandboxed placement behave identically.

**Rationale**: this is the only supply route that is simultaneously per-launch, install-free and
trust-free, and it is the one Pi documents for exactly this purpose.

**Alternatives considered**: installing into `~/.pi/agent/extensions/` (persistent, affects the
user's own `pi`, forbidden by FR-012a). `.pi/extensions/` in the worktree (project-local, needs
trust, and writes into the user's repository). Pi's RPC mode (`docs/rpc.md`) — the right shape for a
different feature, and it would replace the PTY session model rather than observe it, the same
reason 026 declined Copilot's `--acp`.

**Source**: `packages/coding-agent/docs/extensions.md`; `src/cli/args.ts` (`-e, --extension
<source>`); `packages/coding-agent/package.json` dependencies (`jiti`);
`src/core/trust-manager.ts`.

---

## R5 — Where does a conversation's name live, and what must a bounded read look at?

**Decision**: read a **bounded prefix** of the session JSONL — a fixed byte budget from the start of
the file, stopping at the last complete line — and resolve the label as:

1. the **latest** `{"type":"session_info","name":…}` entry seen within the budget, if any; else
2. the text of the **first user message** within the budget — which is precisely the fallback Pi's
   own picker uses (`SessionInfo.firstMessage`, defaulting to `"(no messages)"`); else
3. the existing neutral placeholder.

Pi's own name semantics are "the latest `session_info` entry wins", resolved by walking entries in
reverse. A bounded prefix therefore reads the name **best-effort**: a name set at startup or early is
found; one set by `/name` deep into a long conversation is not, and the row falls back — which
FR-011 already sanctions in as many words ("where the bound is reached without a usable label, the
fallback chain above applies").

Layout facts this rests on:

- File: `<base>/sessions/--<encoded cwd>--/<timestamp>_<session-id>.jsonl`, where the encoding is
  `cwd` with the leading separator stripped and every `/`, `\` and `:` replaced by `-`, wrapped in
  `--`…`--`.
- Line 1 is the header: `{"type":"session","version":3,"id":"<uuid>","timestamp":…,"cwd":…}`. The
  header carries the id and the cwd; it does **not** carry the name.
- Every later line carries `id`, `parentId`, `timestamp` and a `type`.

**Rationale**: the prefix is where the *common* case lives. Pi records a name only when asked to, so
most conversations have none and the label comes from the first user message — which is, by
construction, at the start of the file. A prefix read gets that exactly right, always, at constant
cost, satisfying FR-015 and SC-006b.

**Alternatives considered**: reading the whole file the way Pi's own picker does — `buildSessionInfo`
streams every line of every session file and accumulates `allMessagesText`, so listing a project is
O(total conversation bytes); adopting it would break SC-006b outright, and it is the specific cost
FR-011's bound was written against. A bounded **tail** read — it would catch late `/name` renames,
but it would systematically miss the first-user-message fallback that covers the majority of rows,
trading a rare improvement for a common regression. Reading Pi's picker output by running `pi`
— a process spawn per project open, and FR-003a's no-probe posture forbids it.

**Source**: `src/core/session-manager.ts` — `getDefaultSessionDirPath` (line ~479),
`getSessionName` / `appendSessionInfo` (lines ~1150–1175), `buildSessionInfo` (line ~688);
`packages/coding-agent/docs/session-format.md`.

---

## R6 — What does the sandbox image have to ship, and at what version?

**Decision**: add `ARG PI_CLI_VERSION=0.85.1` beside the two existing pins and install
`@earendil-works/pi-coding-agent@${PI_CLI_VERSION}` in the **same** `npm install -g --omit=dev`
invocation that already installs `@anthropic-ai/claude-code` and `@github/copilot` — one layer, one
cache purge, no new base image, no new package manager.

`engines.node >= 22.19.0` must hold against the image's `node:22-trixie-slim` tag; the tag tracks the
newest 22.x, so it does today. This is the one image-side fact worth asserting rather than assuming,
and `mise run image` + `mise run test-sandbox` is where it gets asserted — the existing
`crates/micold-daemon/tests/sandbox_real_ai_cli.rs` iterates `AiCli::ALL`, so `AiCli::Pi` puts `pi`
under that check automatically and the test **fails until the image ships it**. That failure is the
Test-First red for the image change.

`PI_OFFLINE=1` is also set in the image and on every launch (see R9).

**Rationale**: FR-017 wants a pinned version and nothing more; FR-021 wants no second enumeration of
the CLIs, and reusing `AiCli::ALL` in the image check is what keeps the image honest for a fourth
provider too.

**Alternatives considered**: Pi's shell-script installer or a per-platform binary — there is none;
the distribution is npm. A separate `RUN` layer — more image layers and a second cache purge for no
benefit. Pinning a major-range rather than an exact version — the two existing CLIs pin exactly, and
a floating pin makes the published image unreproducible.

**Source**: `packaging/sandbox/Containerfile`; npm registry metadata for 0.85.1;
`crates/micold-daemon/tests/sandbox_real_ai_cli.rs`.

---

## R7 — Which `ActivitySource` does Pi need, and how do its events reach the daemon?

**Decision**: **the seam changes** — a new `ActivitySource` variant is added for the case Pi
introduces, and `ActivitySource::EventLog` is left exactly as Copilot uses it:

```rust
/// The provider reports only through a component the application supplies at launch
/// (feature 029, FR-012a). The daemon materialises that component, injects it into the
/// spawn, and tails the append-only log it writes at this derived path.
Extension { log: PathBuf },
```

The log path is derived purely from `(config_dir, session_id)`:

```
<pi base>/micold-activity/<session-id>.jsonl
```

— beside `sessions/`, never inside it, so it can never be mistaken for a conversation by discovery.
The component learns the same path from an environment variable set at spawn.

The daemon then **reuses `EventLogTail` unchanged**: same directory watch, same append-only read,
same no-timer guarantee (FR-014 is satisfied by the mechanism that already satisfies FR-019 for
Copilot). Only the line→`ActivityEvent` mapping is new, a `pi_event` beside `copilot_event`.

Event mapping (Pi extension event → the vocabulary the `Activity` state machine already consumes):

| Pi event | `HookKind` | Signal |
|---|---|---|
| `turn_start` | `UserPromptSubmit` | `Working` |
| `agent_start` | `PreToolUse` | `Working` |
| `tool_execution_start` | `PreToolUse` | `Working` |
| `tool_execution_end` | `PostToolUse` | *(no change — turn continues)* |
| `agent_settled` | `Stop` | `AwaitingInput` |
| `turn_end` | `Stop` | `AwaitingInput` |
| `session_shutdown` | *(termination)* | `Ended { reason }` |

Unknown types are ignored, not rejected — Pi gains event types between versions.

**Rationale**: `ActivitySource::EventLog` means "the provider writes a log we can find". Pi writes
no such log; a component *we* supply does, and the daemon has to do two extra things at spawn —
materialise the component and inject `-e`/the env var — that `EventLog` does not imply. Keying that
work off `EventLog` would make it fire for Copilot too, and keying it off "is this Pi?" is exactly
the conditional FR-019/FR-020 forbid. A distinct variant makes the difference structural rather than
conditional (Principle V), and the precedent is already in the enum: `ActivitySource::Hooks` is
payload-free *because* the daemon supplies per-session launch material the provider cannot derive.
This is the same pattern with a derivable path attached.

This is FR-020 landing as written: Pi could not be expressed through the seam as it stands, so the
seam is what changes, and the change is available to every provider rather than to Pi alone.

**Alternatives considered**: reuse `EventLog` and branch on `AiCli::Pi` in the daemon (a conditional
on which CLI a session runs — FR-020 forbids it by name). Have the component POST to the daemon's
loopback receiver like `claude`'s hooks (it would need the port and per-session token pushed into
the extension's environment, it makes the component a network client inside the user's agent, and it
is a much larger surface for FR-012b's "reviewable in one place"). Derive activity from Pi's session
file (FR-012 forbids inferring from file size or mtime, and a conversation that thinks without
writing would read as idle). A new trait method instead of a variant (every provider would have to
implement something only one of them uses — the trait's "no method has a default" rule makes that a
cost on all three).

**Source**: `crates/micold-core/src/provider.rs` (`ActivitySource`, lines 56–76);
`crates/micold-daemon/src/event_log.rs`; `crates/micold-daemon/src/activity.rs` (`copilot_event`);
`crates/micold-daemon/src/state.rs` (lines ~307–321, ~1337–1361);
`packages/coding-agent/docs/extensions.md` (event list, `ctx.isIdle()`).

---

## R8 — Environment variables Pi reads

**Decision**: confirmed, and only the first is used by `config_dir()`.

| Variable | Effect | Used here |
|---|---|---|
| `PI_CODING_AGENT_DIR` | overrides the config directory; default `~/.pi/agent` | **yes** — the `CLAUDE_CONFIG_DIR`/`COPILOT_HOME` analogue |
| `PI_CODING_AGENT_SESSION_DIR` | overrides session storage; itself overridden by `--session-dir` | no — relocating the store is what FR-005a forbids |
| `PI_OFFLINE` | disables startup network operations: update checks, package updates, install/update telemetry | **yes** — see R9 |
| `PI_SKIP_VERSION_CHECK` | disables the `pi.dev` latest-version request | **yes** — see R9 |
| `PI_TELEMETRY` | overrides install/update telemetry and provider attribution headers | **yes** — see R9 |

Pi also *sets* `AI_AGENT=pi` and `PI_CODING_AGENT=true` on its children, and injects
`PI_SESSION_ID` / `PI_SESSION_FILE` into its own `bash`/`powershell` tools. None of those are a
liveness signal the application can read (R2) — they exist only inside Pi's own process tree.

**Source**: `packages/coding-agent/docs/environment-variables.md`.

---

## R9 — Principle IV: what does a Pi launch send off the device?

**Decision**: launch Pi with `PI_OFFLINE=1`, `PI_SKIP_VERSION_CHECK=1` and `PI_TELEMETRY=0` in the
session's environment, and set the same three in the sandbox image.

Pi's documented startup behaviour includes an update check, a `pi.dev` latest-version request, and
install/update telemetry. A session this application spawned on the user's behalf must not reach the
network on its own account without an explicit opt-in — this is the same judgement, and the same
Principle IV clause, that made `--no-remote` deliberate rather than incidental for Copilot. Model
traffic is not covered by this: that is the user's own configured provider and the reason they
started the session.

Per-launch environment, not configuration: nothing in the user's `~/.pi` is modified (FR-007), and
the user's own `pi` is unaffected.

**Alternatives considered**: writing `offline: true` into Pi's settings file (modifies the user's
configuration, which FR-007 forbids). Leaving the defaults and documenting them (Principle IV is
NON-NEGOTIABLE and reads "nothing leaves the device without explicit informed opt-in"; a line in the
user guide is not an opt-in).

**Source**: `packages/coding-agent/docs/environment-variables.md`.

---

## R10 — Will Pi prompt for project trust in a fresh worktree?

**Decision**: Not for the worktrees this application creates. Pi asks for a trust decision only when
`hasTrustRequiringProjectResources(cwd)` is true — that is, when the directory actually contains
trust-requiring resources under `cwd/.pi` or `.agents/skills`. A worktree with neither never reaches
the prompt.

Where a project *does* carry them, the prompt is Pi's own, it appears in the session's own terminal,
the user answers it there, and the sidebar must not meanwhile report the session as failed — the
same posture 026 recorded for Copilot's `trustedFolders`. The activity component is unaffected
either way: `-e` extensions are CLI scope and load before any trust decision (R4).

**Rationale**: it removes the "Open" item that the Copilot contract had to carry, and it means the
quickstart does not need a trust-prompt step for the default case.

**Alternatives considered**: pre-seeding a trust decision into Pi's trust store — modifying the
user's configuration (FR-007), and deciding a security question on their behalf.

**Source**: `packages/coding-agent/src/core/trust-manager.ts`;
`packages/coding-agent/docs/extensions.md`.

---

## Summary of seam impact

| Seam capability | Pi needs a change? |
|---|---|
| `id` / `display_name` / `command` / `is_available` | no — `AiCli::Pi`, `"Pi Coding Agent"`, `"pi"`, existing `resolves_on_path` |
| `launch_args` | no — one vector for both modes (R1) |
| `config_dir` | no — same env-or-home shape as both existing providers (R3, R8) |
| `recorded_session_ids` / `has_recorded_conversation` | no — directory listing under a derived per-cwd path (R5) |
| `read_title` | no — best-effort bounded read (R5) |
| `mark_archived` / `is_archived` | no — empty marker in Pi's own storage, as both existing providers do |
| `activity_source` | **yes — one new `ActivitySource` variant** (R7) |
| daemon spawn preparation | **yes — materialise + inject the component, keyed on that variant** (R4, R7) |

One change, in the seam, available to every provider. That is the answer to the second purpose the
spec set for this feature.
