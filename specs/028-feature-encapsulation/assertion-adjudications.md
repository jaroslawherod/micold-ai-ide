# Adjudicated assertion removals (feature 028, FR-021)

FR-021 restates for this feature the freeze feature 021 put in place: an expectation may be
relocated, but not relaxed, rewritten or deleted. `scripts/check-assertions-frozen.sh` enforces it,
and as of T004 it recognises this feature the same way it recognises 021 — by the branch naming it,
or by a commit touching `specs/028-feature-encapsulation/`.

That scope extension is the reason this file exists at all. The check reads its adjudications from
whichever feature is in scope, so a 028 branch consults *this* file and never 021's; entries filed
in the other feature's directory would be silently invisible, and every removal here would read as
unadjudicated.

**This feature renames assertion spellings by the thousand without changing a single expectation.**
Moving `session` state off the root and behind `features::session::State` changes what a test says
about the thing it is asserting — `state.session_id` becomes `state.session.id` — while asserting
exactly the same proposition. The check normalises whitespace and strips snake_case module paths,
which absorbs some of that, but not a field moving under a struct. Each such group gets one entry
below, naming the task that moved it.

Two rules carry over from 021's file unchanged, and they are what make it safe:

- **Nothing goes in without a reason.** An entry names the task that removed the assertion and says
  why the removal is not a relaxation. A `was:` line under no `## ` heading is refused outright.
- **Nothing stays in after it stops being true.** Every entry is re-verified against the tree on
  each run: a `was:` line naming an assertion that is back in the suite fails the check as loudly as
  an unadjudicated removal. This file cannot outlive what it permits.

The `was:` lines are the assertion as the check normalises it — whitespace removed, snake_case
module paths stripped — exactly as the report prints it.

## The check was verified to block before anything was renamed (T005)

Not an adjudication; the record of the non-vacuity probe FR-017 asks for. A single assertion in the
suite was altered, the check run, and the failure observed and reverted. Recorded here because the
freeze is the safety argument for the whole restructuring: a check that reports instead of blocking
would let every subsequent phase pass by default, and that is precisely the failure this file's
scope extension was written to close.

The probe: `crates/micold-client/tests/feature_write_isolation.rs` carries a two-line `assert!` whose
message was rewritten in place — `"OWNERS names paths that are not \`State\` fields any more"` became
`"OWNERS is stale"` — with the predicate and the `assert!(` opener untouched. That is deliberately
the hardest case for this check and the one issue #146 was filed about: a line-based diff sees the
opener as context and the changed line as carrying no `assert` token, so the rewrite would enter
neither the removed set nor the added one and pass as "no assertion removed". The whole-file
multiset comparison catches it.

Observed, at exit 1:

```
assertion freeze: FAILED — 1 assertion(s) removed and not reinstated

  - assert!(stale.is_empty(),"OWNERSnamespathsthatarenot`State`fieldsanymore:{stale:?}")
      was in: crates/micold-client/tests/feature_write_isolation.rs
      no near match survives — this looks like an outright deletion

feature 028's FR-021 freezes the existing suite for that feature's duration.
This change is in scope: it touches specs/028-feature-encapsulation/.
Tests may be ADDED or RELOCATED; expectations may not be relaxed, rewritten or deleted.
```

Three things this pins, beyond the non-zero exit. The check names the assertion and the file it left.
It resolves scope through the **directory** signal, not the branch name — this branch is
`feat/feature-encapsulation`, which contains no "028", so had T004 extended only the branch case the
freeze would still have been reporting. And it points the reader at
`specs/028-feature-encapsulation/assertion-adjudications.md`, this file, rather than 021's.

The first run of the probe blocked correctly and then cited feature 021's FR-027 as the rule, one
line above naming 028 as the scope. That is fixed, and the suite now has a case holding it: reading
the report is how every adjudication below gets written, and a report that sends the reader to the
wrong requirement corrupts the input to that judgment. The probe was reverted; nothing in the suite
carries the altered text.

## T006 — `help`'s three variants moved behind `Message::Help`

Six assertions, one shape. Each asks what the Escape path answers when a help surface is open, and
each compares it against the message that closes it. That message is now spelled
`Message::Help(HelpMsg::AboutClosed)` rather than `Message::AboutClosed` — the same message, reached
through the wrapper the conversion introduced, with the variant keeping its name and losing only the
feature-name prefix it never needed (contract M1).

Nothing here says less than it did. The call under test is unchanged, the expected value is the same
value, and the equality is still an equality: `Some(x)` where `x` closes the About dialog. What
changed is the path by which the vocabulary names it, which is the whole content of this task.

The check reports five of the six at 89–97% against their own successors, and the sixth
(`escape(&both)`) at 89% for the same reason — a longer spelling of the same constructor. Read
individually rather than pattern-matched, as Q3 requires: each survivor was compared against the
assertion it replaced, and in every case the predicate, the subject and the expected value are
identical modulo the wrapper.

```
was: assert_eq!(escape(&both),Some(MAboutClosed))
```
```
was: assert_eq!(escape(&state),Some(MAboutClosed),"adialogoutranksamenu,whicheverwasopenedfirst(contractD1)")
```
```
was: assert_eq!(escape(&state),Some(MHelpMenuToggled),"Escapeclosestheoverflowmenu,whichbeforeT031itleftopen")
```
```
was: assert_eq!(on_escape(&state),Some(MAboutClosed))
```
```
was: assert_eq!(on_escape(&state),Some(MAboutClosed),"thescrimemitswhateverEscapewould,sothetwopathscannotdisagree")
```
```
was: assert_eq!(open.on(TEscape),Some(&MAboutClosed))
```

## T009 — `settings`'s ten variants moved behind `Message::Settings`

Two of the three are the same rename the previous entries describe: `Message::SettingsCancelled`
became `Message::Settings(settings::Msg::Cancelled)`, so an assertion naming the constructor names
it by its new path. `on_escape(&s)` still returns what Escape does to an open settings form, and
`rules.on(Trigger::Escape)` still pairs that surface with that message; the subject, the predicate
and the expected value are each unchanged. Read individually as Q3 requires, neither says less than
it did — the value they name is the same value, spelled through the wrapper that now owns it.

```
was: assert_eq!(on_escape(&s),Some(MSettingsCancelled))
```
```
was: assert_eq!(rules.on(TEscape),Some(&MSettingsCancelled),"dispatchasksonequestion—whathappensonthistrigger—andgetsthepairing,soit\cannotmismatchasurface'srulewithanother'smessage")
```

### The third is a floor that this feature made false (FR-020)

`root_is_routing_only`'s vacuity probe asserted `total >= 85`. That number was not a property of the
rule; it was 021's arithmetic, recorded when T064 folded 22 variants into one and the root still had
89 arms. Feature 028 folds all ten features' vocabularies, and the root ends at 15 arms — 10 wrapper
variants and 5 cross-cutting ones. A floor of 85 fails at the fourth conversion and would have to be
re-derived nine more times to keep saying the one thing it exists to say: *the scan is still
parsing*.

**So the floor was lowered to 12 and its work handed to something that does not decay.** The probe
now also asserts the scan finds the root's `ScrolledBeneathOverlay` and `EscapePressed` arms by
name. Those two are cross-cutting — belonging to no feature is exactly why 028 leaves them at the
root — and a scan that has stopped parsing produces neither name at any count. That is a stronger
vacuity argument than the floor was, not a weaker one: 85 could be met by a scan reading the wrong
`match` in the same file, and the named check cannot.

The lowered floor is still doing its own share. It sits under the 15 the feature arrives at and far
above the 0 a broken parse reports, so a scan that goes quiet mid-migration fails on the count
before the names are even looked at.

```
was: assert!(total>=85,"thescanfoundonly{total}armsintherootreducer—itfound89afterT064,anda\scanthathasgonequietreportstherootasroutingonly")
```

## T010 — `sidebar`'s ten variants moved behind `Message::Sidebar`

All five are one constructor renamed: `Message::SidebarFilterMenuToggled` became
`Message::Sidebar(sidebar::Msg::FilterMenuToggled)`, and the filter panel's dismissal rule is
asserted in five places. Read individually as Q3 requires — the subject (`escape`, `on_escape`,
`open.on`, `scroll_beneath`), the predicate and the expected value are unchanged in every one, and
each still says the same thing about the same surface: Escape closes the open filter panel, and a
scroll behind a modal invalidates the menu anchored beneath it. The message they name is the same
message, spelled through the wrapper that now owns it.

```
was: assert_eq!(escape(&popover_alone),Some(MSidebarFilterMenuToggled),"withnomodalthepopoveristhetopmostsurface,andEscapeisitsown")
```
```
was: assert_eq!(on_escape(&state),Some(MSidebarFilterMenuToggled),"Escapemustdismissthefilterpanelwhileit'sopen")
```
```
was: assert_eq!(on_escape(&state),Some(MSidebarFilterMenuToggled),"theeverydaycase:onelightweightsurfaceopen,andEscapeclosesit")
```
```
was: assert_eq!(open.on(TEscape),Some(&MSidebarFilterMenuToggled))
```
```
was: assert_eq!(scroll_beneath(&state(Some(&dialogs()[0]),true)),vec![MSidebarFilterMenuToggled],"ascrollbehindanopenmodalstillinvalidatesthemenuanchoredbeneathit,anddoes\nottouchthemodal")
```

## T012 — `worktree`'s eighteen variants moved behind `Message::Worktree`

Both are one constructor renamed. `Message::WorktreeDeleteCancelled` became
`Message::Worktree(worktree::Msg::DeleteCancelled)`, and `Message::WorktreeRenameCancelled`
became `Message::Worktree(worktree::Msg::RenameCancelled)`. Read individually as Q3 requires:
the subject is `on_escape(&state)` in both, the predicate is `assert_eq!` in both, and the state
each is handed — a delete confirmation opened by `DeleteRequested`, a rename opened by
`RenameStarted` — is unchanged. Each still says exactly what it said: Escape cancels the
worktree operation that is currently asking, and cancels *that* one rather than some other
dismissable surface, which is what makes the named message worth asserting instead of a bare
`is_some()`. The message they name is the same message, spelled through the wrapper that now
owns it.

```
was: assert_eq!(on_escape(&state),Some(MWorktreeDeleteCancelled))
```
```
was: assert_eq!(on_escape(&state),Some(MWorktreeRenameCancelled))
```

## T013 — `project`'s nineteen variants moved behind `Message::Project`

One constructor renamed: `Message::ProjectForgetCancelled` became
`Message::Project(project::Msg::ForgetCancelled)`. Read as Q3 requires — the subject is
`on_escape(&state)`, the predicate is `assert_eq!`, and the state is a forget confirmation opened
by `ForgetRequested`, all unchanged. It still says what it said: Escape cancels the forget
confirmation, and cancels *that* rather than any other open surface, which is why the test names
the message instead of asserting the dialog merely closed. The message it names is the same
message, spelled through the wrapper that now owns it.

```
was: assert_eq!(on_escape(&state),Some(MProjectForgetCancelled))
```

## T014 — `session`'s thirty-seven variants moved behind `Message::Session`

Seven, and six of them are the plain case: one constructor renamed, nothing else touched.
`restart_message` is still asked the same question with the same state and still has to answer
with the same message — that the bar's restart control names the attached *shell instance* in
Regular mode and the session's own process otherwise, which is `012` BUG-004 — and the pane's
press handler still has to publish exactly `TerminalFocused` and exactly no focus message for a
press outside it. Read individually as Q3 requires: subject, predicate and expected value are
unchanged in all six; only the path to the constructor is longer.

The seventh is the one worth stopping on. `the_bars_restart_control_asks_which_process_it_is_
restarting` reads `ui/terminal.rs` as *source text* and greps it for the defect's spelling, so
the constructor's name is not incidental to that assertion the way it is to the other six — it
is the thing being matched. It was updated to the new spelling rather than left to pass
vacuously against a string the file can no longer contain. The negation still forbids exactly
what it forbade: a session-level restart written straight into the bar. FR-017's non-vacuity
concern is the live one here, and the guard is checked by the positive assertion immediately
above it, which requires `on_press(restart_message(` and would fail if the control stopped
asking at all.

```
was: assert!(!code.contains("on_press(MTerminalRestartRequested)"),"abaresession-levelrestartinthebarisBUG-004:inRegularmodethesession'sprimary\isstillrunning,sotherequestisano-opandthecontroldoesnothing")
```
```
was: assert!(!published.iter().any(|m|matches!(m,MTerminalFocusReleased|MTerminalFocused)),"apressoutsidethepanemustnotchangethekeyboardholder\(FR-005,FR-006,FR-008a):{published:?}")
```
```
was: assert!(published.iter().any(|m|matches!(m,MTerminalFocused)),"apressinsideanunfocusedpanetakesthekeyboard(FR-008b):{published:?}")
```
```
was: assert_eq!(restart_message(&state,SessionInew()),MTerminalRestartRequested)
```
```
was: assert_eq!(restart_message(&state,id),MShellInstanceRestartRequested(id,instance),"inRegularmodethebardescribestheattachedshellinstance,soitsrestartmust\namethatinstance—restartingthesessionleavesthedeadshelldead")
```
```
was: assert_eq!(restart_message(&state,id),MTerminalRestartRequested)
```
```
was: assert_eq!(restart_message(&state,id),MTerminalRestartRequested)
```

## T022 — G1's non-vacuity probe (FR-017)

Not an adjudication; a record. `Message::ProbeAboutOpened` was added to `app::Message` with one
arm, `crate::features::help::about_opened(self)`, and
`cargo test -p micold-client --test root_vocabulary_is_cross_cutting` was run. The guard failed,
naming the feature:

```
thread 'no_root_variant_belongs_to_one_feature' panicked at
crates/micold-client/tests/root_vocabulary_is_cross_cutting.rs:513:5:
the root vocabulary holds 1 variant(s) produced and consumed by exactly one feature (FR-013):
  `Message::ProbeAboutOpened` — `help`
```

Two of the guard's other tests failed alongside it, and both are the guard working rather than
noise: nothing emits a variant that was just invented, so `variants_with_no_producer_are_reported_
not_failed` reported it as unrecorded, and the vocabulary count moved from 15 to 16. Three of six
tests passed, so the run was a real run — an injection that fails to compile demonstrates nothing.

Reverted with `git checkout -- crates/micold-client`; the six tests pass again.

## T024 — G3's non-vacuity probe (FR-017)

`src/features/probe.rs` was added declaring `pub enum Msg { Tick }` and no `update`, then removed.
`cargo test -p micold-client --test feature_registration_cost` failed:

```
thread 'every_feature_with_a_vocabulary_has_a_reducer_entry_point' panicked at
crates/micold-client/tests/feature_registration_cost.rs:471:5:
a feature declares a vocabulary with nowhere to handle it (FR-015):
  `probe` declares `pub enum Msg` and no `update`

Add `pub fn update(&mut State, Msg) -> Vec<Outcome>` to the feature module (shape A), or
`pub fn update(&mut App, Msg) -> Task<Message>` to `src/shell/<feature>.rs` when the transitions
must return an `iced::Task` (shape B). A vocabulary answered by the root instead is the coupling
FR-001 exists to remove.
```

Two of the file's existing guards fired alongside it, which is worth recording rather than tidying
away: the probe was not named in `features/mod.rs`, so `every_feature_module_is_registered_exactly_once`
reported the disagreement between that file and the directory, and `every_feature_module_has_an_isolation_test`
reported the missing `tests/features_probe.rs`. A half-added feature trips every guard it is half of.
G3 is the one that names the missing reducer, and it is the only one of the three that would still
have fired had the probe been fully registered.

No assertion text changed; the working tree was restored with `rm`.

## T028 — the notification queue moved behind `state.notifications`

One rename, applied everywhere: `state.notify` is `state.notifications.queue`, because the queue is
now a member of `features::notifications::State` rather than a flat field of `app::State`. Thirty-one
assertions changed spelling and none changed meaning — every one still asks the queue the same
question about the same queue, reached one segment further in.

Twenty-three of them are plain field-path renames across `tests/notifications.rs`,
`tests/worktree_delete.rs`, `tests/open_project_git_gate.rs`, `tests/background_restart.rs`,
`tests/banner_is_not_a_snackbar.rs`, `src/shell/service_control.rs`, `src/shell/daemon_sync.rs` and
`src/shell/persist.rs`.

The twenty-fourth is different and worth naming: `tests/idle_subscriptions.rs` greps the
subscription guard as **source text**, so the spelling *is* the assertion — the same shape as T014's
restart-control case. Its non-vacuity is unchanged: the surrounding test still fails if the guard
stops naming the queue at all, and the new expected text is the text the source now has.

```
was: assert!(!st.notify.is_active())
was: assert!(app.core.notify.is_active())
was: assert!(app.core.notify.is_active(),"restartingtheservicemusttelltheuserwhatitcosts")
was: assert!(app.core.notify.visible().is_none(),"anordinaryprojectswitchmustnotreporttheconnection")
was: assert!(app.core.notify.visible().is_some(),"adroppedopisstillreported")
was: assert!(core.notify.visible().is_none())
was: assert!(guard.contains("notify.is_active()"),...)
was: assert!(st.notify.is_active())
was: assert!(st.notify.visible().is_none())
was: assert!(st.notify.visible().is_none(),"aninfonoticeoutliveditsownduration")
was: assert!(st.notify.visible().is_some(),"theerrorwasclearedaftertheinfoduration—itisbeingtimedbythewrongseverity")
was: assert!(state.notify.visible().is_none())
was: assert!(state.notify.visible().is_none(),"afullysuccessfuldeletemustreportnothing,got:{:?}",state.notify.visible())
was: assert!(state.notify.visible().is_some())
was: assert!(state.notify.visible().is_some(),"nonotificationreachedthequeueatall,soeveryassertionaboveisvacuous")
was: assert_eq!(error.notify.visible().map(|n|n.level),Some(LError))
was: assert_eq!(info.notify.visible().map(|n|n.level),Some(LInfo))
was: assert_eq!(st.notify.pending(),0)
was: assert_eq!(st.notify.pending(),2)
was: assert_eq!(st.notify.visible().map(|n|n.message.as_str()),Some("first"))
was: assert_eq!(st.notify.visible().map(|n|n.message.as_str()),Some("second"),"dismissingleftagapinsteadofpromotingwhatwaswaiting")
was: assert_eq!(state.notify.pending()+from(state.notify.visible().is_some()),1,"agenuineleftovermuststillreachtheuser")
was: assert_eq!(state.notify.pending(),0)
was: assert_eq!(state.notify.pending(),0,"onemessageshouldnotqueuebehinditself")
was: assert!(guard.contains("notify.is_active()"),"thesnackbarclockisinside`{guard}`,whichdoesnottestwhetheranotificationis\showing.Subscribedatrestitwakestheprocessfourtimesasecondforthelifeofthe\application,andnobehaviouraltestinthisworkspacecanseeit(FR-032a,SC-017).")
```

## T029 — About and the Help menu moved behind `state.help`

Two renames, applied everywhere: `state.about_open` is `state.help.about_open` and
`state.help_menu_open` is `state.help.help_menu_open`, because both fields are now members of
`features::help::State` rather than flat fields of `app::State`. Twenty-two assertions changed
spelling and none changed meaning — each still asks the same flag the same question, reached one
segment further in. Every one is a plain field-path rename, across `tests/about_open.rs`,
`tests/features_help.rs`, `tests/overlay_dismissal_delta.rs`, `tests/overlay_dispatch_ordering.rs`,
`tests/overlay_transition_identity.rs` and `tests/project_switcher.rs`.

Nothing here is a source-text assertion of the T028 `idle_subscriptions.rs` kind: no test in this
set greps for the old spelling, so no expected string had to be re-pinned.

```
was: assert!(!menu.help_menu_open,"Escapenowreachestheoverflowmenu,whichthesubscription'smatchnevernamed")
was: assert!(!st.about_open)
was: assert!(!st.help_menu_open)
was: assert!(!st.help_menu_open,"openingtheswitcherclosestheoverflowmenu")
was: assert!(!st.help_menu_open,"themenutheactionwaschosenfromdoesnotstayopenbehindthedialogitopened")
was: assert!(!state.help_menu_open)
was: assert!(!state.help_menu_open,"overflowmenu")
was: assert!(!state.help_menu_open,"theoverflowmenumustclosewhencontentscrollsbeneathit")
was: assert!(!state.help_menu_open,"theoverflowmenusurvived{name}opening")
was: assert!(closing.state().about_open,"thesnapshotmustkeepthestateasitwas,nottracktheliveone")
was: assert!(st.about_open)
was: assert!(st.about_open,"stillexactlyone,notasecondinstance")
was: assert!(st.help_menu_open)
was: assert!(state.help_menu_open)
was: assert!(state.help_menu_open,"precondition:themenuisopen")
```

## T030 — the window's size and focus moved behind `state.window`

Two renames, applied everywhere: `state.focused_field` is `state.window.focused_field` and
`state.window_size` is `state.window.window_size`, because both fields are now members of
`features::window::State` rather than flat fields of `app::State`. Twenty assertions changed
spelling and none changed meaning — each still asks the same field the same question, reached one
segment further in. Every one is a plain field-path rename, across `tests/app_state.rs`,
`tests/features_window.rs`, `tests/switcher_forget_menu.rs` and `tests/terminal_focus.rs`.

The first one below is worth naming because it is not a bare field read: `terminal_focus.rs`
compares a four-tuple of `(terminal_focused(), terminal_released, focused_field, active_session)`
against the tuple it captured before a window-focus round trip. The rename reaches inside the tuple
and nowhere else — the same four facts are compared to the same four, and the round trip it pins
(FR-013–FR-015) is untouched.

```
was: assert_eq!((s.terminal_focused(),s.terminal_released,s.focused_field,s.active_session),before,"awindowfocusroundtripmustleavethekeyboardexactlywhereitwas\(released={released};FR-013–FR-015)")
was: assert_eq!(s.focused_field,None,"andthefieldmustnotstillbelieveitholdsit")
was: assert_eq!(s.focused_field,Some(FieldIAddWorktreeName))
was: assert_eq!(s.focused_field,Some(FieldIAddWorktreeName),"thefieldtheuseristypingintostillholdsit(FR-018)")
was: assert_eq!(st.focused_field,None)
was: assert_eq!(st.focused_field,None,"thefieldthatdoesholditisbelieved")
was: assert_eq!(st.focused_field,Some(FieldIAddWorktreeTicket),"thefieldthatalreadylostfocuscannottakeitawayfromtheonethathasit")
was: assert_eq!(st.focused_field,Some(FieldIRenameProjectName))
was: assert_eq!(st.focused_field,Some(FieldISettingsEnvIncludeTimeout),"thelaterclaimreplacestheearlierone—thereisoneslot")
was: assert_eq!(st.window_size,(0,0),"unknownuntilreported")
was: assert_eq!(st.window_size,(0,0),"unknownuntilthewindowsays")
was: assert_eq!(st.window_size,(1280,720))
was: assert_eq!(state.focused_field,None)
was: assert_eq!(state.focused_field,None,"nothingisfocusedtobeginwith")
was: assert_eq!(state.focused_field,Some(FieldIAddWorktreeName))
was: assert_eq!(state.focused_field,Some(FieldIRenameProjectName))
```

## T031 — the add-worktree form's two fields moved behind `state.worktree_form`

Two renames: `state.worktree_form` is `state.worktree_form.form` and `state.worktree_error` is
`state.worktree_form.worktree_error`, because both are now members of
`features::worktree_form::State`. The first drops a stutter rather than adding a segment — the
qualifier already names the form, so `worktree_form.worktree_form` would say it twice — and the
second keeps its full name because it is not the form's error: it also carries a refused project
open (FR-001a), which happens with no form on screen. Seventeen assertions changed spelling and
none changed meaning, across `tests/app_state.rs`,
`tests/indeterminate_stops_with_its_operation.rs` and `tests/open_project_git_gate.rs`.

Two of them read `state.worktree_form.as_ref().unwrap().error`, which is a **different** field —
`WorktreeForm`'s own error, inside the form — and they change only in the segment before it. Both
still ask the form about its own error, and the field they ask about did not move.

```
was: assert!(state.worktree_error.is_none())
was: assert!(state.worktree_error.is_none(),"discoveryansweringmakesafailureagainstthepreviousliststale")
was: assert!(state.worktree_form.as_ref().unwrap().error.is_none())
was: assert!(state.worktree_form.as_ref().unwrap().error.is_some())
was: assert!(state.worktree_form.is_none())
was: assert!(state.worktree_form.is_some())
was: assert!(state.worktree_form.is_some(),"formstaysopenforretry")
was: assert_eq!(state.worktree_error.as_deref(),Some("boom"))
was: assert_eq!(state.worktree_form.as_ref().map(|f|f.status),Some(WorktreeFormSCreating),"thecreatedidnotstart,sothistestwouldprovenothing")
was: assert_eq!(state.worktree_form.as_ref().map(|f|f.status),Some(WorktreeFormSEditing),"thefixtureismeanttobeanopenformwithnothinginflight")
was: assert_eq!(state.worktree_form.as_ref().unwrap().status,WorktreeFormSCreating)
was: assert_eq!(state.worktree_form.as_ref().unwrap().status,WorktreeFormSEditing)
was: assert_eq!(state.worktree_form.as_ref().unwrap().type_,None)
was: assert_eq!(state.worktree_form.as_ref().unwrap().type_,Some(ConventionalTFeat))
```

## T032 — the Settings form and the theme moved behind `state.settings`

Three fields became members of `features::settings::State`: `state.settings_draft` is
`state.settings.settings_draft`, `state.system_scheme` is `state.settings.system_scheme`, and
`state.theme_pref` is `state.settings.theme_pref`. All three keep their flat names — no stutter to
drop, since none of them repeats the word `settings` — so six assertions gained one segment and
none changed what it asks, across `tests/logical_state_ownership.rs`, `tests/system_theme.rs` and
`tests/terminal_focus.rs`.

The seventh is not a rename. `feature_write_isolation.rs`'s non-vacuity floor counted the fields
`app::State` declares and required at least 40 — a check that the struct scan is parsing a real
declaration rather than returning an empty list. This feature takes that count down on purpose:
after T032 the root parses to 39 fields, and by T036 it will be far fewer. As a root-only measure
the floor had become a countdown, lowered at every commit, and a floor lowered to fit stops
catching what it was for. It now counts both levels — the root's fields plus the members of every
feature struct the root holds — and keeps the same threshold of 40. That total is what a write can
resolve to, it is the number that was ~60 before this feature started, and the moves leave it
unchanged. The floor asks strictly more than it did: it is now vacuous only if *both* scans go
quiet, where before the feature structs were not read at all.

```
was: assert!(s.settings_draft.is_none())
was: assert!(scan.state_fields>=40,"`State`parsedtoonly{}fields—thestructscanisnotreadingwhatitthinksitis",scan.state_fields)
was: assert!(matches!(state.theme_pref,ThemePFollowSystem|ThemePLight|ThemePDark))
was: assert_eq!(state.system_scheme,SystemSDark)
was: assert_eq!(state.system_scheme,SystemSDark,"atransientdetect()failuremustnotoverwritethelast-knownscheme")
was: assert_eq!(state.system_scheme,SystemSUnspecified,"Ok(Unspecified)isagenuineOSreading,notafailure—itmuststillupdatethescheme")
was: assert_ne!(state.theme_pref,before)
```

## T033 — the project switcher, its menu and the rename moved behind `state.project`

Five fields became members of `features::project::State`. Three keep their flat names —
`project.forget_target`, `project.rename_draft`, `project.selector` — and two are trimmed, because
the qualifier already says `project`: `project_menu_open` is `project.menu_open` and
`project_switcher_open` is `project.switcher_open`, the same stutter `notifications.queue` (T028)
and `worktree_form.form` (T031) dropped. Thirty-one assertions changed spelling across
`tests/forget_project.rs`, `tests/logical_state_ownership.rs`, `tests/overlay_dismissal_delta.rs`,
`tests/overlay_dispatch_ordering.rs`, `tests/project_switcher.rs`, `tests/sidebar_state.rs`,
`tests/switcher_forget_menu.rs` and one in `src/shell/workspace.rs`; none changed meaning. The two
renames are visible in the text of the assertion rather than only in its path, which is why they
are called out here: an assertion that read `st.project_menu_open` now reads `st.project.menu_open`,
and it is asking the same question of the same bit.

```
was: assert!(!st.project_switcher_open)
was: assert!(!st.project_switcher_open,"openingtheconfirmmodalclosestheswitcher(open_overlay)")
was: assert!(!st.project_switcher_open,"openingtheoverflowmenuclosestheswitcher")
was: assert!(!state.project_switcher_open)
was: assert!(!state.project_switcher_open,"projectswitcher")
was: assert!(!state.project_switcher_open,"theprojectswitchersurvived{name}opening")
was: assert!(choose_a_non_repository().core.selector.is_none(),"thepickermustclosebeforetherefusalisreported,orthenotificationrenders\behindthemodal'sscrimandtheuserseesnothinghappen")
was: assert!(st.project_menu_open.is_some())
was: assert!(st.project_switcher_open)
was: assert!(st.project_switcher_open,"theswitcherpanelstaysopenbehindtherowcontextmenu,sotherowliststaysvisible")
was: assert!(state.forget_target.is_none())
was: assert!(state.project_menu_open.is_none(),"projectcontextmenu")
was: assert!(state.project_menu_open.is_none(),"theprojectcontextmenusurvived{name}opening")
was: assert!(state.rename_draft.is_none())
was: assert_eq!(st.forget_target,None,"nothingwasstagedforremoval")
was: assert_eq!(st.forget_target,Some(PathBfrom("/a")))
was: assert_eq!(st.project_menu_open,None)
was: assert_eq!(st.project_menu_open,None,"openinganotherpopoverclosestheprojectcontextmenu")
was: assert_eq!(st.project_menu_open,None,"sameprojecttogglesoff")
was: assert_eq!(st.project_menu_open,None,"thecontextmenucloses")
was: assert_eq!(st.project_menu_open.as_ref().expect("menuopen").anchor,(412,233),"thepanel'stop-leftcornersitsattheclickpoint,soitopensbelow-rightofthepointer")
was: assert_eq!(st.project_menu_open.as_ref().map(|m|m.path.clone()),Some(PathBfrom("/a")))
was: assert_eq!(st.project_menu_open.as_ref().map(|m|m.path.clone()),Some(PathBfrom("/b")),"adifferentprojectreplacesit—onlyonemenuiseveropen")
was: assert_eq!(st.project_menu_open.as_ref().unwrap().anchor,(100,100))
was: assert_eq!(state.forget_target.as_deref(),Some(Pnew("/a")))
```

## T034 — the worktree list, its menu and its dialogs moved behind `state.worktree`

Six fields became members of `features::worktree::State`, and five shed the `worktree` the
qualifier now carries: `hovered_worktree` is `worktree.hovered`, and
`worktree_delete_keep_branch`, `worktree_delete_target`, `worktree_menu_open` and
`worktree_rename_draft` are `worktree.delete_keep_branch`, `worktree.delete_target`,
`worktree.menu_open` and `worktree.rename_draft`. `worktrees` keeps its name, because trimming it
leaves nothing: it is the collection the feature is about, not a fact about one worktree, and
`worktree.worktrees` is a plural inside a singular rather than a word said twice.

Thirty assertions changed spelling across `tests/app_state.rs`, `tests/features_worktree.rs`,
`tests/logical_state_ownership.rs`, `tests/overlay_dismissal_delta.rs`,
`tests/session_default_no_worktree.rs`, `tests/sidebar_state.rs` and `tests/sidebar_tree.rs`. None
changed meaning: each still reads the same field of the same state and expects the same value. One
of them —
`assert!(state.worktree_rename_draft.as_ref().unwrap().error.is_some())` — asks about
`WorktreeRenameDraft`'s own `error`, a field that did not move; only the segment in front of it
did, exactly as T031's two `worktree_form.as_ref().unwrap().error` assertions.

```
was: assert!(!state.worktree_delete_keep_branch)
was: assert!(!state.worktree_delete_keep_branch,"defaultstodelete")
was: assert!(state.hovered_worktree.is_none())
was: assert!(state.worktree_delete_keep_branch)
was: assert!(state.worktree_delete_target.is_none())
was: assert!(state.worktree_menu_open.is_none())
was: assert!(state.worktree_menu_open.is_none(),"worktreecontextmenu")
was: assert!(state.worktree_rename_draft.as_ref().unwrap().error.is_some())
was: assert!(state.worktree_rename_draft.is_none())
was: assert!(state.worktrees.is_empty())
was: assert!(state.worktrees.iter().any(|w|w.dir_name=="feat-x"))
was: assert!(state.worktrees.iter().any(|w|w.dir_name=="feat-x"),"therowstandsuntilthedaemonconfirmstheremoval")
was: assert_eq!(st.worktrees.len(),1,"visibilityisaviewconcern—pruning,renamingandsessionlookupallreasonabout\existence,andahiddenworktreestillexists")
was: assert_eq!(state.hovered_worktree.as_deref(),Some("feat-a"))
was: assert_eq!(state.worktree_delete_target.as_deref(),Some("feat-x"))
was: assert_eq!(state.worktree_menu_open,None)
was: assert_eq!(state.worktree_menu_open.as_ref().unwrap().anchor,(120,300))
was: assert_eq!(state.worktree_menu_open.as_ref().unwrap().anchor,(140,610))
was: assert_eq!(state.worktree_rename_draft.as_ref().unwrap().text,"MyLogin")
was: assert_eq!(state.worktrees,before,"startingaDefaultsessionmustnotcreateaworktreeentry")
was: assert_eq!(state.worktrees.len(),1)
was: assert_eq!(state.worktrees.len(),1,"theincludestilllands")
was: assert_eq!(state.worktrees.len(),6)
```

## T035 — the sidebar's ten fields moved behind `state.sidebar`

Ten fields became members of `features::sidebar::State`. Six shed the `sidebar` the qualifier now
carries: `sidebar_filter_open`, `sidebar_filters`, `sidebar_hidden`, `sidebar_scroll_offset`,
`sidebar_viewport_height` and `sidebar_width` are `sidebar.filter_open`, `sidebar.filters`,
`sidebar.hidden`, `sidebar.scroll_offset`, `sidebar.viewport_height` and `sidebar.width`. The
other four never carried it — `default_expanded`, `expanded`, `pending_reveal_scroll`,
`show_agent_worktrees` — and keep their names.

Sixty-two assertions changed spelling across `tests/app_state.rs`, `tests/forget_project.rs`,
`tests/logical_state_ownership.rs`, `tests/overlay_dismissal_delta.rs`,
`tests/overlay_dispatch_ordering.rs`, `tests/overlay_registry.rs`, `tests/sidebar_state.rs`,
`tests/sidebar_tree.rs`, `tests/switch_active.rs` and `src/main.rs`. None changed meaning.

One name needs care, because `expanded` is not unique in this codebase: a `SidebarEntry`'s node
has an `expanded` of its own, and sixteen assertions read *that* one. Those are untouched — they
still say `node.expanded`, `feat_a.expanded` and so on, because the node's flag is not the
feature's set and did not move. Only the reads of the root's `expanded` — the `BTreeSet<String>` of
which worktree rows are open — became `state.sidebar.expanded`.

```
was: assert!(!Sdefault().show_agent_worktrees)
was: assert!(!st.default_expanded,"andtheDefaultrow,whichhasnonametopruneby,isresetoutright")
was: assert!(!st.expanded.contains("wa1"),"/a'sexpansionisprunedby/b'sworktreenames,soarowopenedinoneprojectcannot\renderinanother(FR-007)")
was: assert!(!st.pending_reveal_scroll,"andnothingisarmedtoscrollto,whichiswhatstopsascrollfiringlateragainst\anunrelatedrow(invariantI5)")
was: assert!(!state.expanded.contains("feat-a"))
was: assert!(!state.pending_reveal_scroll)
was: assert!(!state.pending_reveal_scroll,"andnothingisarmedtoscrollto.Thisisanapp-initiatedclearlikethecloseand\removearms,anditgoesthroughthesamefunctionforthesamereason—ascrollarmed\withnotargetstaysarmed,thenfiresagainstwhateverrowappearsnext(invariantI5)")
was: assert!(!state.pending_reveal_scroll,"butnothingisopenedorscrolledontheuser'sbehalf:theyclickedarowtheycould\alreadysee,andscrollingitwouldmovethelisttheywerereading(FR-006)")
was: assert!(!state.pending_reveal_scroll,"thereisnorowtoscrollto.Anarmedscrollwithnotargetstaysarmed—nothingdrains\it—andthenfiresagainstwhateverrowappearsnext;FR-001aforbidsscrollingatall\whentheuserclosesthesessiontheywereon(invariantI5)")
was: assert!(!state.pending_reveal_scroll,"withnothingarmedtoscrollto")
was: assert!(!state.show_agent_worktrees)
was: assert!(!state.show_agent_worktrees,"theincomingprojectmustbeenteredwithagentworktreeshidden")
was: assert!(!state.sidebar_filter_open)
was: assert!(!state.sidebar_filter_open,"openinganoverlaymustclosethefilterpanel")
was: assert!(!state.sidebar_filter_open,"precondition:thepanelclosed")
was: assert!(!state.sidebar_filter_open,"sidebarfilterpanel")
was: assert!(!state.sidebar_filter_open,"thefilterpanelsurvived{name}opening")
was: assert!(!state.sidebar_filters.contains(&feat))
was: assert!(!state.sidebar_filters.contains(&feature))
was: assert!(!state.sidebar_hidden)
was: assert!(both.sidebar_filter_open,"andthepopoverbeneathitisuntouched—oneEscapeclosesonesurface")
was: assert!(st.pending_reveal_scroll,"therevealedrowisnousebelowthefold,soaswitcharmsthescrollthatbringsit\intoview(FR-008)")
was: assert!(state.default_expanded)
was: assert!(state.expanded.contains("feat-a"))
was: assert!(state.expanded.contains("feat-a"),"anditstaysopenbybecomingordinaryuser-openstate,whichisthehonestdescription\ofwhattheuserwaslookingat(invariantI3)")
was: assert!(state.expanded.contains("feat-x"))
was: assert!(state.expanded.contains("fix-b"),"its*open*statesurvivesthecommit,though—onlyitspresencegoes(contract§5.3)")
was: assert!(state.expanded.contains("kept-open"),"andtherestoftheprojectisexactlyasitwas(FR-006)")
was: assert!(state.expanded.contains("kept-open"),"andtherestoftheprojectisuntouched—amemorythatcannotbehonouredmustnotcost\theuseranythingelse(FR-006)")
was: assert!(state.expanded.is_empty())
was: assert!(state.expanded.is_empty(),"anditisopenwithoutanythingbeingwrittentotheuser'sownexpansionset:\open-nessisderived,soaworktree-listreplacementhasnothingtolose(FR-001b)")
was: assert!(state.pending_reveal_scroll,"andbroughtintoview")
was: assert!(state.pending_reveal_scroll,"andthenewcurrentsessionarmsitsownscroll")
was: assert!(state.show_agent_worktrees)
was: assert!(state.sidebar_filter_open)
was: assert!(state.sidebar_filter_open,"precondition:thepanelisopen")
was: assert!(state.sidebar_filters.contains(&feat))
was: assert!(state.sidebar_filters.contains(&feat),"togglingpanelvisibilitymustnotaltertheactivefilterset(FR-007/FR-008)")
was: assert!(state.sidebar_filters.contains(&feature))
was: assert!(state.sidebar_filters.is_empty())
was: assert!(state.sidebar_hidden)
was: assert!(state.sidebar_hidden,"theflagmustliveonState")
was: assert_eq!(app.core.sidebar_scroll_offset,0,"therevealknowswhereitsentthelist,soitmustnotwaittobetold:leaving734\hereisBUG-002,andthenextarrivalmeasuresitsrowagainstapositionthepanel\leftlongago")
was: assert_eq!(state.expanded,expanded_before)
was: assert_eq!(state.sidebar_filters,chosen,"cancelling{name}reachedintothesidebar'sfilters,whichitdoesnotown")
was: assert_eq!(state.sidebar_filters,chosen,"closingthepanelisputtingthechooseraway,notclearingthechoice—asidebarthat\silentlyunfiltereditselfeverytimethepanelcollapsedwouldbeunusable")
was: assert_eq!(state.sidebar_filters,filters_before)
was: assert_eq!(state.sidebar_filters.len(),2)
was: assert_eq!(state.sidebar_viewport_height,0,"precondition:nolayouthashappenedyet")
```

T040 rolls these up for the phase; each move is adjudicated as it lands, so the freeze is green at
every commit rather than only at the end (contract C.1).
