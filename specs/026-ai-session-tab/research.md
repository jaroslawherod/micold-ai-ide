# Phase 0 research: The AI Session as a Tab

Nine questions the spec leaves to implementation. Each is recorded as Decision / Rationale /
Alternatives so the tasks can be written against a settled answer rather than re-deciding.

---

## R1 — What "stopped" means for the AI process

**Decision.** The AI tab wears the stopped mark for `SessionLifecycle::Idle`, `Failed` and
`InterruptedResumable`. It does **not** wear it for `Starting` or `Restarting { .. }`, and obviously
not for `Running`.

**Rationale.** FR-012d names `NotStarted` and `Exited` — which are `ShellLifecycle` variants, the
terminal tab's vocabulary. The AI process has a different and larger lifecycle
(`SessionLifecycle`: `Idle`, `Starting`, `Running`, `Restarting { attempts }`, `Failed`,
`InterruptedResumable`), so the requirement's *rule* has to be translated rather than its *names*
copied. The rule is FR-012d's own: the mark appears exactly where the menu can act. That set is
already written down once, in `attached_process_restartable` — `Idle | Failed |
InterruptedResumable` for `AiCli`, `NotStarted | Exited` for a shell — so the translation is a
lookup, not a judgement. `Restarting` and `Starting` are in-progress states, excluded by FR-012e
along with the shell's `Starting`.

**Alternatives considered.** Treating "not `Running`" as stopped: rejected by FR-012d, and it would
mark a session mid-`Restarting`, sending a user to a menu that has nothing to offer. Introducing a
third mark for the crash-loop `Failed` state: rejected as scope — `Failed` is stopped and
restartable, which is all the mark claims.

---

## R2 — One predicate behind both the mark and the menu

**Decision.** Generalise `ui/terminal.rs::attached_process_restartable` into a function that answers
for **any** process the strip can show — the AI process, or a named instance — and derive three
things from it: whether the tab wears the mark (FR-012d), whether that tab's menu has a restart item
(FR-006a), and therefore whether the menu opens at all (FR-006b). Keep the existing
`attached_process_restartable` as a thin call into it, so the bar's own restart control cannot drift
from the strip.

**Rationale.** FR-012d asks for an *agreement* between two things — the mark and the menu — and this
file has already learned once what that costs when they are two readings of one fact. The comment on
`empty_terminal_message` records it: the pane and the bar disagreed "for exactly as long as they
were two readings of one fact", and the fix was to derive both from this same predicate. Doing it
again here makes FR-012d true by construction, and makes FR-006b's "no menu when it would be empty"
a consequence rather than a second rule to keep in step.

**Alternatives considered.** A `lifecycle` match at each of the three call sites: rejected — it is
exactly the shape that produced BUG-004 in this file (`restart_message` reading a fact the
*predicate beside it* already had). A method on `Session`: attractive, but the split is
`TerminalMode`-shaped view logic and `micold-core` is render-free; the existing predicate's home is
the right one.

---

## R3 — What the stopped mark is built from

**Decision.** Extend `material::ActivityBadge` with a constructor that takes a `BadgeEmphasis`
directly (`ActivityBadge::for_emphasis`), leaving `ActivityBadge::new(signal, roles)` as sugar over
it, and draw the stopped mark with it. Whether the mark reuses `BadgeEmphasis::Ended` or adds a
`Stopped` variant is settled by the visual pass, not here; the constructor is the part the tasks
need.

**Rationale.** Principle VIII says extend the shared primitive rather than fork one, and this
primitive is already the right shape for three independent reasons: it is a small status dot, it is
drawn from the shared `Icon` vocabulary (a raw glyph literal rendered as tofu once — its BUG-004),
and its module doc already says it reserves "the width of the slot ... whether or not one is drawn",
which is exactly R4's requirement. What blocks direct reuse is only that `new` takes an
`ActivitySignal` — daemon activity, a different fact from process lifecycle — so the fix is to let
the emphasis in through the front door rather than to synthesise a fake signal.

**Alternatives considered.** A new `StoppedMark` component: rejected as a fork of a dot that already
exists, and it would need its own showcase entry, its own slot-reservation logic and its own tests.
Passing a contrived `ActivitySignal::Ended`: rejected — it would make the sidebar's activity
vocabulary carry a meaning it does not have, and `tests/showcase_completeness.rs` poses variants by
name, so the lie would be on the gallery page.

---

## R4 — The mark occupies a reserved slot, never a conditional child

**Decision.** Every tab always builds the same children: a leading slot (the mark, or an empty space
of the same size), the label, and the trailing slot (the close control on a terminal tab, an empty
space of the same size on the AI tab). Nothing is pushed-or-not.

**Rationale.** Feature 023 FR-008a, recorded at length in `ui/terminal.rs`: a conditional child
shifts every sibling after it, and iced's positional `Tree::diff_children` then hands the pressed
control its neighbour's node, dropping the `is_pressed` that `on_press` fires from — "the press
vanishes and the user has to press twice". A mark that appears when a process exits is exactly such
a child, and it appears *inside a button whose press is the feature*. Feature 012 already solves
this for the active indicator (an inactive tab draws a transparent rule of the same thickness) and
this morning's BUG-005 fix generalised it to the width axis; this is the same pattern one level in.

**Alternatives considered.** Pushing the mark only when stopped: rejected above. Reserving the slot
only on tabs that can ever be stopped: meaningless — every tab can.

---

## R5 — Horizontal scrolling

**Decision.** Extend `material::Scrollable` with a horizontal direction and use it for the terminal
tabs. It hard-codes `scrollable::Direction::Vertical` today; the direction becomes a builder step,
defaulting to vertical so both existing call sites are unchanged.

**Rationale.** Principle VIII again, and the wrapper is where the design system's 4px themed
scrollbar lives — a hand-rolled horizontal scroller would reintroduce the exact divergence this
component was created to end (its module doc: "two call sites, and they already disagree"). It is
also where dismiss-on-scroll is reported from (feature 021 FR-009), and a strip that scrolls is a
place the ground moves under an open tab menu, so routing through it gets that behaviour for free
rather than as a bug filed later.

**Alternatives considered.** `iced::widget::scrollable` directly at the call site: rejected by
Principle VIII and by the scrollbar divergence. Clipping without scrolling: rejected — FR-002a
requires the tabs to remain reachable.

---

## R6 — The edge fade, and what can gate it

**Decision.** The fade is drawn as an overlay on the scrolling viewport's leading and trailing
edges, present when content lies beyond that edge and given a distinct treatment when the tab beyond
it is the marked one (FR-002e). It is **appearance**, so it is verified by the `visual-pass` skill,
not by a layout gate — and the plan says so rather than implying a gate covers it.

**Rationale.** The layout-snapshot gates resolve rectangles. A gradient is drawn, not laid out: it
occupies the same box whether it is opaque or invisible, which is the family of defect the
`visual-pass` skill exists for and which its own doc lists first ("colour, tone and elevation").
Pretending otherwise is how feature 012 arrived at two visual passes finding what a green suite had
missed. What *can* be gated is the fact behind it — "content lies beyond this edge" is a pure
function of the viewport offset and the content width, and that function gets a unit test.

**Alternatives considered.** A style-snapshot assertion: it records values, not composited results,
so it would assert the fade exists without saying it is visible. Kept as regression cover for the
role used, not as proof.

---

## R7 — FR-002c is a defect that is live on `main`

**Decision.** Fix the bar's overflow behaviour **first**, before the AI tab is added, and register a
covered state with enough instances to overflow so the fix has a gate.

**Rationale.** The bar is a plain `row!` with no bound on the strip, so past about five instances
its trailing children — the "+" and the mode toggle — are laid out narrower or at zero, silently.
That is 012 BUG-005 one level out, it is reachable on `main` today, and this feature brings the bar
to it sooner by making the strip always visible and adding a tab. Fixing it first means the AI tab
lands on a bar that can hold it, and means the fix is verifiable on its own rather than tangled with
new behaviour.

**Alternatives considered.** Filing it as a separate bug against feature 012 and waiting: honest,
but it blocks this feature's own FR-002c, and the two fixes are the same edit.

---

## R8 — The tab menu's identity

**Decision.** Widen the existing menu state to name **which tab** it was opened on — the AI process
or a specific instance — rather than adding a second, parallel menu surface. One
`SurfaceId`, one registration, one entry in `tests/overlay_registration.rs`'s `POPOVERS`.

**Rationale.** FR-006a defines the AI tab's menu as "the terminal tab's menu minus Close", which is
a statement that they are one menu with one item filtered — and the spec says why: "so the two tabs
cannot drift into offering different actions". Two surfaces is the shape that lets them drift. The
state today is `shell_instance_menu: Option<(ShellInstanceId, u16, u16)>`; the change is the first
element, from an instance id to the tab identity of `data-model.md`.

**Alternatives considered.** A second `ai_tab_menu` surface: rejected — it doubles the registration,
the dismissal rules and the "close every menu" list that feature 012's BUG-005 already had to
remember to update, for a menu that is a subset of one that exists.

---

## R9 — The AI tab under the existing tab gates

**Decision.** Register the AI tab in the covered states so
`tests/gates/tab_children_fit.rs` runs against it, and expect **two** covered states to move:
`session-terminal-instance-tabs`, and `session-terminal-bottom-bar`, which has at most one instance
and therefore drew no strip at all until FR-003.

**Rationale.** Both of that gate's assertions apply to this tab and both are meaningful for it.
`every_control_inside_a_tab_holds_its_touch_target` is the one that catches the AI tab being
squeezed by the scrolling viewport's own bounds; `a_tabs_content_sits_on_its_tabs_midline` is what
holds FR-010a's "the icon sits on the tab's own midline" once the trailing slot is empty rather than
occupied — the leading and trailing slots being equal is precisely what makes that true, and it is
the property that failed at 4.6dp this morning. The second covered state is easy to forget and is
where FR-003's whole visible change lands.

**Alternatives considered.** Registering only the multi-instance state: rejected — it would leave
FR-003, the requirement that changes what a single-instance user sees, with no geometry coverage at
all, which is the gap feature 019 found for this very strip.
