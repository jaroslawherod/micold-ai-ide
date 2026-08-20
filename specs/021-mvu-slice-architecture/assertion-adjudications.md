# Adjudicated assertion removals (feature 021, FR-027)

`scripts/check-assertions-frozen.sh` fails when an assertion disappears from the suite and does not
reappear. That is the right default and it is the whole safety argument for this restructuring: if
the suite is red, the refactor broke something, and that inference only holds while the assertions
are frozen.

It is not, however, the whole rule. FR-027 forbids expectations being **relaxed, rewritten or
deleted**; it permits them to be relocated, and — necessarily — it cannot forbid the *spelling* of an
assertion changing when the thing it names changes shape. A function that answered `bool` and now
answers `Option<T>` is asserted differently and asserts the same thing.

spec.md Q3 settled that this must not be automated away. A `norm()` waiver for `x.field` →
`x.field()` would have auto-passed ten faithful renames and two deliberate reversals alike, and said
nothing about either. The reader is the adjudicator, and the report is built for that — it prints
the closest surviving assertion beside each loss so the usual case is settled at a glance.

**What was missing was anywhere to write the verdict down.** Until T074 the script had two whole-run
settings and no third option, so a branch that had adjudicated its report honestly still exited 1,
and the only way to a green blocking job was to relax the check for everyone. This file is the
third option: one entry per removal, naming the task that removed it and why it is not a relaxation.
It is the same discipline `ALLOWED` in `tests/feature_write_isolation.rs` and `CORE_MEDIATED` beside
it already use, and it carries the same two rules — **nothing goes in without a reason, and nothing
stays in after it stops being true.** A `was:` line that is no longer missing from the suite fails
the check just as loudly as an unadjudicated removal, so this file cannot outlive what it permits.

The `was:` lines are the assertion as the check normalises it: whitespace removed and snake_case
module paths stripped, exactly as the report prints it.

## T067a-6 — `switch_active` answers `Option<Vec<Outcome>>`

Twenty-eight of the thirty-four, and the largest single group. `switch_active` used to answer `bool`;
it now answers `Option<Vec<Outcome>>`, and **`None` is still the refusal** — the outcomes are the
sidebar consequences of arriving in a project, which the root applies. So `assert!(st.switch_active(p))`
became `assert!(st.switch_active(p).is_some())`: the same proposition, about the same call, with the
same truth value.

This is not the `x.field` → `x.field()` case Q3 refuses. Nothing became a computed predicate over
state; one return value acquired a payload, and `is_some()` names the half that was there before.

**The weakening Q3 warns about did happen here, and was caught rather than argued away.** Applying
`.is_some()` uniformly silently stopped four files exercising behaviour that had moved *out of* the
function — `#[must_use]` does not catch it, because an `assert!` that consumes the value satisfies
the attribute. `switch_active.rs`, `background_restart.rs`, `app_state.rs` and `daemon_sync.rs`'s own
unit test each got a helper that drains the outcomes the way the root does, so those sites assert
*more* than they did before. The two `app_state.rs` entries below report "no near match survives"
for that reason: they are now `assert!(switch(&mut state, &p))` over the draining helper.

```
was: assert!(!st.switch_active(Pnew("/b")))
```
```
was: assert!(core.switch_active(Pnew("/a")))
```
```
was: assert!(st.switch_active(Pnew("/a")))
```
```
was: assert!(st.switch_active(Pnew("/b")))
```
```
was: assert!(st.switch_active(Pnew(p)))
```
```
was: assert!(state.switch_active(&first))
```
```
was: assert!(state.switch_active(&other))
```

## T064 — the add-worktree form's messages moved into the nested unit

`Message::AddWorktreeCancelled` became `Message::WorktreeForm(Msg::Cancelled)`, and
`Message::AddWorktreeBranchSelected` became `Message::WorktreeForm(Msg::BranchSelected)`. T064 is the
task that promotes the form to a nested unit with its own message vocabulary, so the message an
assertion names is the message the code now sends. The report pairs each at 92–99%.

Worth being precise about why this is admissible when Q3 refuses mechanism renames in general:
`norm()` strips snake_case module paths because those carry no truth value, and deliberately keeps
CamelCase so that swapping one enum variant for another stays visible. That is exactly right, and it
is why these four are reported. The adjudication is that the variant did not swap — it was *nested*,
and `WorktreeForm(Cancelled)` is the same message under the type that now owns it. A reader confirms
that against T064 rather than against the text, which is what an adjudication is for.

```
was: assert!(!published.iter().any(|m|matches!(m,MAddWorktreeCancelled)),"pressingrow{index}ofthebranchlist—anin-usebranch,drawnpastthedialog'sown\edge—cancelledtheadd-worktreeform.Theuserreachedforabranchandlosttheform\andeverythingtypedintoit,withnomessageandnowaytotellarefusalfroma\cancellationtheynevermade(FR-034,FR-035,SC-009;021FR-012asaysthispressmustdo\nothingatall).\n\nWhatitpublished:{published:?}",)
```
```
was: assert!(!published.iter().any(|m|matches!(m,MAddWorktreeCancelled)),"selectinganavailablebranchcancelledtheform:{published:?}",)
```
```
was: assert!(published.iter().any(|m|matches!(m,MAddWorktreeBranchSelected(_))),"pressingrow{index}ofthebranchlistdidnotselecttheavailablebranchunderit,so\thefixtureisnotpressingrowsatallandtherefusaltestprovesnothing.Published:\{published:?}",)
```
```
was: assert_eq!(on_escape(&state),Some(MAddWorktreeCancelled))
```

## T062 — `current_session_writers.rs` asks a stronger question

The only two of the thirty-four with no surviving counterpart, and the only two that say *more* than
what they replaced.

Both asserted the same thing: that `app.rs` writes `active_session` exactly once, inside the
`Message::SessionSelected` arm — the one exemption from the `set_current_session` funnel, because the
user clicked a row already in front of them and revealing it would scroll a list they were reading.

T062 moved every feature's arms out of the root, so that arm is now `features/session::selected`.
Re-pointing the count at `features/session.rs` would have made it 2 and said nothing about *which*
two. So the file now asserts that `app.rs` writes the field **never**, and separately that exactly
the two named functions in the session feature write it — `["set_current_session", "selected"]`. A
count of one became a naming of which two, and "at most one write here" became "no writes here".

The reasoning is in that file's own header, under "Where the exemption lives after T062".

```
was: assert!(arm.lines().take(12).any(is_a_write),"theonepermitteddirectwriteissupposedtobeinside`MSessionSelected`,and\itisnotthereanymore")
```
```
was: assert_eq!(writes.len(),1,"`app.rs`maywrite`active_session`exactlyonce—the`SessionSelected`arm.Found:\n\{}\n\nEverythingelsethereducerdoestothecurrentsessiongoesthrough\`set_current_session`;aseconddirectwritehereistheshapethiswholecheckexiststo\catch,becauseitlooksentirelyordinaryinreview.",writes.join("\n"))
```

## Merge of `main` — `resolve_foreground_after_catalog` answers `Option<Vec<Outcome>>`

Three, and unlike every group above these are not this feature's own removals: they belong to
`main`'s `fix(010): ask which session to show again once the catalog arrives (BUG-013)`, which
landed after this branch diverged and was merged in at the end.

The collision is the same one T067a-6 caused everywhere else. `resolve_foreground_after_catalog`
answered `bool` — did it move the pointer — and moved it by calling `set_current_session`, which
had become outcome-returning here in the meantime. Merged as written it compiled with a warning and
**dropped the reveal**: the right session would be resolved and its row left off-screen, which is
precisely the half-fix BUG-013 was filed about. So it now answers `Option<Vec<Outcome>>`, `Some`
meaning exactly what `true` meant, and the shell drains it — the shape `switch_active` already has,
for the same reason and with the same `None`-is-the-refusal reading.

The assertions are `.is_some()` / `.is_none()` over the same call in the same tests, and they assert
the same proposition. The merge commit records the rest of the integration.

was: assert!(st.resolve_foreground_after_catalog(),"theresolvemustbere-runnowthatthedataitneededexists")

was: assert!(!st.resolve_foreground_after_catalog(),"nothingwasmissing,sonothingisre-resolved—theuserisontheoverviewbecausethat\iswheretheruleputthem(FR-007)")

was: assert!(!st.resolve_foreground_after_catalog())
