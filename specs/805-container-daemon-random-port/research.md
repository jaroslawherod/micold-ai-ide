# Research: random port for container daemons (805)

## R1. Where to pick
Decision: at save, in the Daemons form reducer, using the registry as the taken set.
Rejected: pick at start (breaks the stable address, FR-003); pick in a shell after save (the user
could not see it at once, and the stored value would be zero until then).

## R2. How to get a random free port
Decision: ask the OS with `TcpListener::bind("127.0.0.1:0")` and drop it; retry up to 16 times if
the port is already stored for another daemon.
Rejected: the `rand` crate plus a probe bind (new dependency, Principle V vetting, and a range
choice to defend); scanning from a fixed base (not random, collides across installs); keeping the
listener open (the container runtime must bind it).

## R3. Race between picking and the container binding
Decision: accept it. The port is reported at start by the existing `PortUnavailable` error (story 3,
spec edge case); never re-rolled.
Rejected: re-pick on failure (silent change of address, contradicts story 2 and 3).

## R4. Cleared port on edit
Decision: keep the stored port (FR-006). Rejected: re-roll (unstable), or error (spec says keep).

## R5. Testability
Decision: inject the port source through `Model::next_port`; core picker takes a closure.
Rejected: a global or env override (hidden state); binding real ports in client tests only
(cannot test the no-free-port path).

## R6. Start-time error
Decision: reuse `RuntimeError::PortUnavailable` text and remedy as is; both already name the port
and Settings > Daemons.
Rejected: new error type (duplicates it).

## R7. Several windows
Decision: each window keeps its own registry copy; the save also avoids ports in the store's current
registry (`Model::taken_elsewhere`). A same-instant double save is accepted (last-writer-wins, already
true for all settings) and surfaces as the start error.
Rejected: a cross-window lock (new mechanism for a rare race); trusting the window's copy only (a
stale copy would pick a port another window just stored).

The store write replaces the whole registry with the saving window's copy (`set_registry`); that is existing behaviour, out of scope for 805. `taken_elsewhere` only keeps the picked port clear of the other window's ports.
