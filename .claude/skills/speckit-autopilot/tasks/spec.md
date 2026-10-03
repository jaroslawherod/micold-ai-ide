# Task: write the spec (spec unit)

When: the feature flow starts, or another flow returned `NEXT: feature`.

1. `speckit-specify` with the feature description (and the BUG record with its repro, when a
   bugfix unit switched here). **The spec's number is the run's issue number**: give the skill
   `SPECIFY_FEATURE_DIRECTORY=specs/<issue>-<short-name>`, which it uses as is. GitHub hands
   issue numbers out centrally, so two runs never take the same one. The skill offers up to 3
   clarification questions: do not ask them. Leave them as `[NEEDS CLARIFICATION]` markers for the
   clarify unit. After a bugfix switch the branch already holds the committed BUG record: it ships
   in the design PR.
2. Create the ledger as `autopilot.md` in the feature directory from
   [../templates/autopilot-ledger.md](../templates/autopilot-ledger.md). Set **Worktree branch** to
   the exact output of `git branch --show-current`, and **Kind**, **Issue** and **Input** from your
   prompt.
3. A fresh reviewer checks spec.md: [review.md](review.md) with
   [../rubrics/spec.md](../rubrics/spec.md).
4. Commit, and push the branch (`git push -u origin HEAD`) so the work survives, but open no PR: the
   spec ships in the design PR.

Hands on: return `DONE` with `PR: none`. Next unit: clarify.
