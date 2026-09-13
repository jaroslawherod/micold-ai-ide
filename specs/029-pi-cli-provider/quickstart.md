# Quickstart: Validating the Pi provider

**Feature**: 029-pi-cli-provider | **Date**: 2026-09-12 | **Plan**: [plan.md](./plan.md)

Runnable procedures that prove the feature works end to end. Details live in
[contracts/pi-cli.md](./contracts/pi-cli.md) and [data-model.md](./data-model.md) and are not
repeated here.

Sections **A** and **B** are the automated gates. Section **C** is the recorded manual procedure the
constitution's Principle I requires for the process-spawn wiring that no unit test can reach — the
activity component inside a real `pi`. Section **D** is the sandbox obligation.

---

## Prerequisites

```bash
mise trust                       # once per fresh worktree
pi --version                     # the CLI under test; any version — no floor is gated (FR-003a)
```

`pi` is installed with `npm install -g @earendil-works/pi-coding-agent` (or pnpm/bun). It needs
Node ≥ 22.19.0 and a configured model provider — the same authentication the user would set up to
run `pi` on their own. **Section C needs a working model provider; A, B and D do not.**

For a store that is safe to inspect and throw away, point Pi's base directory somewhere private:

```bash
export PI_CODING_AGENT_DIR="$(mktemp -d)/pi-agent"
```

Every path below is relative to that base.

---

## A. The seam, without launching anything

The whole provider profile against a fixture store — no process spawn, no model provider, no network.

```bash
mise run test-core
```

Expected: `crates/micold-core/tests/pi_provider.rs` passes, covering

- `id`, `display_name` = `"Pi Coding Agent"`, `command` = `"pi"`, and that neither name leaks into
  the other surface (FR-001a, SC-004a);
- `launch_args` is the **same vector for `Fresh` and `Resume`** — `["--session-id", <uuid>]` — and
  carries none of `--name`, `--no-session`, `--session`, `--session-dir` (FR-005a, contract §Launch);
- `config_dir` honours `PI_CODING_AGENT_DIR`, treats an empty value as absent, and falls back to
  `<home>/.pi/agent` with no platform branch (FR-003, Principle VI);
- `recorded_session_ids` lists every `*.jsonl` in a fixture per-cwd directory, including a UUIDv7 id
  Pi would have generated itself and a file whose header records a `parentSession` — a fork, listed
  on the same terms and with its parentage unread (FR-015);
- `read_title` returns the latest in-prefix `session_info` name; falls back to the first user message
  when there is none; returns `None` for a file whose only name appears **beyond the byte budget**;
  and never errors on a truncated, empty or garbage file (FR-011);
- `mark_archived` writes an empty `<id>.archived` beside the conversation, `is_archived` reads it,
  the conversation file is **byte-identical afterwards**, and the marker is excluded from
  `recorded_session_ids` (FR-016);
- `activity_source` returns `Extension { log }` at `<base>/micold-activity/<id>.jsonl` — outside
  `sessions/`, and never inside it.

Plus, in the same run:

- `ai_cli_provider_seam.rs` — the new `ActivitySource` variant round-trips the seam and `AiCli::ALL`
  is `[ClaudeCode, Copilot, Pi]`, sorted, with `Default` still `ClaudeCode`;
- `schema_hash.rs` / `protocol_roundtrip.rs` — the third value serializes without disturbing the two
  existing ones (SC-003);
- `settings_ai_cli.rs` — the FR-012e switch defaults **on**, persists, and a settings file written
  before this feature loads with it on.

Then the seam-honesty gates, which are the point of the exercise:

```bash
mise run test
```

Expected: `micold-client/tests/no_concrete_implementations.rs` passes **unchanged** — no session,
storage, sidebar or terminal code names Pi (FR-019) — and no test needed a new exemption (SC-007).

---

## B. A real session, start to resume

Launch, close, reopen, resume — the FR-005/FR-005a loop, with a real `pi`.

1. Start the application and open a project:

   ```bash
   mise run run
   ```

2. Pick **Pi Coding Agent** as a one-off override and start a session. Expected:
   - the terminal shows `pi` running in that session's own worktree;
   - the sidebar row's CLI label reads `pi` (FR-009), and the terminal bar's pinned AI tab names
     `pi` as text beside the glyph (FR-010);
   - the row is identifiable without relying on colour, a glyph, or a tooltip alone.

3. Send one message, then check Pi's store from a second shell:

   ```bash
   ls "$PI_CODING_AGENT_DIR"/sessions/--*--/
   ```

   Expected: one `<timestamp>_<session-id>.jsonl` whose id part is **the application's own session
   id** — the identity is carried by where the conversation is stored, and the file's `session_info`
   entries (if any) were not written by the application (FR-005a, SC-004b).

4. Confirm the conversation is the user's, outside the application:

   ```bash
   cd <that worktree> && pi --session-id <that id>
   ```

   Expected: Pi resumes **the same conversation**, with its history. This is FR-005a's whole point.
   Quit Pi.

5. Quit the application, restart it, reopen the project, resume the session. Expected: the same
   conversation continues; no fresh conversation is started under that identity (FR-005, SC-002).

6. Set a name from inside the session's own terminal — `/name my task` — early in the conversation.
   Expected: the sidebar row picks it up on the next label read (FR-011). Before that, the row shows
   the first thing you said; before *that*, the neutral placeholder.

7. Close the session in the application, then reopen the project. Expected: it is **not**
   rediscovered (FR-016), and the conversation file is still present and intact in Pi's store — the
   close left a marker, not a deletion:

   ```bash
   ls "$PI_CODING_AGENT_DIR"/sessions/--*--/     # <id>.jsonl AND <id>.archived
   ```

8. With the application closed, run `pi` by hand in that worktree, say something, quit, then reopen
   the project. Expected: that conversation appears as a session row alongside the ones you started
   here, with an `Unknown` badge and no observation of its storage (FR-015, FR-013).

---

## C. The activity component inside a real `pi` (Principle I recorded procedure)

The constitution's narrow GUI/process-spawn exception: this is the wiring no unit test reaches.
**Record the outcome of each step in the PR.**

1. **The badge moves.** Start a Pi session, send a prompt that takes a few seconds. Expected: the
   sidebar badge reads working within a second of the turn starting and waiting within a second of
   it settling, in the **same presentation and vocabulary** the other two CLIs use (FR-012, SC-005).
   A turn that thinks without writing anything still reads as working — that is what distinguishes a
   reported signal from an inferred one.

2. **Nothing but activity is written.** While that session runs:

   ```bash
   cat "$PI_CODING_AGENT_DIR"/micold-activity/<session-id>.jsonl
   ```

   Expected: only `{"type":"…","at":"…"}` lines. **No prompt text, no response text, no tool name, no
   file path, no model name** (FR-012b). This is the check that has to be re-run whenever the
   component changes.

3. **The user's own `pi` is untouched.** After the session has run:

   ```bash
   ls "$PI_CODING_AGENT_DIR"/extensions/ 2>&1      # expect: no such directory, or unchanged
   ls <worktree>/.pi/extensions/ 2>&1              # expect: no such directory
   git -C <worktree> status --porcelain            # expect: no new files
   ```

   Then run `pi` by hand in that worktree and confirm no component of this application loads and no
   activity log is written for it (FR-012a, FR-007).

4. **Declining works and is not a fault.** Turn the FR-012e switch off in Settings — confirm it is
   **one switch, application-wide**, beside the default-CLI setting, and appears nowhere per-project
   or per-session (FR-012e, SC-005a). Start a new Pi session. Expected: it runs normally; its badge
   reads unknown; the row is **not** marked failed, degraded, or needing attention (FR-012f); no
   `-e` appears in the process's argument list:

   ```bash
   ps -o args= -C pi                              # expect: pi --session-id <uuid>, and nothing more
   ```

   Existing sessions keep their recorded CLI. Turn it back on; the next session reports again.

5. **Failure degrades, it does not break.** With the switch on, make the component unloadable (point
   the daemon at a deliberately broken file, or delete it between materialisation and spawn). Expected:
   the session still starts and is usable; the badge reads unknown, **never idle**; nothing is
   presented as broken (FR-012d).

6. **No timer of ours.** Leave the application open with several Pi sessions, most of them idle, for
   a few minutes. Expected: no periodic wakeup and no per-idle-session work — the same observation
   feature 026 recorded for `copilot`, since the mechanism is the same `EventLogTail` (FR-014).

7. **Nothing is sent off the device.** Start a Pi session with no model provider configured and the
   machine offline. Expected: Pi starts, complains about its own missing provider in its own
   terminal, and the application makes no network attempt of its own. `PI_OFFLINE`,
   `PI_SKIP_VERSION_CHECK` and `PI_TELEMETRY` are visible in the session's environment
   (Principle IV):

   ```bash
   tr '\0' '\n' < /proc/$(pgrep -n pi)/environ | grep '^PI_'
   ```

---

## D. The sandbox

```bash
mise run image                 # build :dev from the working tree
mise run test-sandbox          # the real-runtime suite, both crates, release, one at a time
```

Expected: `crates/micold-daemon/tests/sandbox_real_ai_cli.rs` passes. It iterates `AiCli::ALL`, so it
covers `pi` the moment the variant exists — and **fails before the Containerfile ships it**, which is
this change's Test-First red (FR-017, FR-021).

Then, with sandboxed placement selected in the application:

1. Start a Pi session. Expected: it starts with nothing installed into the sandbox by the user, and
   the badge behaves exactly as it did on the host in §C.1 — the component travels with the session
   service, not with the image (FR-012a, FR-017a, SC-007a).
2. Substitute an image that lacks `pi`. Expected: reported where the user selects an image **and**
   where they select a CLI, naming both, and Pi is not offered at session start (FR-018).

---

## What "done" looks like

- §A and §B pass, and `no_concrete_implementations.rs` passed **without a new exemption**.
- §C is recorded, step by step, in the PR — including step 2's log dump.
- §D passes, and the image pin is an exact version.
- `docs/user-guide/` and `README.md` state the three things FR-022 requires outright: that a Pi
  session's conversation lives in Pi's own store and is resumable outside this application; that a Pi
  session loads a component of this application into Pi, what it reports and how to decline; and that
  the in-use warning is advisory — on Pi, in fact, never raised at all (contract §Known limitations).
