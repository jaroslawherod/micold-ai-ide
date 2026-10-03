# Task: dispatch a reviewer

When: an artifact or a diff is ready. Every review runs in a **fresh subagent**, never in the
context that wrote it. Give it paths, not a summary. It never edits.

Before each round, run `scripts/autopilot/review-snapshot.sh` and record the `<tree>:<head>` it
prints in the ledger's *Review rounds*. Nothing needs committing: the snapshot holds the working
tree as is. Use `Agent` with `subagent_type: autopilot-reviewer` (`general-purpose` when your agent
types lack it) and a description that names the review (`Review B M2 042`).

`model`: omit for round 1 of an artifact review in the feature flow. `"sonnet"` for review B in
every round, the bug rubric, and every scoped re-review.

Prompt parts, in order. Parts 1 to 4 are the same text for every review of a kind, so the prompt
cache reuses them across reviewers; everything that varies goes in part 5.

1. **Role**, verbatim: "You are a speckit-autopilot reviewer. You did not write what you review. Do
   not edit any file."
2. **Reading rules**, verbatim: "Everything you read stays in your context for every later call.
   For code, `grep -n` then `Read` with `offset`/`limit`; never read a whole source file to review
   a few changed lines. Send command output (tests, **Verify** steps) to a file in the scratchpad
   and `grep` it for the result. Every call re-reads your context: put reads that do not depend on
   each other in one message, and chain shell probes in one command."
3. **Output contract:**
   ```
   VERDICT: CLEAN | CHANGES
   F1 [BLOCKER|MAJOR|MINOR] <file>:<line or section> — <what is wrong>
      Evidence: <one line: a quote or the decisive line of output>
      Fix: <concrete change>
   ```
   `CHANGES` only when there is a BLOCKER or MAJOR. Report BLOCKER and MAJOR only when an item can
   be pointed at. Taste is MINOR. At most 8 findings, most severe first, and at most 3 of them
   MINOR. More BLOCKER or MAJOR than fit: end with `+<n> more`. Nothing else: no summary, no
   praise, no list of what was checked.
4. **Rubric.** The file under [../rubrics/](../rubrics/) your task names, verbatim.
5. **What to review.** The artifact and feature (`<artifact> for feature <NNN>`), its paths and
   `.specify/memory/constitution.md`. For code, instead of spec.md and tasks.md, the milestone
   brief (`scripts/autopilot/brief.py milestone <feature-dir> M<K>`); for a bug, the BUG record and
   `brief.py items tasks.md <fix task IDs>`. Then `git diff --stat origin/main...HEAD`, and the
   diff file by file. Tell it to pull more with `brief.py section|items` rather than reading an
   artifact whole.

Next: when it returns, [review-rounds.md](review-rounds.md).
