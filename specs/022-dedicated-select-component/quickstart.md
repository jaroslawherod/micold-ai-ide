# Quickstart: Dedicated Select Component on a Shared Picker Base

Two parts. **§A** is what the machine checks. **§B** is what a person has to look at, because this
feature's whole subject is how two controls *look* beside each other and how a list *arrives* — and
no test in this repository can see either.

---

## §A — The automated suite

```bash
mise run test        # whole workspace, matching CI
mise run test-core   # render-free logic only, when iterating on the keyboard rule
```

Green is the gate. The gates that matter most to this feature, and what each is watching:

| Gate | Watching |
|---|---|
| `cdk_no_appearance.rs` | the base names no colour, type, shape or duration — `exit` is a bare number, as `gap` already is |
| `one_overlay_implementation.rs` | one hand-written cdk overlay, and the `select.rs`/`pick_list` sanction **gone**. Its staleness check fails the build while a sanction that no longer applies is listed, so this cannot be forgotten |
| `material_boundary.rs` | `pick_list` has left `WRAPPED_WIDGETS` |
| `material_builder_api.rs` | `Select` still constructs with required inputs and terminates in `.into()` |
| `component_api_opacity.rs` | no progress value in any public signature |
| `idle_requests_no_frames.rs` | **a settled picker asks for no frames.** The one most likely to catch a real defect here — an exit track that never finishes looks fine and burns the CPU forever |
| `logical_state_ownership.rs` | openness and highlight are presentation, by that file's own "screen switched off" test |
| `typeahead_is_generic.rs` | the component still names no branch, worktree or git — following the rename |
| `showcase_completeness.rs` / `showcase_captions.rs` | both pickers catalogued; the select `interactive` with a non-empty `live` list |
| `anatomy_size.rs` / `content_placement.rs` | the trigger lays out at its stated height and puts its content where §7.7 says |
| `tokens.rs` | the select's pairings clear AA in both schemes |
| `app_state.rs` | `selecting_a_type_sets_the_form_value` and `type_selection_is_ignored_while_creating` pass **unmodified** — the regression check on FR-030 |

---

## §B — The manual pass

```bash
cargo run -p micold-client --bin micold-showcase   # the gallery
mise run run                                        # the application
```

### B1 — The two lists, side by side (SC-001)

In the gallery's **Components** page, open the `Select` entry's list and the `Typeahead` entry's list
and compare them. Eight properties, zero differences:

1. surface tone 2. elevation 3. corner radius 4. list padding 5. row height 6. row padding
7. row hover / pressed / selected treatment 8. selection marker, and the space reserved for it on
unmarked rows

Then toggle the scheme and do it again. A difference here is the feature failing at the one thing it
exists to do.

### B2 — The transition (SC-002, SC-003)

Open and close each list several times.

- It **grows** from slightly compressed to full while fading in, and settles rather than snapping.
- It **fades out** on the way, in noticeably less time than it took to arrive.
- Reverse one mid-flight: it continues from where it is, it does not jump to either end.
- Press where a row used to be while the list is fading out: **nothing is chosen.**
- Watch the rest of the page throughout. **Nothing outside the list moves, at any point.**

### B3 — The select's own feedback (SC-005)

Open the select. Its active indicator thickens and takes the accent, and stays that way until the list
closes — **with nothing on the page supplying that fact.** This is the accepted fidelity gap being
closed rather than reworded; if it does not happen, the gap is still there.

### B4 — Keyboard only (SC-004)

Put the pointer away. For **each** picker: reach it, open it, move down and up through the rows, take
one with Enter, reopen, dismiss with Escape, reopen, and Tab out of it. All five keys mean the same
thing in both controls. Taking a row with Enter must **not** also submit the dialog behind it.

### B5 — Placement (SC-006)

Four placements, none clipped, none off screen, none widening its container:

1. inside the add-worktree dialog — a content-sized box, the placement that defeated the first
   hand-rolled attempt at this in feature 013
2. at the bottom edge of the window — **flips above the trigger**
3. at the right edge
4. on a full-height page (the gallery)

### B6 — The application still works (SC-009)

Open the add-worktree dialog, pick a type, create a worktree. Identical to before — same options, same
result. Then do it again choosing nothing, and confirm the form validates exactly as it did.

### B7 — Both schemes

Every state in B1–B5, in light and in dark.

---

## Recording the pass

§B is evidence, so it is recorded the way features 006, 010, 020 and 021 recorded theirs: the date,
the platform, and any step that did not behave as written. A step that fails is a defect, not a note.

**On honesty about what was checked.** Feature 021 §B8 carries a table saying which half of a pass was
automated and which needed eyes at a display, and says plainly when the second half was not done. The
same applies here and more so — §B1 and §B2 are the feature's two headline claims and *neither* can be
automated. A green suite is not this feature working.

| Recorded | |
|---|---|
| Date | |
| Platform | |
| B1 — eight properties, both schemes | |
| B2 — grow-and-fade, interruptible, nothing else moves | |
| B3 — the indicator answers for itself | |
| B4 — five keys, both pickers | |
| B5 — four placements | |
| B6 — the form is unchanged | |

### Screenshots

`mise run screenshot` (added during feature 018's BUG-002) pulls a frame off the monitor's PipeWire
node, which is the only route to a screenshot on a stock GNOME/Wayland session. B1 is worth a frame
holding both lists open at once — the comparison this feature is judged on, in a form a reviewer can
check later.
