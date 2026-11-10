# Quickstart

1. `scripts/autopilot/review-facts.py --out-dir "$SCRATCH"` on a branch with a Rust change: open the printed file; check Files, Symbols, Callers, and `unknown` coverage.
2. On a docs-only branch: exit 0, symbols `skipped`.
3. `git checkout --detach`: exit 1 with a reason.
4. Run the staged flow on a fixture diff with an unchecked error path; the verdict should report it with probe evidence.
5. Run a staged verdict through `brief.py` and the ledger reader; they accept it unchanged (SC-004).
6. `mise run gate` (includes `scripts/tests/autopilot.test.sh`).
No visual pass (no UI).
