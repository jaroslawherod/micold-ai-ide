# Contract: Session Service Lifecycle

**Feature**: 028-client-managed-daemon

The observable contract between the application, the session service, and the machine. Every clause
is stated so that it can be checked from outside the process — that is what makes it a contract
rather than a description of the implementation.

## §1 Starting

1. The application MUST start a service when, and only when, it needs one and none answers the
   endpoint. No other actor starts it: no installer, no service manager, no login item.
2. Concurrent starters MUST converge on one service. The discriminator is unchanged from feature
   010: `connect()` proves liveness, an exclusive `flock` arbitrates recovery, and the re-check after
   taking the lock is mandatory.
3. A service MUST NOT be startable by socket activation. `LISTEN_FDS` adoption is removed; a daemon
   handed a listener on fd 3 has no defined behaviour under this contract.

## §2 Living

4. A service MUST outlive every client. Client exit — clean, crashed, or killed — MUST change no
   session's fate and MUST NOT signal the service.
5. Exactly one presence count exists, incremented after a successful handshake and decremented when
   the connection ends for any reason. A refused handshake MUST NOT count.
6. A connection that ends without a clean close MUST be counted as gone within 60 seconds.

## §3 Stopping

7. The service MUST stop when, for 30 continuous minutes, its presence count has been zero.
8. The 30 minutes are measured on a monotonic, suspend-inclusive clock. Time the machine spends
   suspended counts. A wall-clock change MUST NOT move the deadline in either direction.
9. The count reaching zero starts the window; the count leaving zero cancels it. A service that has
   never had a client is counted as idle from its own start.
10. Live sessions MUST NOT extend the window. They end with the service, and are afterwards
    presented as `InterruptedResumable` — never lost, never auto-resumed.
11. The stop MUST be an unwind, in this order: diagnostics line, stop accepting, persist sessions as
    interrupted-resumable, terminate session process trees, release the endpoint and the lock,
    return. The lock is released last.
12. After the stop, an observer MUST find: no process descended from the service, no socket or port
    bound by it, no lock held by it, and no file at the endpoint path.
13. The stop MUST leave the next start indistinguishable from a first start.

## §4 Racing

14. A client connecting while §3 is in progress MUST end attached to a working service — the
    departing one if it accepted, a fresh one otherwise — without a user-visible failure and without
    a manual retry.
15. The client's reconnect MUST absorb a single transient failure without raising the connection
    banner.

## §5 Placement

16. §1–§4 hold identically for a service running on the host and one running in the sandbox. The
    presence count, the window, the clock and the order in §3 are the same code.
17. In the sandbox the service is PID 1, so §3's unwind stops the container. The container MUST
    afterwards report status `exited` with its restart count unchanged.
18. **Exception (approved amendment, spec FR-022):** while the keep-it-running
    opt-in is on, the sandbox's restart policy is `unless-stopped` and clause 7 does not apply to
    it. The runtime restarts any container that exits under that policy — measured, three times in
    seven seconds — so an idle stop and that opt-in cannot both hold. The opt-in wins, and its copy
    says so.

## §6 Diagnostics

19. An idle stop MUST write one diagnostics line, before teardown, naming inactivity as the reason.
20. A crash, a kill, or an out-of-memory ending MUST NOT produce that line. Reading the diagnostics
    MUST therefore distinguish the two without further evidence.
