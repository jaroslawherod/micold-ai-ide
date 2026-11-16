# Contract: message catalog (`apps/client/src/i18n/`)

- Files `en.json` (source), `pl.json`; `pseudo` is computed from `en` (test/dev only). Shape:
  `{ "<area>.<id>": "<ICU message>" }`, e.g. `sidebar.sessionCount: "{count, plural, one {# session} other {# sessions}}"`.
- Ids are a TypeScript union derived from `en.json`; `t(id, values)` is the only way a component gets text
  (`useT()`); accessibility labels, page title (`document.title` via a `useTitle(id)` hook) and state names
  go through it.
- Locale resolution order: stored `micold.locale` -> `navigator.languages` (nearest language, e.g.
  `pl-PL` -> `pl`) -> `en`. Missing message in the chosen locale -> English text, and a `console.warn`
  in development only.
- Dates and relative times via `Intl.DateTimeFormat` / `RelativeTimeFormat` through `platform/intl.ts`;
  when `Intl` lacks a feature, a plain English fallback is used.
- Checks: ids of `pl` subset of `en`; placeholder names equal; Polish plural categories `one/few/many/other`;
  lint `no-literal-string` over `app/` and `src/` (exempting tests and catalog files).
- Locale control: `LocalePicker` (keyboard reachable, named via the catalog) on the sign-in page and in the
  sidebar header; switching updates the `IntlProvider` value without reload and writes the preference.
