# Plan drift, M1

No unplanned file. Planned-but-different notes:

- `typeScale` entries carry `size`, `lineHeight`, `weight` only: data-model.md lists a `font` field, but the token set has no font token (fonts are out of scope, spec.md line 103), so none is emitted.
- Tests in `ts.rs` also use `tables::SPACING` and `tables::STATES` names as `camelCase` keys (`disabledContent`), as research.md says.
