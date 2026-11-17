# UI end-to-end suite (placeholder)

Blocked on #1954 (the web image). The Playwright tests for the web client go here, run by the
`ui` suite of `scripts/e2e-kind.sh` against the same kind release as the API suites (#1981, FR-013).

- Base URL: the harness will pass the web Service's URL (port-forwarded, `web.enabled=true`) in
  `MICOLD_E2E_WEB_URL`. With no web image the suite stays skipped.
- Until then `crates/micold-e2e/tests/ui/main.rs` is a placeholder that skips naming #1954.
- To enable: replace `PLACEHOLDERS` for `ui` in `scripts/e2e-kind.sh`, add the Playwright project
  here, and let the `ui` suite invoke it.
