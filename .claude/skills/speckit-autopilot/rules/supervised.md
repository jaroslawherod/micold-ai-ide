# The `supervised` argument

A trailing `supervised` argument (`/speckit-autopilot #<n> supervised`, as the `autopilot-supervisor`
skill starts a run) marks the run as supervised. Drop the word before routing, and when the ledger
is created write the literal line `- **Supervised**: yes` under **Kind**. `scripts/autopilot/supervised.sh`
matches that line, and an escalation then ends the turn instead of blocking on a question.

Before the ledger exists a supervised run asks nothing: where [flows/choose.md](../flows/choose.md)
would ask, take the recommended flow and write the choice in the ledger's **Input** line when it
is created.

Who notifies: a supervised run sends no `PushNotification`; the supervisor sends the one notification for a batch of questions. An unsupervised run notifies as before.

What the supervisor does when a supervised run stops: [supervisor-close-out.md](supervisor-close-out.md).
