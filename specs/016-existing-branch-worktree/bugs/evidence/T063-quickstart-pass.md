# T063 — the seven quickstart scenarios, run against the built app

**Date**: 2026-08-18
**Result**: all seven **PASS**, including Scenario 4's offline check.
**Run by**: an agent, not a person at a display — Xvfb `:79` at 1600×1400, rendered by Mesa's
lavapipe (software Vulkan), driven with `xdotool`, captured with `import`. Per the repo's
`visual-pass` skill. Read *What this pass does not answer* before treating it as a substitute for
someone looking at the screen.
**Build**: this branch's own `micold-ai-ide` and `micold-daemon`, each built by its own
`cargo build -p …` invocation and copied to `~/vp16/bin` before launching, with the pair verified to
agree (`strings … | grep included_worktrees` → daemon 3, client 4) — see *Build hazard* below.
**Isolation**: `HOME`, `XDG_RUNTIME_DIR=/tmp/vp79` and `XDG_DATA_HOME` pointed at scratch
directories, so the app spawned its own daemon on its own socket with its own catalog. The user's
running daemon (PID 299344, `XDG_RUNTIME_DIR=/run/user/1000`) was never contacted and was verified
still up afterwards; everything this pass started was stopped by PID.

## Fixture

`quickstart.md`'s recipe verbatim in `/tmp/wt-016`, with two deliberate departures noted under
Scenarios 2 and 3 — both because the recipe as written makes its own check vacuous.

## Scenario-by-scenario

### 1 — Reuse an existing branch (US1, FR-001/002/004) — **PASS**

`t063-s1-conflict-crop.png`: creating `feat/reporting` pauses and names the branch as already
existing, offering **Reuse**, **Overwrite**, **Cancel**. Not the old "A branch with that name
already exists."

**Cancel** returned to the form with `feat` and `reporting` still filled in (FR-007). **Create** →
**Reuse** produced the sidebar row, and
`git -C /tmp/wt-016/.claude/worktrees/feat-reporting log --oneline` contains `OUTSIDE-WORK` — the
history survived. A session started on it and behaved like any other worktree (FR-023).

### 2 — Overwrite a stale branch (US2, FR-005/006) — **PASS**

**Departure from the recipe.** The fixture creates `feat/stale` at `main`, so step 4's
`rev-parse feat/stale main` is already equal before anything is overwritten — the check cannot fail.
`feat/stale` was moved to `feat/reporting`'s tip first, so the two commits genuinely differ going in.

`t063-s2-warn-crop.png`: **Overwrite** raises a second, explicit warning naming the branch and
stating its commits will be discarded. **Back** returned to the reuse/overwrite choice with nothing
changed (US2 AS3). **Overwrite** → **Confirm** created the worktree, and `rev-parse feat/stale main`
then reported one commit — reset to `main`'s tip, as specified.

### 3 — Reuse rollback must not delete the branch (FR-008, SC-003) — **PASS**

**Departure from the recipe.** As written this scenario is unreachable: pre-flight now catches a
pre-existing target directory *before* the branch prompt, so **Reuse** is never offered and creation
never gets far enough to roll back. The clash was instead created while the branch prompt was
waiting, which reaches the real post-pre-flight failure path the scenario is about.

`t063-s3b-fail-crop.png`: creation fails with a directory-clash message. Critically,
`git branch --list feat/reporting-2` still lists the branch and `git log feat/reporting-2` still
contains `OUTSIDE-WORK` — the rollback undid the worktree, not the branch.

### 4 — Continue from a remote-only branch (US4, FR-016/017/020) — **PASS**

`t063-s4-crop.png`: the panel identifies `feat/from-elsewhere` as a branch on **origin** and offers
**Continue from origin** and **Start fresh at HEAD**, with the divergence warning on the latter
(FR-018) and the remote-staleness note visible. After **Continue from origin**:

```
rev-parse feat/from-elsewhere origin/feat/from-elsewhere   # same commit
rev-parse --abbrev-ref feat/from-elsewhere@{upstream}      # origin/feat/from-elsewhere
```

**Offline check (FR-020, Principle IV)** — `t063-s4-offline-crop.png`: with
`/tmp/wt-016-remote.git` moved aside and the app restarted, the panel still appeared, in under four
seconds, sourced from local `refs/remotes`. No hang, no network error, no fetch.

### 5 — Pick a branch from the list (US2, FR-010–FR-015) — **PASS**, and it is where BUG-002's fix shows

`t063-s5-blocked-crop.png`: local and remote branches listed, remote rows carrying their remote,
checked-out rows carrying `· in use by …`, staleness note present.

The blocked row `main · in use by the project checkout` was again drawn **outside** the dialog card,
over the window background — the geometry BUG-002 needed. Pressing it left the form open, the list
open, and nothing selected. That is the fix working in the shipped binary, not in a test harness.

Selecting an available branch cleared the explanation, re-enabled **Create**, and showed the derived
`.claude/worktrees/<name>` preview (FR-014). **Create** produced a worktree on exactly that branch
with its history intact. Switching back to **New branch** returned the new-branch inputs with no
leftover selection (FR-015).

### 6 — Branch already checked out (US5, FR-021) — **PASS** on what is reachable

Steps 1–2 as written are not reachable: the new-branch form derives `<prefix>/<name>`, so `main`
cannot be produced through it. The equivalent state — `main` selected and refused with no reuse or
overwrite offered — is what Scenario 5 step 3 exercises above, and it holds.

Steps 3–4 ran as written against a holder worktree, `t063-s6-holder-crop.png`:

> 'feat/reporting-3' is already checked out in the worktree 'zz-holder'.

Only **OK** is offered; the holder is named; nothing changed on disk.

### 7 — No regression for a free name (FR-025, SC-008) — **PASS**

`t063-s7-sidebar.png`: `chore` / `something-new` created immediately, with no extra dialog and the
same steps as before the feature.

## Findings this pass surfaced

None of these block T063 — the scenarios' own expectations all held — but no geometry gate or unit
test can see any of them, which is the reason a pass like this exists. All three are diagnosed in
[BUG-003.md](../BUG-003.md); the summaries here are what was *seen*, the report has the traces.

1. **The search magnifier is drawn on top of the "B" of the "Branch" label** in the collapsed, empty
   picker (`t063-s5-label-zoom.png`, 600% zoom). `FormField` positions the resting label without
   knowing the inner input carries a leading icon, so every searchable picker built this way
   overlaps them.
2. **A worktree on `feat/reporting-2` renders as "Feat reporting 2" with a bogus `REPORTING-2`
   issue tag** — *fixed 2026-08-18, see BUG-003* —, while `feat/reporting` renders correctly as "Reporting". First noted here as a
   divergence between the picked-branch and typed-name creation paths; it is not — both call the
   same function, and the trailing `-2` is the whole of the difference. It is misread as an issue
   key, which empties the descriptive remainder and makes `display_name` fall back to the whole
   directory name, type token included (008 FR-017).
3. **The directory clash has two hand-written wordings** depending on where it is caught —
   pre-flight: "A worktree folder named 'X' already exists. Choose a different name, or remove the
   existing folder first."; daemon: "a worktree with that name already exists". Only the first tells
   the user what to do, and the second is what Scenario 3 shows. The arm directly above it in the
   same `match` carries a comment explaining why this exact duplication was removed after BUG-001.

Also looked at and **not** a defect: the picker field never shows the selected branch, only the
derived preview does (`t063-s5-field-zoom.png`). That is 021 FR-014a as specified.

## Drift found in the tooling around this task

- **`quickstart.md`'s automated gate is stale.** It names `cargo test --features gui` and
  `cargo clippy --features gui --all-targets`; the `gui` feature no longer exists after the
  workspace split into `micold-core` / `micold-client` / `micold-daemon`. Corrected in this change
  to what CI actually runs: `mise run test`, `cargo fmt --all -- --check`, and
  `cargo clippy --workspace --all-targets`. The same stale commands are
  quoted in tasks T060–T061, which are already checked off and are left as the historical record.
- **The `visual-pass` skill's build snippet silently builds one binary.**
  `cargo build -p micold-client --bin micold-ai-ide -p micold-daemon` builds *only* the named bin —
  `-p micold-daemon`'s binary is never built, so `target-shared/debug/micold-daemon` is whatever
  another worktree left there. That is exactly the mismatch BUG-002's pass recorded, reintroduced by
  the fix that was supposed to prevent it. Two separate `cargo build` invocations inside one build
  lock is the working form.

## What this pass does **not** answer

- **Mid-flight animation.** The dialog and list transitions are 150–300 ms; a screenshot pipeline
  cannot reliably catch a chosen frame. Whether they *look* right is unverified.
- **Frame pacing.** lavapipe is a software rasteriser; nothing here speaks to smoothness on a GPU.
- **Light scheme.** Everything above was captured in the dark scheme only.
- **Scenario 3's pre-flight interception** and **Scenario 6 steps 1–2** as literally written — both
  preconditions are no longer reachable in the current app, and equivalent states were constructed
  instead, as described above.

## Build hazard (unchanged from BUG-002's pass, and it bit again)

`target-shared/` is shared by every worktree, and cargo's uplifted `debug/micold-daemon` is a single
name the last builder wins. The first launch of this pass ran a client from this branch against
another branch's daemon; `strings … | grep included_worktrees` returned `daemon=0` while the client
had 4, which is how it was caught. A pass run against a mismatched pair proves nothing, and the
version banner prints *matching* version numbers — the schema hash is what differs, and it is not
shown.
