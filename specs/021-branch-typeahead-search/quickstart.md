# Quickstart: Validating Branch Selector Type-Ahead Search

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) |
**Contracts**: [match-ranking.md](./contracts/match-ranking.md),
[typeahead-component.md](./contracts/typeahead-component.md)

Two halves. **§A** is the automated suite and is what CI runs. **§B** is the recorded manual pass that
Principle I's GUI-wiring exception requires for the render glue — the parts of `src/ui/` that no test
can reach. §B is not a substitute for §A: every rule with a decision in it is in §A.

---

## Prerequisites

```bash
mise trust          # once per fresh worktree
```

A git repository with many branches, for §B. To make one:

```bash
cd /tmp && rm -rf ta-demo && git init ta-demo && cd ta-demo
git commit --allow-empty -m init
for b in feat/login feat/logout feat/reporting feat/JIRA-412-checkout-flow_retry-v2 \
         chore/deps chore/dialog-cleanup docs/api fix/db-lock main-line release/2026-08; do
  git branch "$b"
done
```

---

## §A Automated

```bash
mise run test-core     # the matching module, fast
mise run test          # the whole workspace, matching CI
```

| Check | Lives in | Covers |
|---|---|---|
| Tiers, offsets, spans | `crates/micold-core/tests/typeahead_match.rs` | FR-003, FR-006, FR-006a, FR-008, FR-010 |
| Ranking and stability | `crates/micold-core/tests/typeahead_rank.rs` | FR-002, FR-004, FR-007 |
| Truncation around a match | `crates/micold-core/tests/typeahead_fit.rs` | FR-011d |
| 500-name budget | `crates/micold-core/tests/typeahead_budget.rs` | SC-002 (logic half) |
| Ranking quality, ≥95% in top five | `crates/micold-core/tests/typeahead_corpus.rs` | SC-003 |
| 8 characters is enough at 200 branches | `crates/micold-core/tests/typeahead_corpus.rs` | SC-001 |
| Keyboard rule — saturation, disabled rows, fall-through | `crates/micold-core/tests/typeahead_keys.rs` | FR-017, FR-017a |
| Form transitions | `crates/micold-client/tests/branch_search_state.rs` | FR-005, FR-014, FR-016, data-model §2 invariants |
| Picker regressions | existing `worktree_*` / `app_state` tests | FR-012, FR-013 |
| Blocked branch cannot be selected | `crates/micold-client/tests/branch_search_state.rs` | FR-012a |
| Picking an available branch is unchanged | `crates/micold-client/tests/branch_search_state.rs` | FR-013 |
| Focus opens the list, blur closes it | `crates/micold-client/tests/branch_search_state.rs` | FR-001b |
| The component names no branch/worktree/git | `crates/micold-client/tests/typeahead_is_generic.rs` | FR-019, FR-021a |
| Component API shape | `tests/material_builder_api.rs`, `tests/component_api_opacity.rs` | Principle VIII, contract §1 |
| Layer split | `tests/cdk_no_appearance.rs`, `tests/material_boundary.rs` | contract §3.5, §4.2 |
| Overlay closed list | `tests/one_overlay_implementation.rs` | contract §3.7 |
| No frames at rest | `tests/idle_requests_no_frames.rs` | contract §3.6 |
| Gallery completeness | `tests/showcase_completeness.rs`, `tests/showcase_captions.rs` | FR-020, contract §6 |

**Expected**: all green. A failure names the rule it broke; none of these are advisory.

### The frame half of SC-002

```bash
mise run test -- frame_probe
```

Confirms typing into the picker requests no frames beyond those the input itself causes — the probe
and reference scene already in the repository, not new instrumentation.

---

## §B Manual — the render glue

Run the app against the demo repository:

```bash
mise run run
```

Open it as a project, press **New worktree**, choose **Existing branch**.

### B1 — Narrowing and emphasis (US1)

1. Choosing **Existing branch** shows the field with its placeholder and **the list already open**,
   holding every branch in the picker's usual order — before anything is typed, and without a click
   (FR-001b). Click elsewhere in the dialog and the list closes; click back into the field and it
   returns, with the search text untouched.

   > Opening it *with the picker* rather than on a focus event is deliberate: the rendering stack's
   > text input reports neither focus nor blur, so a focus-triggered list was reachable by pointer
   > only — a developer arriving with Tab saw an empty field and no list until they typed.
2. Type `log`. Only `feat/login`, `feat/logout` and `chore/dialog-cleanup` remain.
3. **The `log` inside each name is emphasised** and the rest of the name is not.
4. `feat/login` and `feat/logout` sit above `chore/dialog-cleanup` — earlier match position first.
5. Watch the **Directory** and **Branch** preview rows below the field while typing: they do not move.
   Neither do the Create/Cancel buttons. ✅ FR-001a.

### B2 — Near misses (US2)

1. Clear the field and type `reportng`. `feat/reporting` is listed, with `reporting` emphasised as one
   run — it still reads as a word. ✅ FR-010.
2. Clear and type `frep`. `feat/reporting` is listed, with `f` and `rep` emphasised as separate
   characters.
3. Clear and type `fl` (two characters). Only literal `fl` matches appear — `feat/login` does **not**.
   Type a third character and approximate results appear. ✅ FR-006a.
4. Type `lagi` (four characters, one wrong). `feat/login` is listed with `logi` emphasised as one
   run — single-typo tolerance holds at short lengths, not only long ones. ✅ SC-004.
5. Type `zzq`. The list says no branches match; the text stays in the field, editable. ✅ FR-015.

### B3 — Long names (FR-011d)

Type `retry`. The row for `feat/JIRA-412-checkout-flow_retry-v2` truncates with a **leading** ellipsis
and `retry` is visible. Then type `JIRA` — the same row now truncates at the end instead, with `JIRA`
visible.

### B4 — Selection survives, and is visible (FR-014, FR-014a, FR-014b)

1. Type `log`, pick `feat/login`. The preview shows `Branch: feat/login`. **The field still holds
   `log`** — it never becomes the branch name.
2. Reopen the list and type `zzq` so nothing matches. The preview still shows `feat/login`.
3. Clear the field. The full list returns, `feat/login` is still the selection, and **its row is marked
   as the current selection** — distinctly from wherever the keyboard highlight happens to be. ✅

### B5 — Keyboard (FR-017, FR-017a)

With the list open: Down/Up moves the highlight without the caret leaving the field; it stops at the
ends rather than wrapping. Enter picks the highlighted row. Escape closes the list and picks nothing.
Ordinary characters keep reaching the field throughout.

**Tab** closes the list *and* moves focus on, which is the pair that matters: tab to **Create** and
press Enter, and it presses the button. An open list claims Enter, so one that outlived the focus
that opened it would pick a branch instead.

### B5a — Step count and reach (SC-001, SC-006)

In the demo repository, select `feat/JIRA-412-checkout-flow_retry-v2` **without scrolling and by
typing at most 8 characters** (`retry` is 5). Count the actions: focus the field, type, pick — no more
than picking from the old dropdown took. ✅ SC-001, SC-006.

### B6 — Unavailable branches (FR-012, FR-012a)

With one branch checked out in another worktree, search for it. It is listed and marked in use, with
the holder named in the row itself. Click it: **nothing happens** — it does not become the selection
and the list stays open. Highlight it with the keyboard and press Enter: likewise nothing. This is the
one place the picker's behaviour deliberately differs from feature 016, which allowed the selection
and refused at Create.

### B7 — Both schemes (FR-011, FR-012b)

Toggle the appearance setting. In each scheme: the emphasis is legible, the unemphasised remainder is
legible, and a disabled row is distinguishable from an enabled one by more than the absence of
emphasis. Put the keyboard highlight on the selected row and confirm the emphasis, the highlight and
the selection marker all stay individually legible on that one row.

### B8 — The gallery (US3)

```bash
cargo run -p micold-client --bin micold-showcase
```

Find the **Typeahead** entry — it has been on the page since User Story 1, because the completeness
gate does not allow a component without one. What User Story 3 adds is that it is **live**: type into
it and it narrows and emphasises over its sample data, with no repository involved. Toggle the scheme
and confirm both. Its caption names the states exercised live.

**The open/close rule (BUG-001, FR-020a).** The entry drives the component's own rule rather than
posing a state, so confirm all four:

1. On launch the entry's list is **closed** — a search field and nothing beneath it. This is the state
   the branch picker rests in; the entry used to be pinned open and never showed it.
2. Press the field. The list opens **before anything is typed**, floating over the page without moving
   the entries around it (FR-001b).
3. Press a row. The list closes and the marker stays on the row that was chosen.
4. Reopen it, then press somewhere else on the page — or press Escape. The list closes, and the search
   text and the choice both survive.

---

## Recording the pass

§B is evidence, so it is recorded the way features 006, 010 and 020 recorded theirs: note the date,
the platform, and any step that did not behave as written. A step that fails is a defect, not a note —
§B describes what the feature does, not what it usually does.

### §B8's open/close rule (BUG-001)

| Recorded | |
|---|---|
| Date | 2026-08-07 · Linux |
| The rule itself — closed at rest, opens on reach, opens on typing, closes on a pick, closes on a dismissal | **yes** — five tests in `crates/micold-client/tests/showcase_state.rs`, each observed failing before the reducer existed |
| The showcase binary launches and renders with the entry rewired | **yes** |
| §B8 steps 1–4 confirmed **on screen** | **not yet recorded** — the four steps are a visual check and need a human at the display |

The split is deliberate rather than a shortcut. The rule is a decision about state, so it lives in the
reducer where a test can drive it (Principle I); what `controls.rs` does with the answer is the thin
glue the GUI-wiring exception covers, and that half is what §B8's four steps confirm.
