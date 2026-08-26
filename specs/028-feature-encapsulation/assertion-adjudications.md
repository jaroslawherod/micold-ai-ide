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

(Observed output recorded by T005 below.)
