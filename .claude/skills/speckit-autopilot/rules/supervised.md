# The `supervised` argument

A trailing `supervised` argument (`/speckit-autopilot #<n> supervised`, as the `autopilot-supervisor`
skill starts a run) marks the run as supervised. Drop the word before routing, and when the ledger
is created write the literal line `- **Supervised**: yes` under **Kind**. `scripts/autopilot/supervised.sh`
matches that line, and an escalation then ends the turn instead of blocking on a question.
