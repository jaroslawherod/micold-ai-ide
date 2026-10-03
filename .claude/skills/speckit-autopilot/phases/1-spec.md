# Phase 1: spec unit

1. `speckit-specify` with the feature description (and `BUG-<k>` with its repro, when a bug switched
   here). It offers up to 3 clarification questions: do not ask them. Leave them as
   `[NEEDS CLARIFICATION]` markers for Phase 2. After a bug switch, the branch already holds the
   committed BUG record; it ships in the design PR.
2. Create the ledger as `autopilot.md` in the feature directory from
   [../templates/autopilot-ledger.md](../templates/autopilot-ledger.md). Set **Worktree branch** to the exact output of
   `git branch --show-current`.
3. A fresh reviewer checks spec.md against the spec rubric.
4. Commit, and push the branch (`git push -u origin HEAD`) so the work survives, but open no PR: the
   spec ships in the design PR. Return `DONE` with `PR: none`.
