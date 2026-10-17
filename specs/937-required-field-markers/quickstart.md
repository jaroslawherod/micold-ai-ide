# Quickstart: Required-field markers in forms (937)

## Part A — automated

```bash
scripts/gate.sh            # or: mise run gate
mise run test-core         # micold_core::required only
```

Expected: green, including `crates/micold-core/src/required.rs` tests, the `features_*` tests for
each draft's `required_fields()`, the anatomy tests for `.required(..)`, and
`tests/required_call_sites.rs` ([contract C5](contracts/required-field.md#c5--gate-testsrequired_call_sitesrs)).

## Part B — visual pass (needs eyes at a display, or `visual-pass`)

1. **Showcase**: `mise run showcase` (or the `visual-pass` skill). The
   *Required field* entry shows the legend `* required`, labels ending in `*` resting and floating,
   and the error state with "This field is required" in light and dark. In greyscale
   (screenshot desaturated) the marker and message still read (SC-005).
2. **Add worktree**: open it. `* required` above the first field; `Type*` and `Name*` marked,
   `Ticket` not; no error shown. Focus Name, Tab away: Name shows the error, Type does not.
   Press Create (enabled): Type and Name both in error, nothing created. Type a name: its error
   clears with the caret where it was. Switch to *Existing branch*: `Branch*` marked.
3. **Run in parallel**: `Prompt*`, `Type*`, `Name*`, `Base branch*`; Start runs enabled; pressing
   it empty puts all four in error.
4. **Rename project / worktree**: `Project name*` / `Worktree name*`; clear it and press Rename:
   the field shows the error on itself, no separate line under it.
5. **Settings**: Terminal (`Scrollback lines*`), Environment (`Timeout*`, `Long-task threshold*`,
   `Warn at*` while shown), GitHub (each `Label*`), Jira (no markers until something is typed with
   the switch on), Daemons (new daemon: `Name*`; SSH: `Host*` appears and disappears with the
   choice; container: `Container name*`, `Image reference*`, `Image file*` only for a file source;
   `Port` unmarked), Appearance (no legend).
6. **Two windows**: an error in one window's add-worktree form does not appear in the other's.

Record the pass in `specs/937-required-field-markers/visual-pass.md`.
