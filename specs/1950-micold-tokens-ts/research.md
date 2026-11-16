# Research: 1950 micold-tokens-ts

- **Key casing**: camelCase from the Rust identifier (`on_surface_variant` -> `onSurfaceVariant`); radii/motion names from the CSS kebab names -> camelCase (`extra-small` -> `extraSmall`, `short-1` -> `short1`). Every key matches `^[a-z][A-Za-z0-9]*$`, so none can be a reserved word issue in object-key position. Rejected: quoted kebab keys (awkward to use from TS).
- **Elevation is scheme-dependent** (shadow alpha and tint differ by scheme). FR-003's "exported once" holds: `elevation` is one export whose level entries mirror `elevation::Level`: `level`, `surface` (a colour-role key), `shadow: null | { offsetY, blur, alpha: { light, dark } }`; the tinted surface colours are colour roles and live under `colors.light/dark`. Rejected: duplicating the whole family per scheme.
- **Numbers**: integers printed as integers; fractions via Rust `{}` on `f32`/`f64` (shortest round-trip, no exponent for this range, locale-free). A test asserts no `e` and no trailing `.0` in the output.
- **Units**: unitless numbers (dp, ms, opacity 0-1); easings as `[x1, y1, x2, y2]` tuples, which React Native `Easing.bezier` takes directly. Rejected: CSS strings (`200ms`, `cubic-bezier(...)`).
- **Atomic write**: write `<path>.tmp` in the same directory, then `rename`; on error remove the temp and exit 1 with the path and the io error.
- **Regeneration command** (printed by the drift test): `cargo run -p micold-core --bin micold-tokens-ts -- apps/client/src/theme/tokens.generated.ts`.
- **Not emitted**: `tracking`, anatomy, density (as in CSS). Spacing is emitted (CSS emits it; the client needs it).
