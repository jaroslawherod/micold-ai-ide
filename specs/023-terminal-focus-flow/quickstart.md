# Quickstart: Natural Terminal Focus Flow

Two parts. **§A** is what the machine checks. **§B** is what has to be *watched at a display*,
because half of this feature's evidence is "the press I made did the thing on the first press" and
"no focus ring blinked" — neither of which any test in this repository can see.

§B is run with the repo's `visual-pass` skill (Xvfb + `xdotool` + `import`), not by asking a person.

---

## §A — The automated suite

```bash
mise run test        # whole workspace, matching CI
mise run test-core   # render-free logic only, when iterating on the predicate
```

Green is the gate. The gates that matter to this feature, and what each is watching:

| Gate | Watching |
|---|---|
| `terminal_focus.rs` | the predicate's truth table over all four terms; only the displayed session's terminal is ever eligible (FR-020); each navigation clearing a release; a release surviving a dialog and a window switch; launch focusing a restored session |
| `terminal_focus.rs` (kept from 006) | `route_key`'s truth table — unchanged by this feature, and the regression check that says so |
| `terminal_pane.rs`'s inline `mod tests` | `press_routing`; `press_grants_focus`, the press that grants focus (FR-008b) — the case that made a TUI need two presses; and **no press outside the pane's bounds produces a `TerminalAction(Write(..))`** (FR-003, SC-008). Inline because both functions are `pub(crate)` |
| `terminal_bar_stability.rs` | **the bottom bar does not branch on terminal focus**, and `terminal_released` is written only by the two helpers, anywhere in `crates/micold-client/src/`. The gate most likely to catch a real regression: a focus-conditional child shifts its siblings and iced silently swallows the press on them (research R1). Also **that the bar carries no release-focus control** (FR-021b, BUG-001) — the two together pass only for an unconditional removal |
| `keymap.rs` (pre-existing, from 006) | the reserved release chord still encodes per platform. This feature adds nothing here; it is listed because a regression would look like FR-021/FR-021b breaking — and with the affordance gone, the chord is the only explicit release left, so this gate now carries the whole "never trapped" guarantee |

### The three assertions to read before believing the suite

1. `terminal_focused` is **not a field any more.** A test that still writes
   `State { terminal_focused: true, .. }` will not compile — that is the migration working, not a
   failure. Set `terminal_released` or drive the message.
2. `no_scattered_release_writes` — `terminal_released` is assigned only inside `focus_terminal()`
   and `release_terminal()`. Seven scattered assignments are what this feature is undoing; the gate
   is what stops the eighth.
3. `window_focus_changes_no_focus_term` — `WindowFocusChanged` touches none of the predicate's
   terms. FR-013–FR-015 are satisfied by *not* writing anything, and a future "helpful" restore
   would break them.

---

## §B — The visual pass

```bash
mise run run    # the application; it spawns/attaches the daemon itself
```

Start a session, let its terminal come up, and click into it so it holds the keyboard.

### B1 — One press, one outcome (US1, SC-001, SC-002)

With the terminal focused, press **once** and confirm each acts on that press:

1. the mode toggle in the pane's status bar (AI CLI ⇄ Regular) — **the reported bug; this is the
   case that used to need two**
2. a Regular Terminal instance tab, and the "+" that opens another
3. a different session in the sidebar
4. a toolbar action, and an item in an open menu
5. the sidebar filter field

For 1–4, type immediately afterwards: the characters must reach the terminal, with no click in
between. For 5, the characters must go into the field.

Then press once on empty space in the sidebar and on a disabled control: nothing happens, and typing
still reaches the terminal.

**Rapid alternation.** Press a control, press back into the terminal, press the control again, as
fast as you can. The keyboard must settle on exactly what the last press asked for — no lingering
focus and nothing restored from an earlier press (spec Edge Cases).

SC-008 — that none of these presses reaches the attached process — is evidenced by §A's inline
assertion in `terminal_pane.rs`, not by watching pixels here.

### B2 — No blink (FR-008a, SC-007)

Screenshot the focus indicator during B1's presses — specifically the frame right after the press
lands on the mode toggle. The ring must be continuously present. A pass that shows it absent for one
frame is a fail even though the click worked: that is the release-and-reacquire shape the contract
forbids.

Confirm no control in the bar appears or disappears as focus changes — that appearing/disappearing is
precisely what swallowed the press. (This paragraph used to say the release affordance is always in
the bar, greyed when the terminal does not hold the keyboard. The affordance is gone entirely —
FR-021b, BUG-001 — which satisfies the same rule: a child that never exists cannot shift its
siblings. The rule itself is unchanged and still watched by `terminal_bar_stability.rs`.)

### B3 — Away and back (US2, SC-003)

1. Focused terminal → switch to another window → switch back → type. It reaches the process, no
   click.
2. Press the release chord (`Ctrl+Shift+E`) → switch away → back → type. It does **not** reach the
   process; app shortcuts work.
3. Open Settings, type into a field, switch away → back. The field still has the caret and the
   characters go into it.
4. **The process is undisturbed (FR-025).** Run `yes | nl` in the pane so it prints numbered lines
   continuously, then release and re-acquire focus half a dozen times — chord, press back into the
   pane. The line numbers must be unbroken, with no gap, no restart from 1, and no reflow. Focus is
   not something the attached process is allowed to notice. (The release affordance was a third
   route here; it is gone — FR-021b, BUG-001. Two routes still alternate the state, which is what
   this step actually exercises.)

### B4 — Landing ready to type (US3, SC-004)

From an unfocused terminal, do each and type straight away:

- select a session; start a new one
- toggle the mode; open, close and switch a Regular Terminal instance
- switch to another project with a restored session
- quit and relaunch the application with a session displayed

Every one must accept the keystroke with zero presses.

### B5 — Never taken mid-word (US4, SC-005)

Open the Add Worktree form and type into its name field while a background session is producing
output (run `yes | head -100000` in a second session). Every character lands in the field; none
reaches a terminal. Dismiss the form: the keyboard returns to the terminal and typing resumes —
unless you released it first (B3.2), in which case it stays with the application.

### B6 — Pressing into the terminal (FR-008b, SC-009)

With the terminal **not** holding the keyboard, run a mouse-aware program in it (`vim`, or any TUI
with a mouse-driven list). Press once inside the pane, on a target the program should react to. The
terminal takes the keyboard **and** the program receives that press. Needing a second press is the
reported bug's mirror image and is a fail.

Then the harder case: put the caret in the sidebar filter field, type a character to be sure it holds
the keyboard, and press **once** into the pane. The terminal takes the keyboard on that press and the
next character reaches the process — `focus_terminal()` clears `focused_field`, so this does not
depend on the field's blur arriving first.

---

## What a recorded pass must contain

Per the `visual-pass` convention, the recorded run states for each of B1–B6: the command, what was
driven, the screenshot(s), and the observation. B2 and B6 are the two that cannot be inferred from
the others and must show pixels.
