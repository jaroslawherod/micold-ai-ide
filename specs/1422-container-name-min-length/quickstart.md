# Quickstart: Container name minimum length

## Part A: automated
1. `mise run test-core` : predicate tests pass.
2. `cargo test -p micold-client --test features_daemons` : new minimum-length, hint and legacy tests pass.
3. `mise run gate`.

## Part B: visual pass
1. Settings, Daemons, Add daemon, runtime container, name `a`, container name empty: no placeholder; Save shows "A container name needs at least 2 characters." beneath the container name field; no daemon added.
2. Type `ab` in the container name field, Save: daemon created.
3. Name `ab`, container name empty: placeholder `ab`; Save creates it.
4. Container name `x`: refused with the same message.
5. With a registry containing container name `a`: it lists unchanged; edit and Save is refused with the message.
