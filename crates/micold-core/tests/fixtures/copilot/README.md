# Copilot fixture corpus (feature 026, T001/T003)

Captured from **GitHub Copilot CLI 1.0.80** on Linux, 2026-08-20. Every test that reads Copilot's
store reads *these* files through `tests/support::copilot_home()`, so no test needs `copilot`
installed and none can touch a developer's real `~/.copilot`.

The files are stored under **logical names**, not under the layout Copilot uses. The layout is a
function of the working directory (`sidebar-sessions-state/<sha256_hex(cwd)>.json`), and a test's
cwd is a fresh temporary directory, so the helper materialises the corpus into a scratch
`COPILOT_HOME` and computes the index filename for whatever cwd the test chose.

## The recorded hash vector (T003)

`contracts/copilot-cli.md` says the index filename is the lowercase hex SHA-256 of the working
directory string exactly as Copilot recorded it. This is the vector that pins it — produced by
Copilot itself, not by us:

| working directory | `sha256_hex` |
|---|---|
| `/tmp/copilot-hash-probe` | `75980abda9809593b6cc1c6005b85aca235c3f973c6afac4f9a2ea707710dd98` |

Copilot wrote `sidebar-sessions-state/75980abda9…dd98.json` for a session started in that
directory. `tests/copilot_provider.rs` asserts the workspace's own
`protocol::hashing::sha256_hex` reproduces it, so a change to the hashing helper is caught here
rather than silently orphaning every recorded session. Cross-referenced from
`specs/026-multi-provider-sessions/contracts/copilot-cli.md`.

## Version note

Research R1–R6 verified this layout against **1.0.62**; the capture above re-verified it against
**1.0.80** and every documented shape still holds — the index filename, `schemaVersion: 1`, the
`cwd`/`sessionIds` keys, `session-state/<uuid>/workspace.yaml`'s `name:` key, and
`session-state/<uuid>/events.jsonl`.

One thing worth knowing, because it made the index look absent at first: **the index is written by
the interactive TUI, not by `-p` non-interactive mode.** A `copilot -p …` run creates
`session-state/<uuid>/` and its `events.jsonl` but no `sidebar-sessions-state/` entry at all. This
application always spawns the interactive CLI in a PTY, so the index is always written for the
sessions it starts — but a probe script that uses `-p` will conclude the file does not exist.

## What each file is

| File | What it is | Captured? |
|---|---|---|
| `index-well-formed.json` | A per-cwd index listing three ids, deliberately **not** in sorted order (`A`, `C`, `B`) so "ids in index order" is a real assertion | shape captured, ids substituted |
| `index-schema-version-2.json` | `schemaVersion: 2` — Copilot's version, not ours; contributes no sessions rather than being parsed hopefully | authored |
| `index-truncated.json` | Cut off mid-array — contributes nothing, never an error | authored |
| `index-empty.json` | Zero bytes — same | authored |
| `workspace-named-plain.yaml` | `name:` as a plain scalar | captured shape |
| `workspace-named-quoted-colon.yaml` | `name: 'Reply with the single word: hello'` — single-quoted **and** containing a colon, the case a naive `split(':')` reader gets wrong | captured verbatim from a real session |
| `workspace-named-double-quoted.yaml` | The double-quoted form | authored |
| `workspace-unnamed.yaml` | No `name:` key — a session Copilot has not summarised yet; the label stays `Pending` | captured verbatim (a session started and given no prompt) |
| `events-full-turn.jsonl` | A whole turn: `session.start` → `user.message` → `assistant.turn_start` → `permission.requested` → `tool.execution_start` → `tool.execution_complete` → `assistant.turn_end` → `session.shutdown`, with six off-contract types mixed in | captured, with two edits below |
| `events-dangling-turn.jsonl` | `assistant.turn_start` with no matching `turn_end` — the process died mid-turn | captured (truncated from a real log) |
| `events-unknown-and-malformed.jsonl` | Off-contract types, a line that is not JSON, a blank line, and `session.error` | captured, plus one authored line |

### The two edits to `events-full-turn.jsonl`

Both are recorded here rather than left for a reader to notice:

1. **`permission.requested` is authored**, to the envelope shape every other line uses. It cannot be
   captured non-interactively — `--allow-all-tools` is required for `-p` mode and it is exactly the
   flag that stops Copilot asking. `session.error` in `events-unknown-and-malformed.jsonl` is
   authored for the same kind of reason: it needs an upstream failure to occur.
2. **The `system.message` body is elided.** The captured line was 27 KB of Copilot's own system
   prompt — the vendor's text, off-contract, and ignored by the mapping either way. The envelope is
   verbatim; `data` is a placeholder.

Everything else — including the session/tool/turn ids, timestamps and `data` payloads — is as
Copilot wrote it, with the probe's session uuid and working directory rewritten to the fixture's
(`/fixture/worktree`).
