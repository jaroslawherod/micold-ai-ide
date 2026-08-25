# 014-hide-agent-worktrees T034 — quickstart Part 2, run for the first time

**Date**: 2026-08-21
**Run by**: an agent, not a person at a display — Xvfb `:83` at 1600×1400, Mesa lavapipe (software
Vulkan), driven with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.
**Build**: this branch's own `micold-ai-ide` + `micold-daemon`, built in one invocation and copied
out of the shared target directory **inside** the build lock (`~/vp83/bin`, 2026-08-20 21:03). The
newest commit touching `crates/` is `d28a0c6` (2026-08-19), so the pinned pair is this branch.
**Isolation**: `XDG_RUNTIME_DIR=/tmp/vp83`, a scratch `XDG_DATA_HOME`. Everything started here was
stopped by PID afterwards.

## Fixture

Three throwaway repos, built exactly as the quickstart prescribes:

- `repo` — two user worktrees (`feat-101-login`, `fix-102-timeout`), the decoy `agent-foo`, two
  machine-generated agent worktrees (`agent-a885b42dc521fbda1`, `agent-abf6a58b16c3c9e6f`), and the
  orphan directory `agent-ae474105b29fbeb68` that git does not know about.
- `agentonly` — one agent worktree and nothing else (scenarios 8, 9).
- `big` — 25 `feat-*` and 25 `agent-*` worktrees (scenario 11).

## Result

| # | Claim | Result |
|---|-------|--------|
| 1 | Exactly the three user rows; no agent worktree, no orphan | **PASS** — `Default`, `Agent foo`, `101 login` (feat), `102 timeout` (fix). `s1-hidden-by-default.png` |
| 2 | "Show agent worktrees" chip present, outlined (off), above the tag chips | **PASS** — `s2-reveal-chip-off.png` |
| 3 | Press → fills; the three agent worktrees join, each with a muted `agent` chip | **PASS** — `s3-revealed-with-agent-badges.png`. See the note on "unmoved" below. |
| 4 | Press again → agent rows go, user rows untouched | **PASS** |
| 5 | A tag filter is never cleared by the reveal toggle, and revealed rows obey it | **PASS** — `s5-tag-filter-survives-reveal.png` |
| 6 | Relaunch with the chip left **on** → off again, agent worktrees hidden | **PASS** (FR-010a) |
| 6a | Switch project with the chip on → off in the other project; switching back does not restore it | **PASS** (FR-010e) |
| 7 | Hover a revealed agent row → normal action cluster; delete confirm has no agent-specific warning | **PASS** — `s7-delete-confirm-no-agent-warning.png` |
| 7a | Every string this feature adds says "agent", never "assistant" | **FAIL, then fixed** — see below |
| 8 | Agent-only repo shows "No worktrees yet…", not "No worktrees match the filter." | **PASS** |
| 9 | The reveal chip is still present there, beside "No tags to filter yet." | **PASS** — `s9-agent-only-repo-filter-panel.png` |
| 10 | An agent worktree created externally while the app runs never appears | **PASS** (FR-009) |
| 11 | ~50 worktrees: the list renders immediately, no pause attributable to classification | **PARTIAL** — see below |

### Post-run check (SC-005, FR-008) — **PASS**

```
$ git worktree list | grep -v agent-c227d44ff743bcde2 | diff - worktrees-before.txt
$ git branch --list 'worktree-agent-*' | diff - branches-before.txt   # after the same exclusion
$ ls -d .claude/worktrees/agent-ae474105b29fbeb68
.claude/worktrees/agent-ae474105b29fbeb68
```

Both diffs are empty and the orphan directory survives. `agent-c227d44ff743bcde2` is the worktree
**I** created from a terminal for scenario 10; excluding it, the repository is byte-for-byte what it
was before the app ever opened it. The app pruned, removed, renamed and adopted nothing.

## Scenario 7a — the failure, and the fix

The application is clean: the chip reads "Show agent worktrees"
(`ui/sidebar.rs:272`), the badge reads `agent` (`ui/sidebar.rs:334`), and no user-facing string in
`micold-client` or `micold-core` contains "assistant".

The **user guide** was not. `docs/user-guide/worktrees-and-sessions.md` carried two occurrences:

> | `feat/login · in use by a hidden agent worktree` | Held by an **assistant's** worktree, which the sidebar hides by default. |

> - **A hidden **assistant** worktree** — one of the app's own, but not currently listed. Turn on
>   **Show agent worktrees** in the sidebar to see it.

Both came in later, with `b74dad4` ("tell the user where a held branch actually is" — feature 016's
held-branch message), not with this feature. The spec's Terminology section is unambiguous — *"no
user-facing string should say 'assistant'"* — and the user guide is user-facing; the second line
even sends the reader to a control it declines to name the same way. Fixed in this branch: both now
read "agent". That is the whole of the finding; there is nothing left to file.

This is exactly the kind of drift 7a exists to catch, and it only drifted **because** the procedure
had never been run.

## Scenario 3 — a wording note, not a defect

The quickstart says the user rows are "unchanged and unmoved". They are unchanged; they do *move*.
The list sorts by directory name, so revealing inserts `agent-a885…`, `agent-abf6…`, `agent-ae47…`
above `agent-foo`, `feat-101-login` and `fix-102-timeout`, which all shift down. No requirement asks
for anything stronger — FR-010b wants the badge, SC-003a wants one action — so this is the
quickstart's phrasing being tighter than the spec, not the app misbehaving.

The revealed orphan carries `invalid` **and** `agent`. Consistent: FR-007 hides it regardless of
health, and revealing shows it as what it is.

## Scenario 11 — what was and was not measured

With 50 worktrees the sidebar lists 25 rows and **zero** `agent-` rows, top to bottom
(`s11-fifty-worktrees-bottom.png`) — SC-001 and SC-002 at scale. Switching into the project, the
complete list is present in the first frame I can capture, ~0.6 s after the click, with no
intermediate empty or partial list.

That is not SC-004. SC-004 is a comparison — "no slower than **before the feature**" — and it needs
a pre-feature build to compare against; and `import` itself costs ~300 ms, so this pipeline cannot
resolve the sub-100 ms difference the claim is really about. Recorded as unmeasured rather than
passed.

## Harness artifacts (not app defects)

Two, both worth writing down because each cost several attempts and each looked like a defect:

1. **`xdotool click 1` is too fast for the filter button.** An instantaneous press+release on the
   funnel toggles nothing — repeatedly, reproducibly. `mousedown`, dwell 200 ms, `mouseup` opens it
   every time. A human's click always dwells; this is the synthetic input, not the widget. The same
   family as the `CursorMoved` artifact recorded in 014-forget-project's evidence.
2. **A tooltip covers the panel it just opened.** Clicking the funnel and leaving the pointer there
   renders "Filter worktrees" directly over the reveal chip. Moving the pointer clears it. Ordinary
   tooltip behaviour that only bites because a script does not move its pointer away.

## One finding outside this feature

Opening a project through a **symlinked path** classifies every one of its worktrees `invalid`.
Found here because the harness reached its fixtures through `~/.aaa-vp83 → /tmp/…`; the same repo
opened by its real path is entirely healthy. Filed as
[002 BUG-002](../../002-project-workspace-management/bugs/BUG-002.md), with the side-by-side frame.
Nothing to do with hiding agent worktrees — it just happened to surface here.
