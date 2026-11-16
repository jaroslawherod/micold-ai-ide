# Plan drift, M1

No unplanned file. Planned-but-different notes:

- `typeScale` entries carry `size`, `lineHeight`, `weight` only: data-model.md lists a `font` field, but the token set has no font token (fonts are out of scope, spec.md line 103), so none is emitted.
- Tests in `ts.rs` also use `tables::SPACING` and `tables::STATES` names as `camelCase` keys (`disabledContent`), as research.md says.

Recorded for plan-drift.sh (its path pattern has no `.ts` extension):

- `apps/client/src/theme/tokens.generated.ts`: T009's deliverable, named by the task but not matched by the script.
- `contracts/tokens-module.md`, `micold-tokens-css.rs`: referenced by tasks as sources to follow, not files to change; "MISSING" is expected.
