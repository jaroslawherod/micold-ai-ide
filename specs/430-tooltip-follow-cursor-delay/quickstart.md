# Quickstart: Tooltip follows the cursor and waits before showing

## Part A — automated

```bash
mise run test-core                       # ShowTimer table
cargo test -p micold-client tooltip      # placement units, glue, builder
cargo test -p micold-client --test idle_requests_no_frames
mise run gate                            # before any push
```

Every existing tooltip test passes unchanged (SC-004).

## Part B — visual pass (the `visual-pass` skill, component showcase, Floating section)

1. **Follow**: hover the large trigger; the panel sits beside the pointer and moves with it; stop
   moving and it stays. (US1.1)
2. **Edge flip**: move to the window's right edge, then the bottom edge, then the corner; the panel
   flips to the pointer's other side and never covers the pointer. (US1.2, US3.2)
3. **Leave**: leave the trigger; the panel closes. (US1.3)
4. **Delay**: hover the show-delay trigger; nothing for the delay, then the panel; leave before the
   delay and nothing appears; move inside the trigger during the wait and the wait does not restart.
   (US2.1, US2.2, edge case)
5. **Delay plus follow**: the panel appears beside the pointer's current position and tracks it. (US2.4)
6. **Existing**: the four fixed poses and the 3 s rest pose behave as before. (FR-006)
7. **Idle**: with no tooltip open, CPU is idle. (US2.5, SC-005)
