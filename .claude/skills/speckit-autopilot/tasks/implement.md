# Task: implement one milestone (milestone unit)

When: the previous PR merged and the ledger names milestone K as next (feature flow), or the
bugfix unit returned the fix's task IDs (bugfix flow).

0. **Brief.** `scripts/autopilot/brief.py milestone <feature-dir> M<K>` prints the milestone's
   block, its tasks, the requirements it satisfies and its stories' scenarios. Work from that. Do
   not read spec.md or tasks.md whole; pull anything else with `brief.py section <file> <heading>`
   or `brief.py items <file> FR-012 T031`. **For a bugfix** there is no milestone block: read the
   BUG record, then `brief.py items tasks.md <fix task IDs>` and the patched requirements by ID.
1. `speckit-implement` with exactly:

   ```
   Milestone M<K> only: tasks T0xx–T0yy. Do not start any other task. Stop when these are done.
   ```

   Its mandatory `tdd.run` hook drives red → green → refactor. For a bugfix, the first task is a
   regression test that fails on `origin/main` for the reported reason.
2. If it asks "proceed anyway?", never answer "yes": close the checklist item as the tasks unit
   does ([tasks.md](tasks.md) step 3: confirm, or fix the spec or plan), and escalate only when it
   needs a user decision.
3. Confirm every task in the range is ticked and none outside it. An open task in the range means
   the milestone is not done. Then check your context ([../rules/context.md](../rules/context.md),
   *Hand over at 150k*).

Next: [verify.md](verify.md).
