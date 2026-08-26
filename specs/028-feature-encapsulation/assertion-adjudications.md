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
