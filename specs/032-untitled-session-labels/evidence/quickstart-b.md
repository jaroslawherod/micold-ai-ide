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
