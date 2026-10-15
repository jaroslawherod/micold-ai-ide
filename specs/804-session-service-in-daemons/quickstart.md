# Quickstart

## Part A: automated
`mise run test-core`, then `mise run gate`.

## Part B: visual pass
1. Open Settings: the rail lists Appearance, Terminal, Environment, Daemons, GitHub, Jira, no
   Session service.
2. Daemons > edit a container daemon: runtime, image source/reference/file, credentials, network,
   limits, keep-running all present and scroll; an out-of-range limit shows beside the field.
3. Edit a second daemon: its values differ from the first.
4. Tick a credential share in the form: the Daemons rail row shows *Sharing* at once, and from any section after save.
5. Change a limit on a running daemon and save: the confirmation appears.
6. Host daemon: container groups inert, keep-running caution shown. SSH daemon: none shown.
