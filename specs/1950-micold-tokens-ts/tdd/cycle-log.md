# TDD cycle log: 1950 micold-tokens-ts, M1

| Cycle | Tasks | Red (evidence) | Green | Refactor |
|---|---|---|---|---|
| 1 | T001-T002 | Refactor under green: tables extracted from `css.rs`; existing `tokens::css` tests are the guard; `micold-tokens-css` output compared byte-for-byte before and after (`cmp`, 11997 bytes, identical) | `cargo test -p micold-core --lib tokens::` | css tests iterate `tables::*` |
| 2 | T003-T005 | `module()` stub returning `""`; `cargo test -p micold-core --lib tokens::ts`: 7 failed, 2 passed (`no line \`  light: {\` in the module`, header `None != Some(...)`, `object(0,&[])` `"" != "{}"`) | T006: `module()`, `object`, `array`, `camel` implemented; 9 passed | none |
| 3 | T007-T008 | `micold-tokens-ts` stub `fn main(){}`; `cargo test -p micold-core --test main tokens_ts_bin`: 3 failed (no file written, exit 0 not 1, stdout empty) | T008 binary; 3 passed | none |
| 4 | T009 | n/a (generated artifact) | `cargo run -p micold-core --bin micold-tokens-ts -- apps/client/src/theme/tokens.generated.ts` | none |
