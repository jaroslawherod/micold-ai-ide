# Rubric: bug patch (bugfix flow, after `speckit-bugfix-verify`)

- `bugs/BUG-<issue>.md` has reproduction steps. The reviewer follows them on `origin/main` and they
  reproduce the failure. If not, BLOCKER.
- The root cause names a file and a mechanism, not a symptom. The reviewer confirms it in that code.
- Every requirement the patch adds to spec.md describes behaviour the spec already intended: a
  missed edge case, or a conflict resolved in favour of the stated user story. New behaviour is
  MAJOR and sends the run to the feature flow.
- Reopened tasks carry `(reopened — BUG-<issue>)`. Fix tasks are few. The first is a regression test.
- `speckit-bugfix-verify` reports the BUG as Patched, with no orphaned references.
