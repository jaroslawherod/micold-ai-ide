# TDD cycle log: 968 M1

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T003 → T001, T004 | `micold_core::quick_switch::{filter, reseat, step}`: empty query keeps order; `logpg` keeps `Login page`; prefix > run > scattered, stable; case ignored; no accent folding; spans past non-ASCII; no typo tier; `reseat` key / first selectable / `None`; `step` skips unselectable, stops at ends; 500 × 10 queries < 100 ms | `cargo test -p micold-core --lib quick_switch` against stubs (`Vec::new()`, `None`, `at`): `test result: FAILED. 4 passed; 8 failed` (the 4 passing assert emptiness/`None`/timing, which a stub trivially meets) | same command: `12 passed; 0 failed` |
