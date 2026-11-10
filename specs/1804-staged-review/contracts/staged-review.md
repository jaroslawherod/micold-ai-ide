# Contract: staged review flow (dispatched by the unit)

Applies to code-diff reviews (milestone review A and B, chore, bug fix diffs, round 2+ on the fix diff). Artifact reviews stay single-agent.

1. **Facts** (unit, no model): `review-facts.py --out-dir <scratchpad>`. Failure -> single-agent review.
2. **Plan** (fresh `autopilot-reviewer`, strong model): prompt parts 1 to 4 of `review.md` for stage 2, plus the facts file path, risk (one sentence from the unit), acceptance criteria (`brief.py milestone`). Returns at most 6 probes: `P<n> question / targets / evidence to return`. It does not read the source tree. No probes is allowed for docs-only or empty diffs.
3. **Probes** (unit, one message): one `autopilot-worker` per probe, `model: "haiku"` (`"sonnet"` if the probe runs tests), returns at most 40 lines or `UNANSWERED: <reason>`.
4. **Verdict** (fresh `autopilot-reviewer`, strong model): reads facts file, plan, evidence and the existing output contract and rubric; writes `VERDICT:` and `F<n>` unchanged, at most 8 findings.

Fallback (FR-011): facts failure, any `UNANSWERED`/errored probe, discards the staged result and runs the single-agent review. A stale snapshot discards the result and re-runs the round on a fresh snapshot (staged again, or single-agent if the facts script fails). A staged `CLEAN` with a missing probe is never accepted. Probes are bounded to the plan; a verdict that needs more reading may name it as a finding and the unit falls back.

Shadow mode: the unit dispatches the single-agent review and the staged flow in the same round; actions follow only the single-agent verdict; both verdicts, finding counts and cache-read tokens go to `shadow.md`. Round-limit and snapshot rules (FR-010) count the single-agent verdict. Shadow ends after three code-diff PRs (the next three milestone or chore reviews after this change merges) and the decision is recorded (FR-012, FR-013).

Token accounting: the staged total is the sum of stage 2, every probe worker and stage 4. After stage 4 the unit re-checks the snapshot with `review-snapshot.sh`; a mismatch discards the staged result.
