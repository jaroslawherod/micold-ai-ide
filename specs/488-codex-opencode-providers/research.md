# Research: Codex CLI and OpenCode providers (#488)

Verified 2026-10-09 from the CLIs' public source and docs fetched through `raw.githubusercontent.com`
(`openai/codex` main, `sst/opencode` dev). `developers.openai.com`, `opencode.ai` and the npm
registry were unreachable from the planning container, so versions are pinned in the tasks unit.
Legend: **VERIFIED** (source named) / **ASSUMED** (fallback in force).

## Verification table

| # | Assumption | Status | Source / fallback |
|---|---|---|---|
| V1 | Codex binary is `codex` | VERIFIED | `codex-rs/cli/src/main.rs`: `override_usage = "codex [OPTIONS] [PROMPT]"` |
| V2 | OpenCode binary is `opencode` (npm `opencode-ai`) | VERIFIED | `sst/opencode` README install section, `docs/cli.mdx` |
| V3 | Codex start = `codex` in cwd; resume = `codex resume <SESSION_ID>` (UUID or name; `--last`, picker otherwise) | VERIFIED | `main.rs` `ResumeCommand` |
| V4 | OpenCode start = `opencode`; resume = `opencode --session <id>` (`-s`), `--continue` = last session | VERIFIED | `docs/cli.mdx` flag table (`--continue/-c`, `--session/-s`, `--fork`, `--prompt`) |
| V5 | Neither CLI accepts an externally chosen conversation id on a fresh start | VERIFIED for Codex (no such flag in `main.rs`; ids are minted `ThreadId`s); ASSUMED for OpenCode (no flag in docs; ids are `ses_…`, not UUIDs) | Fallback: **bind after start** (R1) |
| V6 | Codex records conversations at `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-<ts>-<thread-id>.jsonl`; `CODEX_HOME` defaults to `~/.codex` | VERIFIED | `rollout/src/recorder.rs` (`SESSIONS_SUBDIR`, path builder), `utils/home-dir/src/lib.rs` |
| V7 | First JSONL line is a `SessionMeta` carrying `id` and `cwd` | VERIFIED | `protocol/src/protocol.rs` `SessionMeta {id, cwd, timestamp, …}` |
| V8 | Codex first user turn is a `response_item`/`event_msg` line later in the same file, readable from a bounded prefix | ASSUMED (line shape not read) | Fallback: no label; the sidebar keeps its existing fallback |
| V9 | OpenCode stores sessions under `~/.local/share/opencode/` (`project/<slug>/storage/` per docs; current builds expose a database: `opencode db path`) | VERIFIED location, ASSUMED layout | `docs/troubleshooting.mdx`, `docs/cli.mdx` (`db`) |
| V10 | `opencode session list --format json` lists sessions (with project filter) and `opencode export [id]` dumps one as JSON | VERIFIED command, ASSUMED field names (`id`, `title`, `directory`, `time`) | `docs/cli.mdx`. Fallback: unbound, no label |
| V11 | Codex sign-in: `$CODEX_HOME/auth.json` (a keyring mode also exists) | VERIFIED | `login/src/auth/storage.rs` ("Expected structure for $CODEX_HOME/auth.json", `DefaultKeyringStore`) |
| V12 | OpenCode sign-in: `~/.local/share/opencode/auth.json` (written by `opencode auth login`) | VERIFIED | `docs/cli.mdx` login section, `docs/troubleshooting.mdx` |
| V13 | Activity signal | ASSUMED none usable: Codex `notify` fires on turn completion only (no busy edge); OpenCode has a server/plugin event stream whose shape was not verified | Fallback in force: `ActivitySource::None` → `Unknown` (FR-009) |
| V14 | Tool-server (MCP) binding per launch | ASSUMED (Codex `-c mcp_servers.…` override; OpenCode `mcp` config key, `docs/config.mdx` §MCP servers) — no per-launch injection verified | Fallback: `Unsupported{reason}`, logged (FR-011) |
| V15 | Folder-trust prompt | Codex ASSUMED to ask on untrusted dirs (`[projects."<path>"] trust_level` in `config.toml`); OpenCode ASSUMED to never ask | Codex: new `FolderTrust` arm, unrecorded trust ⇒ `create_session` prompt refused as `AsksTrust` (existing path, `ops.rs`); OpenCode: `NeverAsks` |

Residual risk: V8, V10, V14, V15 need a real CLI to confirm. The tasks unit starts with a probe task
that runs each against a real install when available and records the result in
`contracts/*.md`; every provider read is best-effort so a wrong guess degrades to the fallback.

## R1 — Both CLIs mint their own conversation ids (seam gap)

The seam assumes `launch_args(session_id: Uuid, mode)`: the app picks the id (claude, copilot, pi).
Codex and OpenCode cannot be given one (V5), and OpenCode's id is not a UUID.

- **Chosen**: keep every existing `(config_dir, cwd, session_id: Uuid)` seam signature and give
  Minted providers an **app-owned binding file** `<config_dir>/micold-bindings/<session-uuid>`
  holding the CLI's conversation id (a plain string; OpenCode's `ses_…` fits). The provider's own
  methods read it, so `has_recorded_conversation`, `read_title`, `read_label`, `mark_archived`,
  `is_archived`, `activity_source` and `last_activity` keep working unchanged for the daemon's prune
  sweeps (`state.rs` ~3539/3583/3779) and the client's `shell/persist.rs:117`, which has no daemon
  state. Seam additions (all **required**, no defaults — the trait's FR-021 rule; claude/copilot/pi
  and `FakeAiCliProvider` implement them explicitly): `identity() -> ConversationIdentity`
  (`AppAssigned` | `Minted`), `launch_args_in(config_dir, session_id, mode)` (AppAssigned providers
  delegate to `launch_args`; Minted ones look up the binding, bound ⇒ resume args, else fresh),
  `new_conversations(config_dir, cwd, since: SystemTime) -> Vec<ConversationRef>` (AppAssigned ⇒
  empty), `bind(config_dir, session_id, &ConversationRef) -> io::Result<()>`, and
  `sandbox_auth_file` (R5). The daemon, after spawning a Minted session, polls
  `new_conversations` (bounded: every 2 s for ≤60 s, then gives up unbound) and binds.
  No `provider_conversation` field in the session store; no wire change beyond the variants.
- **Concurrency (Principle II)**: with two pending sessions of one provider in one cwd, two new
  conversations may appear at once. Rule: bind only when exactly one candidate is unclaimed;
  otherwise leave unbound. Never guess ⇒ never resume another session's conversation (FR-006).
- Rejected: `--continue`/`--last` (resumes the most recent, may be another session's — violates
  FR-006); matching by timestamp heuristics alone; wrapper script that pre-creates a conversation;
  persisting nothing and always starting fresh (loses US3 for no reason).

## R2 — Reading OpenCode's store

- **Chosen**: a bounded `opencode session list --format json` / `export` call (timeout 2 s, session
  `PATH`, run off the UI path at bind and label time only), parsed best-effort.
- Rejected: link `rusqlite` (new C dependency, schema churn, Principle V/VI cost); read the legacy
  JSON file tree (current builds use a database, V9).
- Cost: a process spawn per bind/label, never per listing; unavailable CLI ⇒ nothing read.

## R3 — Reading Codex's store

Plain files, no spawn: list `sessions/**/rollout-*.jsonl` for candidate ids (id = trailing UUID of
the filename), confirm `cwd` from the first line (bounded 64 KiB prefix, like Pi's
`TITLE_BUDGET_BYTES`), derive label from the first user turn in the same prefix. Walk bounded to
the most recent day directories since spawn time. Archive marker: app-owned `<config_dir>/micold-bindings/<session-uuid>.archived`, never inside `sessions/` and never mounted in the sandbox.

## R4 — Activity

`ActivitySource::None` for both (V13). Rejected for v1: Codex `notify` (idle edge only ⇒ would show
a wrong state between turns), terminal-output heuristics (FR-009 forbids guessing).

## R5 — Sandbox sign-in is not per provider today (seam gap)

`CredentialLayout.ai_cli_auth` is a single `~/.claude/.credentials.json`; copilot and pi share no
sign-in. The spec's "the way existing providers do" therefore means *that one read-only file mount,
nothing else of the CLI's directory*. **Chosen**: seam method `sandbox_auth_file(home) -> Option<PathBuf>` (required; claude returns
its current `~/.claude/.credentials.json`, copilot and pi `None`, so behaviour is unchanged).
`CredentialLayout.ai_cli_auth: Option<PathBuf>` becomes `ai_cli_auth: Vec<PathBuf>`, filled by
`conventional()` from `AiCli::ALL`; the single `CredentialShare::AiCliAuth` toggle ("AI CLI
sign-in") governs all of them. Edits: `sandbox/mod.rs` (`path_for` returns a slice), `shell/sandbox.rs`
:105 (prune absent entries) and :476 (mount each), and the tests reading the field
(`sandbox_credentials.rs`, `sandbox_real_ai_cli_sessions.rs:89`, `shell/sandbox.rs` tests) are updated
mechanically to the Vec with claude's entry first, assertions otherwise unchanged. Each file mounts
read-only at the **same home-relative path in the sandbox home**; a relocated host `$CODEX_HOME`
(outside the home) is not shared — the sandbox CLI shows its login prompt (US5 AS3).
Codex: `$CODEX_HOME/auth.json` when under home else `~/.codex/auth.json`; OpenCode:
`~/.local/share/opencode/auth.json`. Keyring-only hosts share nothing. Read-only ⇒ a token refresh
inside the container cannot persist, as for claude; documented.
Rejected: mounting the whole `~/.codex` / data dir (carries session store and history; FR-013).

## R6 — Wire protocol

`AiCli` is wire-visible: bump `PROTOCOL_VERSION` 38 → 39 once, with the `schema_hash.rs` pins and
`protocol/hashing.rs` updated in the same edit (feature 029 precedent). Binding files are not on the wire, so the bump is
only for the variants.

## R7 — Sandbox image

Add `@openai/codex` and `opencode-ai` to the existing pinned `npm install -g` layer in
`packaging/sandbox/Containerfile` (ARGs `CODEX_CLI_VERSION`, `OPENCODE_CLI_VERSION`, pinned in the
tasks unit). `crates/micold-daemon/tests/sandbox_real_ai_cli.rs` iterates `AiCli::ALL`, so it covers
both once the variants exist. Rejected: curl-installer scripts (unpinned, not reproducible);
OpenCode's native binary (glibc/arch matrix) — npm package already pulls the right one.

## R8 — Offline / telemetry (Principle IV)

Not verified for either CLI; the tasks unit records any update-check or telemetry environment
switch found during the probe in `launch_env()` (Pi's R9 precedent). Default: none set.
