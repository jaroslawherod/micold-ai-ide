# Task: act on a review, and run the next round

When: a reviewer returned.

## After it returns

- Verify each finding against the code or artifact. Decline a wrong one, or one that contradicts
  the spec, and record the reason in the ledger's *Declined review findings*.
- Fix every BLOCKER and MAJOR that holds up. Fix a MINOR only if it takes a few minutes.
- **Stop when clean.** `CLEAN`, or only MINORs: the review is done; fixing MINORs needs no new
  round. Otherwise fix, commit, and dispatch a **new** reviewer, never the old one.
- **Exception: a fix that changes prose only** (wording in spec, plan, docs or comments; no code,
  test, config or behaviour-bearing requirement) needs no new round. Check the fix yourself
  against the finding and record `fixed, prose only` in the ledger. A fix that adds or changes a
  claim about behaviour, evidence or results is not prose only: re-review it.

## Round 2 and later

A re-review checks the fix diff, not the whole artifact again, on `model: "sonnet"`. Its prompt has
parts 1 to 5 of [review.md](review.md), except that part 5 gives the paths and the brief but not
the full diff, and adds:

- the previous round's findings, each marked fixed (with how) or declined (with the reason);
- the fix diff: `scripts/autopilot/review-snapshot.sh diff <last round's snapshot>`.

It confirms each fix holds and each declined reason stands, and applies every rubric item to the
fix diff. It does not reopen what the last round passed. Review B always re-runs **Verify**, and
runs again only after fixes to its own BLOCKER or MAJOR findings.

Run a full round instead, with round 1's model, when:

- the last round ended with `+<n> more`. This round finds the rest and does not count toward the
  limit below;
- the snapshot is missing from the ledger, or `review-snapshot.sh diff` exits 2 (unknown, or
  stale after a rebase).

## Round limit

A round counts only when it follows fixes to that review's own BLOCKER or MAJOR findings, or is
round 1 (a unit continuing a handover is past round 1 when *Review rounds* already lists the
review). These do not count unless they find a BLOCKER or MAJOR: a `+<n> more` continuation, a full
round forced by a missing or stale snapshot, and a round run only because another review's or a
red gate's fixes changed code after its snapshot.

At most 3 counted rounds. A last round that still finds a BLOCKER or
MAJOR is an escalation (category 5); never run one more.

Next: the task that dispatched the review.
