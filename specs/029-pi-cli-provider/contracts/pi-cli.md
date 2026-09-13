# Contract: `pi` CLI Invocation

**Feature**: 029-pi-cli-provider | the **Pi Coding Agent provider profile** of the AI CLI provider
seam (FR-019, `specs/026-multi-provider-sessions/contracts/ai-cli-provider.md`).

The counterpart to `specs/005-worktree-session-terminal/contracts/claude-cli.md` and
`specs/026-multi-provider-sessions/contracts/copilot-cli.md`. Each section below is one seam
capability. Nothing outside the seam may depend on anything in this file.

External dependency contract. Derived from **`@earendil-works/pi-coding-agent` 0.85.1** — its
published documentation and its published source (`earendil-works/pi`, `main`, read 2026-09-12);
per-item sources are in [research.md](../research.md).

Every on-disk format here is *internal to Pi* and may change without notice: all reads are
best-effort and a parse failure degrades the affected capability rather than failing the session.

## Preconditions

- `pi` is on `PATH`. Absence is surfaced when starting a session: the session enters `Failed` with a
  message naming the CLI, and the application does not crash — the same path `claude` and `copilot`
  take (FR-004).
- No minimum version is gated on (FR-003a). `--session-id`, `-e` and the on-disk layout below were
  all present in 0.85.1. A failure report names the installed version rather than pre-empting it.
- Node ≥ 22.19.0 is Pi's own `engines` requirement, not ours; the application does not check it.

## Base directory

```
$PI_CODING_AGENT_DIR  if set and non-empty      # documented as "Override the config directory"
~/.pi/agent           otherwise                 # home-relative on every platform, Windows included
```

An empty `PI_CODING_AGENT_DIR` is treated as absent, and an unresolvable home directory yields
"uncertain" rather than "absent" — both matching `ClaudeProvider` and `CopilotProvider`.

**Windows is `%USERPROFILE%\.pi\agent`.** Pi is a JavaScript bundle with no per-platform resolver;
its default is `homedir()` joined with `.pi/agent` on all three platforms, so there is no
`%APPDATA%`/`%LOCALAPPDATA%` divergence to encode and no `cfg` arm to write.

`PI_CODING_AGENT_SESSION_DIR` and `--session-dir` also exist and relocate the session store
independently of the config directory. **Neither is used.** Relocating the store is what FR-005a
forbids, and honouring a user's own `PI_CODING_AGENT_SESSION_DIR` is a deliberate open item
(§Known limitations).

## Launch — fresh session *and* resume

```
cwd = <worktree path>                    # scopes the session to the worktree; Pi keys its store on it
env  TERM=xterm-256color
env  PI_OFFLINE=1  PI_SKIP_VERSION_CHECK=1  PI_TELEMETRY=0
pi --session-id <uuid>                   # app-generated UUID v4 → app owns the id
```

**One argument vector for both launch modes.** `pi --session-id <id>` is documented as *"Use exact
project session ID, creating it if missing"*: Pi looks the id up among the sessions recorded for this
cwd, opens it if found, and otherwise creates a new session carrying that id (emitting a one-line
warning on stderr, which is expected and not an error). So `launch_args(id, LaunchMode::Fresh)` and
`launch_args(id, LaunchMode::Resume)` are equal — the first provider for which that is true, and a
fact the seam accommodates without change.

- The id must match Pi's `assertValidSessionId`:
  `^[A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9])?$`. A hyphenated UUID v4 passes unchanged.
- `--session-id` may not be combined with `--continue`, `--resume` or `--session`. None are passed.
- **`--name` / `-n` is deliberately not passed.** Pi's conversation name belongs to the user and is
  what the sidebar reads (FR-005a, FR-011); writing an identity into it would put a UUID on every row
  this application started.
- **`--no-session` is deliberately not passed.** The conversation must persist in Pi's own store so
  the user can resume it with a bare `pi` (FR-005a).
- The three `PI_*` variables are per-launch environment, not configuration: nothing in the user's
  `~/.pi` is modified (FR-007) and their own `pi` is unaffected. They disable Pi's startup update
  check, its `pi.dev` version request, and install/update telemetry — Principle IV, by the same
  judgement that made `--no-remote` deliberate for `copilot`. Model traffic is untouched: that is the
  user's own configured provider and the reason they started the session.

When the FR-012e switch is **on**, the daemon appends the activity component to this vector at spawn
— see §Activity signal.

## Sessions recorded for a working directory (FR-015)

```
<base>/sessions/--<encoded cwd>--/            directory listing, *.jsonl only
```

The per-cwd directory name is Pi's own encoding: the absolute cwd with its leading separator
stripped and every `/`, `\` and `:` replaced by `-`, wrapped in `--`…`--`. For
`/home/u/proj/wt` that is `--home-u-proj-wt--`.

Each conversation is one file, named `<timestamp>_<session-id>.jsonl`, where `<timestamp>` is the
creation time with `:` and `.` replaced by `-`. The id is the part **after the first underscore**,
and it is the id `--session-id` addresses.

- Cost is one directory listing per location, no file opened (FR-015): discovery does not read
  conversations, only their names.
- A conversation Pi created on its own carries a UUIDv7 id rather than the application's UUIDv4;
  both parse, both are listed, and nothing distinguishes them on the row (FR-015).
- **Parentage is never read.** A forked conversation is an ordinary file in this directory and is
  listed on exactly the same terms as one started fresh; `parentSession` in the header and `parentId`
  on entries are not consulted (FR-015, and Out of Scope: the relationship is not depicted, the
  conversation still is).
- A missing or unreadable directory contributes nothing, never an error.

## Recorded-conversation detection

A conversation exists for `(cwd, id)` when the listing above contains a file whose id part equals
`id`. Pi writes the file — header line included — at session creation, so unlike `copilot`'s
lazily-created `events.jsonl` there is no "started but not yet recorded" window.

## Session label extraction (best-effort, FR-011/FR-017)

```
<base>/sessions/--<encoded cwd>--/<timestamp>_<id>.jsonl   →   bounded prefix read
```

Read a **fixed byte budget from the start of the file**, stopping at the last complete line, and
resolve in this order:

1. the **latest** `{"type":"session_info","name":"<name>"}` entry within the budget, trimmed,
   non-empty → `SessionLabel::Named`;
2. else the text of the **first** `{"type":"message"}` entry whose `message.role == "user"` → the
   same fallback Pi's own session picker uses (`SessionInfo.firstMessage`);
3. else `SessionLabel::Pending` — the existing neutral placeholder.

- The name is **best-effort by construction**. Pi's own semantics are "latest `session_info` wins",
  resolved by scanning the whole file; within a budget the application sees the latest one *in the
  prefix*. A name set at startup or early is found; a `/name` deep into a long conversation is not,
  and the row falls back — which FR-011 sanctions in as many words.
- The fallback, by contrast, is **exact**: the first user message is at the start of the file by
  definition, so step 2 always sees what Pi's picker would.
- The budget is what makes labelling cost the same for a conversation a minute old and one a year old
  (FR-011, FR-015, SC-006b). Pi's own `buildSessionInfo` streams every line of every file and
  accumulates the full message text; adopting that would make opening a busy project O(total
  conversation bytes), which is the specific cost this bound exists to refuse.
- The name Pi records is **never written by this application** (FR-005a): `--name` is not passed and
  no `session_info` entry is ever appended. What a user reads on a Pi row is Pi's or their own.
- A missing file, a truncated prefix, or an unparseable line NEVER fails the session; the label stays
  `Pending` and is re-read opportunistically, exactly like `claude`'s `ai-title`.

## Activity signal (FR-012, FR-014)

Pi reports busy/idle **only to code loaded into its own process**, so this is the one capability the
seam did not already have a shape for.

```rust
ActivitySource::Extension { log: <base>/micold-activity/<session-id>.jsonl }
```

### What the daemon injects at spawn

```
pi --session-id <uuid> -e <component path>
env MICOLD_PI_ACTIVITY_LOG=<base>/micold-activity/<session-id>.jsonl
```

- `-e, --extension <source>` accepts a path to a `.ts`/`.js` file **without installation** — Pi's
  documented form for exactly this. Nothing is written into `~/.pi/agent/extensions/` or
  `.pi/extensions/`, so the user's own `pi` is untouched (FR-012a, FR-007).
- `-e` extensions are **CLI scope**: they load without a project-trust decision, unlike
  `.pi/extensions/` (FR-012a).
- The runtime is `jiti`, a direct dependency of `pi-coding-agent` itself. **An image that carries
  `pi` carries everything the component needs** — no second thing to ship, pin or check (FR-017a).
- The component file is materialised by the **session service**, which is present wherever sessions
  run, so host and sandboxed placement report identically (FR-012a).
- When the FR-012e switch is off, the `-e` and the environment variable are simply not added. Nothing
  writes the log, no event arrives, the badge stays `Unknown` (FR-012d) and the session is otherwise
  unchanged and is not presented as a fault (FR-012f).
- **Pi treats any extension that fails to load as fatal and exits** — a missing path, a parse error,
  or a throw from the factory alike (observed on 0.85.1, recorded in
  [evidence/quickstart-C.md](../evidence/quickstart-C.md)). So FR-012d cannot lean on Pi degrading:
  the session service omits `-e` when it cannot write the file, and the component wraps its own body
  so an extension API it does not recognise costs the badge, never the session.

### What the component does

Subscribes to Pi's extension events and appends one line per event to
`$MICOLD_PI_ACTIVITY_LOG`. Nothing else: it does not read, alter or transmit conversation content,
reaches no destination but that file, and registers no tool, command, shortcut or flag (FR-012b).

Each line: `{"type":"<event>","at":"<rfc3339>"}`.

### What the daemon does with it

Tails the log with the existing `EventLogTail` — same directory watch, same append-only read from the
file's current end, same absence of any timer of ours (FR-014 is met by the mechanism that already
meets FR-019 for `copilot`) — and maps each line into the `ActivityEvent` vocabulary the `Activity`
state machine already consumes. **The state machine is unchanged.**

| Pi event | `HookKind` | Signal |
|---|---|---|
| `turn_start` | `UserPromptSubmit` | `Working` |
| `agent_start` | `PreToolUse` | `Working` |
| `tool_execution_start` | `PreToolUse` | `Working` |
| `tool_execution_end` | `PostToolUse` | *(no change — turn continues)* |
| `agent_settled` | `Stop` | `AwaitingInput` |
| `turn_end` | `Stop` | `AwaitingInput` |
| `session_shutdown` | *(termination)* | `Ended { reason }` |

The component writes `session_shutdown` only when Pi's own `event.reason` is `"quit"`. The other
reasons — `"reload"`, `"new"`, `"resume"`, `"fork"` — replace the conversation inside a process that
keeps running, and reporting them as an end would read a live session as finished.

Everything else Pi can emit (`session_start`, `message_start`/`update`/`end`, `tool_call`,
`tool_result`, `model_select`, `context`, `session_compact`, `ui_prompt_*`, …) is not subscribed to
and, if it ever appears in the log, is ignored. **Unknown types must be ignored, not rejected** — Pi
gains event types between versions.

Bounds:

- No log ⇒ `Unknown`. That is the honest state for a session with no reporter, and it is what the
  enum already documents.
- A dangling `turn_start` (process killed mid-turn) must not leave the badge `Working` forever; the
  daemon already knows the process is dead and that guard applies here unchanged.
- A session merely **discovered** gets no tail at all — no component, no log, no watch, badge
  `Unknown` (FR-013, SC-006).
- The log lives **outside** `sessions/`, so it can never be misread as a conversation by either the
  application's listing or Pi's own.

## Durable close/remove suppression marker (FR-016)

```
<base>/sessions/--<encoded cwd>--/<session-id>.archived     empty sentinel, app-owned
```

- Written best-effort when the user closes or removes a session; a failure never fails the caller.
- Never read or written by `pi` — our artifact in Pi's storage, so it survives the loss of the
  application's own store, exactly as `<uuid>.archived` does for `claude`.
- Checked by reconciliation: an id in the listing with a matching marker is never reconstructed.
- Cannot be mistaken for a conversation: discovery filters on `.jsonl`, and so does Pi's own listing.
- **Nothing of the user's is deleted or truncated.** The store is shared with their own `pi`, and
  closing a row here is not permission to remove history from it (FR-016).
- Not versioned: it is not part of the application's store and has no shape beyond present/absent.

## Not used

- `--session <path|id>` — resolves an *existing* file or a partial UUID and does not create.
  `--session-id` covers both modes; this would need a second, mode-dependent vector.
- `--session-dir` / `PI_CODING_AGENT_SESSION_DIR` — relocating or splitting the store is what FR-005a
  forbids. See §Known limitations for the read side of this.
- `--fork` — forking is Pi's own workflow and a forked conversation is listed like any other. The
  application never forks on the user's behalf (Out of Scope).
- `--name` / `-n`, `/name` — the conversation name is the user's (FR-005a, FR-011).
- `--no-session` — the conversation must persist in Pi's store (FR-005a).
- `-p, --print` — non-interactive mode; the session is a PTY the user talks to.
- Pi's RPC mode (`docs/rpc.md`) — the best-shaped signal source Pi offers, and the wrong shape for
  this feature: it would replace the PTY session model rather than observe it. The same reason 026
  declined Copilot's `--acp`, and a candidate for a later feature.
- `pi -r` / `--continue` — the application addresses conversations by id; a picker is a UI Pi owns.

## Known limitations

- **No liveness indicator exists.** Pi takes no lock and writes no pid or marker for a session; two
  `pi` processes on one conversation simply both append. The FR-006b best-effort check is therefore
  *unanswerable* for Pi and FR-006c's silence is the operative arm: no probe, no warning, proceed.
  FR-006a is unaffected — a collision between two of the application's **own** sessions is known from
  its own registry and stays a refusal. FR-022's documentation obligation is where this is stated
  plainly to the user.
- **A user's own `PI_CODING_AGENT_SESSION_DIR` is not honoured.** If the user has relocated their
  session store, the application looks in the default location and finds nothing to discover there;
  sessions it starts still work, because Pi resolves the store itself at launch. Reading the variable
  would be a one-line change to `config_dir`'s sibling derivation; it is left out until someone
  actually uses it, rather than guessed at.
- **A `/name` set deep into a long conversation is not shown.** By design — see §Session label
  extraction. The row falls back rather than paying an unbounded read.
- **Project trust** is not prompted for in a bare worktree: Pi asks only when the directory actually
  contains trust-requiring resources (`cwd/.pi`, `.agents/skills`). Where a project does carry them,
  the prompt is Pi's own, it appears in the session's own terminal, the user answers it there, and
  the sidebar must not meanwhile report the session as failed.
