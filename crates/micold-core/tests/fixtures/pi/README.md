# Pi fixture corpus (feature 029, T001/T002)

Every test that reads Pi's session store reads *these* files through `tests/support::pi_home()`, so
no test needs `pi` installed and none can touch a developer's real `~/.pi/agent`.

**Provenance, stated plainly: these files are authored, not captured.** `pi` is not installed on the
machine this corpus was written on, so every line here is written to the format documented for
**`@earendil-works/pi-coding-agent` 0.85.1** — `packages/coding-agent/docs/session-format.md` and
`src/core/session-manager.ts`, the sources research R5 cites — rather than recorded from a run. The
copilot corpus beside this one *was* captured; this one is not, and a reader should not assume
otherwise. `quickstart.md` §A is where the layout is confirmed against a real `pi`; if it disagrees
with anything here, the real `pi` is right and these files are the bug.

The files are stored under **logical names**, not under the layout Pi uses. The layout is a function
of the working directory (`sessions/--<encoded cwd>--/<timestamp>_<session-id>.jsonl`), and a test's
cwd is a fresh temporary directory, so the helper materialises the corpus into a scratch
`PI_CODING_AGENT_DIR` and derives the per-cwd directory name for whatever cwd the test chose.

## Two placeholders the helper substitutes

| In the file | Becomes |
|---|---|
| `/fixture/worktree` | the temporary directory the test is using as the session's cwd |
| `ffffffff-ffff-4fff-8fff-ffffffffffff` | the session id the test materialised the conversation under, so the header's `id` agrees with the filename's |

Both are substituted textually by `tests/support`, the same way `CopilotHome` rewrites its own
fixture cwd. The files stay valid JSONL on disk either way, which is what keeps them readable.

## The padding entry

`conversation-named-past-bound.jsonl` carries one line of a type Pi never writes:

```json
{"type":"micold_fixture_padding", …}
```

The helper replaces that single line with enough filler entries to push everything after it well
past the label read's byte budget. Storing ~256 KiB of filler in git to say "this name is far away"
would be noise; generating it at materialisation time says the same thing in one line. The filler
entries are of the same off-contract type, so a reader that ignores unknown types — which
`contracts/pi-cli.md` requires — sees nothing but distance.

## What each file is

| File | What it is | Expected label |
|---|---|---|
| `conversation-named.jsonl` | v3 header, a first user message, then **two** `session_info` names | the **second** name — latest within the prefix wins |
| `conversation-unnamed.jsonl` | Header and messages, no `session_info` ever. The user message's `content` is a bare string rather than a parts array — both shapes occur | the first user message's text |
| `conversation-named-past-bound.jsonl` | A name, but only after the padding described above | the first user message's text — the late name is **not** read (FR-011's bound) |
| `conversation-header-only.jsonl` | The header line and nothing else — a conversation Pi created for a session the user has not spoken in yet | `Pending` |
| `conversation-unparseable-lines.jsonl` | A line that is not JSON, a blank line, an off-contract type, and two `session_info` entries whose names are empty and whitespace | the user message's text — none of the above is an error |
| `conversation-truncated.jsonl` | Ends mid-line, with no trailing newline, the way a file being appended to while it is read does | the user message's text — the incomplete last line is dropped |

## What is deliberately absent

- **No activity-log fixture.** `micold-activity/<session-id>.jsonl` is written by the component this
  application supplies, not by Pi, so it is not part of Pi's store and its fixtures live with the
  daemon tests that map it.
- **No index file of any kind.** Pi has none: a working directory's conversations *are* its
  directory listing, which is why discovery costs one listing and opens nothing
  (`contracts/pi-cli.md`, FR-015).
