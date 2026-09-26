# Quickstart Part B — evidence

Recorded per milestone by the task that runs the section (T046 for §B1 `claude` rows and §B2;
T040 for §B1 Copilot rows and §B3). Machine: the development machine, Linux, `claude` 2.1.2xx.

## M1 — §B1 (`claude` rows) — 2026-09-22

Command:

```bash
MICOLD_LABEL_CORPUS=1 scripts/build-lock.sh cargo test -p micold-core \
  --test first_turn_label_corpus -- --ignored --nocapture
```

Result: **PASS**. `83 transcripts: 67 titled, 16 labelled, 0 neither`.

The four BUG-001 sessions, each reading its own first turn (SC-001, SC-004, SC-005):

| Session | Label printed |
|---|---|
| `9a536c7e` | `/speckit-autopilot` |
| `e0912fd9` | `Currently when creating new worktree that for example has multiple git submodul…` |
| `2dc1bd13` | `I would like to start pi coding session in same way as I start copilot or claud…` |
| `f2e8ef75` | `When running sessions withing sandbox it displays a warning that session files …` |

Per step:

- four different labels, `9a536c7e` reading `/speckit-autopilot` — **pass** (SC-001, SC-005).
- no conversation with a typed prompt prints "neither": `0 neither` over all 83 — **pass** (SC-002).
- the Copilot half of §B1 is M2's (T040): not run here.

Note: 12 of the 16 labelled transcripts are untitled conversations outside the reporter's project;
they are listed in the probe output and each reads its own first typed turn. No transcript was
modified: the probe opens every file read-only.

## M1 — §B2 (the sidebar after a restart) — 2026-09-22

Run with the repo's `visual-pass` skill (no human at the display): the client and daemon built from
this branch, on a private Xvfb display with a private `XDG_RUNTIME_DIR` and `XDG_DATA_HOME`. The
user's own app and service were left running and untouched, so step 1's "stop the background
service" was met by running a **private** instance instead of stopping the user's — the deviation
that matters (a daemon that has never seen these sessions, reading the CLI's records from cold) is
preserved: the private data dir was seeded with a copy of the real catalog
(`projects/8d4b6897c9929be6.json`, unmodified) and the transcripts under `~/.claude/projects` were
read read-only.

| Step | Result | What was seen |
|---|---|---|
| 1 — open the project without opening a session | **pass** | the rows appear with the list; no session was opened |
| 2 — rows read names or labels, four different ones, the titled one its title | **pass** (tooltip sub-check: see below) | with every worktree group expanded: `/speckit-autopilot` (9a536c7e), `say hi`, `say ok`, `Currently when creatin…` (e0912fd9), `I would like to start pi c…` (2dc1bd13), `When running sessions…` (f2e8ef75), `Dependency updates`, `Old copilot summary`. **No row anywhere reads "New session."** Each label is one line, ellipsised, not wrapped, not overflowing, and drawn exactly like a title (D7) |
| 3 — restart, same labels, no "New session" flash | **pass** | the private daemon and client were restarted and screenshotted from the first frame the window appeared; every frame, the first included, already carried the label |
| 4 — SC-006, the list appears no later by eye | **not run** | needs a ~50-session project with 1,000-record transcripts, which this machine does not have. The design bound stands as the argument (research R7: one ≤1 MiB read per untitled session, once, then persisted); there is no automated timing test by design |

Screenshots: `quickstart-b2-expanded-rows.png` (step 2, every group expanded),
`quickstart-b2-before-restart.png`, `quickstart-b2-restart-first-frame-no-flash.png` and
`quickstart-b2-after-restart.png` (steps 1 and 3).

### Tooltip sub-check — fail, and pre-existing

Step 2 also asks for the tooltip on hover to show the same text. **No tooltip appears on a session
row at all**, for a label or for a title: `session_tree_item()` in
`crates/micold-client/src/ui/sidebar.rs` never calls `.row_tooltip(..)`, while worktree rows do.
That file is identical to `main`, so this is not a regression of feature 032 and it does not break
D7 — a label and a title are treated exactly alike, which is what the spec requires. Recorded as a
follow-up in `autopilot.md`; giving session rows a tooltip is new behaviour and belongs to a spec,
not to this milestone.

## M3 — §B1 (Copilot rows) — 2026-09-26

Command (the same probe; it reports both providers):

```bash
MICOLD_LABEL_CORPUS=1 scripts/build-lock.sh cargo test -p micold-core \
  --test first_turn_label_corpus -- --ignored --nocapture
```

Result: **PASS**.

- `286 copilot sessions: 139 titled, 2 labelled, 145 neither`. The 139 titled include the ~44
  `summary:`-only sessions from Copilot 1.0.36 and earlier, which read as untitled before this
  feature (FR-016). The 145 "neither" are conversations with no typed turn of their own — Copilot
  writes a session directory before the user says anything — so SC-002 holds: no conversation with a
  typed prompt reads "neither".
- `78 transcripts: 61 titled, 17 labelled, 0 neither` for `claude`, with `9a536c7e` reading
  `/speckit-autopilot` and the other three BUG-001 sessions reading their own first turns. The
  counts differ from M1's (`83: 67/16/0`) only because the machine's own store moved on between the
  two runs.
- As **evidence only** (D10): the probe's Copilot half walks every session directory on the machine,
  listed or not, so the old unlisted `summary:` sessions are counted above. SC-008 and SC-009 are
  judged on *listed* sessions by the fixture tests in Part A (`copilot_provider.rs`,
  `untitled_session_labels.rs`), because none of the 44 is in a Copilot per-cwd index.

No file was modified: the probe opens every record read-only.

## M3 — §B3 (a running session) — 2026-09-26

Run with the repo's `visual-pass` skill, as §B2 was: a private Xvfb display (`:79`), a private
`XDG_RUNTIME_DIR` and `XDG_DATA_HOME` under `/tmp/vp79`, a seeded scratch project, and the client
and daemon built from this branch and copied to a private directory before launch (never run out of
`target-shared/`). The user's own app, service and sessions were left running and untouched. The
pinned pair was verified to connect, and the fix's line confirmed live in the build
(`state.rs` `drain_signals`, `live.name_stale = true` in the `SpinnerObserved` branch).

One environment fix was needed: the nested `claude` inherited `CLAUDE_CODE_CHILD_SESSION` and
reported transcript saving as off; relaunching with `CLAUDE_CODE_FORCE_SESSION_PERSISTENCE=1` fixed
it.

| Step | Result | What was seen |
|---|---|---|
| 1 — `claude`: type a prompt, do not let it title the conversation | **pass, with a caveat** | the row left "New session" well inside the 60 s bound, without reopening the project or restarting anything. Over 7 trials (including freezing the child with `SIGSTOP` to widen the window) `claude` 2.1.283 titled the conversation in 0.4–3.5 s, so no frame was caught showing the raw prompt text *distinct from* the generated title: the row went "New session" → title. The 60 s bound and "never stuck on New session" are demonstrated; the literal label frame for `claude` is not, and step 3 catches it on the same code path |
| 2 — let it title itself; restart | **pass** | the row switched to `claude`'s own title (`Repository explanation`), and after the private daemon and client were restarted the first frame already read that title — no "New session" flash |
| 3 — Copilot, before it writes `name:` | **pass, cleanly observed** | `workspace.yaml` confirmed `user_named: false` with no `name:`. While Copilot was still working the row read the derived label `Explain what this repo…`; when `workspace.yaml` gained `name: Summarize Repository Functionality` the row switched to it. This is the label-then-title transition step 1 could not isolate, on the same daemon path |

Screenshots: `quickstart-b3-step1-before.png`, `quickstart-b3-step1-after.png`,
`quickstart-b3-step2-after-restart.png`, `quickstart-b3-step3-copilot-before.png`,
`quickstart-b3-step3-copilot-label.png`.

Note on step 1's caveat: it is a property of this `claude` build, not of the change. `claude` 2.1.283
names a conversation within a few seconds, so its untitled window is short — which is exactly why
the feature's headline population is *past* sessions (§B2) and why the label path is proved on
Copilot here, where the untitled window lasts as long as the first turn.
