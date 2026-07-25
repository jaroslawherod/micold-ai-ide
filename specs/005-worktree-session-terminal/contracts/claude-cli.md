# Contract: `claude` CLI Invocation

**Feature**: 005-worktree-session-terminal | consumed by the `TerminalBackend` impl.

> **Bugfix BUG-002 (2026-07-17)**: As of FR-024, the AI CLI is an abstract **AI CLI provider**.
> This document is the concrete **`claude` provider profile** of that abstraction — the default and
> only provider this version. Each section below maps to a provider-seam capability: launch,
> resume, session-id ownership, transcript location + encoding, recorded-conversation detection,
> and **session-title extraction** (the "Session label extraction" section is the contract the
> title-sync reader of T064 implements). A future provider supplies its own profile without
> changing the session model, persistence, sidebar, or terminal wiring.

External dependency contract. Verified against Claude Code **v2.1.210** (research R6). The app
MUST detect the CLI version and degrade gracefully; the on-disk JSONL format is *internal* and
all file reads are best-effort.

## Preconditions

- `claude` is on `PATH`. Absence is surfaced as an error when starting a session (assumption in
  spec); the session enters `Failed` with a clear message rather than crashing the app.
- Version detected via `claude --version`. Feature gates:
  - `--session-id <uuid>` requires **v2.1.210+**.
  - `ai-title` session titles require **v2.1.205+**.

## Launch — fresh session

```
cwd = <worktree path>                       # scopes the session to the worktree (R6)
env  TERM=xterm-256color
claude --session-id <uuid>                  # app-generated UUID v4 → app owns the id
```

Fallback (CLI < 2.1.210, no `--session-id`): launch `claude` in the worktree, then discover the
id by watching `~/.claude/projects/<encoded-cwd>/` for the new `<uuid>.jsonl` (or a one-shot
`-p --output-format json` read of `session_id`). Documented but secondary.

## Launch — resume (restart, crash-restart, project reopen)

```
cwd = <worktree path>
claude --resume <uuid>                       # specific session, non-interactive (R6)
```

- Missing id → `claude` errors (`No conversation found`); surface it and offer a fresh session.
- `--continue`/`-c` (most-recent-in-cwd) is NOT used — we always target a specific id.

## Session label extraction (best-effort, FR-011a)

- Path: `~/.claude/projects/<encoded-cwd>/<uuid>.jsonl`, where `<encoded-cwd>` is the full
  worktree path with non-alphanumerics replaced by `-` (respect `CLAUDE_CONFIG_DIR`).
- Read the latest `{"type":"ai-title","aiTitle":"…"}` record → `SessionLabel::Named`.
- Until present → `SessionLabel::Pending` (placeholder). Re-read opportunistically (title grows
  with the conversation). A failed/absent read NEVER fails the session — label stays `Pending`.

## Durable close/remove suppression marker (bugfix BUG-003, 2026-07-23)

- Path: `~/.claude/projects/<encoded-cwd>/<uuid>.archived` — an empty sentinel file, sibling to
  the session's `<uuid>.jsonl` transcript, same `<encoded-cwd>` scheme as the transcript path
  (respect `CLAUDE_CONFIG_DIR`).
- Written (best-effort, never fails the caller) when the user **closes** or **removes** a session
  (FR-015a/FR-015c). Never read or written by `claude` itself — this is an app-owned artifact
  that happens to live alongside `claude`'s own data so it survives independently of the app's
  own store (`projects.json`, the per-project state file).
- Checked by reconciliation (FR-020b/FR-020c): an orphan transcript with a matching `.archived`
  marker is never reconstructed as a session, regardless of what the app's own persisted store
  currently contains (including containing nothing at all).
- The `.jsonl` extension filter `discover_transcript_session_ids` (T065) already applies means a
  `.archived` file is never misread as a transcript / session id source.
- Not versioned by `schema_version` — it isn't part of `projects.json` at all; it's provider-side
  storage, addressed purely by session id, with no shape to evolve beyond "present or absent."

## Cwd invariant

`claude` uses its process cwd as project context; there is no `--project-dir` flag. The backend
MUST set cwd to the worktree for both fresh and resume launches.

## Non-goals

- No `-p`/`--print` headless mode (we want the interactive TUI in the terminal).
- Model selection (`--model`) is out of scope for this feature (defaults apply).

## Constitution mapping

Principle II (session identity/restore), IV (app state local; `claude`'s own network use is
external tool behavior, not app-persisted state).
