# Contract: stored pane layout

Field `pane_layout` in the project's state file (`StoredProjectState`), omitted when absent:

```json
{ "layout_version": 1,
  "focused": 3,
  "root": { "split": { "axis": "vertical", "ratio": 0.5,
      "first":  { "pane": { "id": 1, "terminal": { "session": "<SessionId>", "process": "primary" } } },
      "second": { "pane": { "id": 3, "terminal": { "session": "<SessionId>", "process": { "shell": 2 } } } } } } }
```

- `terminal` omitted/`null` = empty pane. `axis`: `vertical` = side by side, `horizontal` = stacked.
- Unknown extra fields are ignored. `layout_version` greater than 1, a missing version, malformed JSON for this field only, more than 6 leaves, a duplicated terminal, a `focused` that is not a leaf, or a ratio outside [0.05, 0.95] → the field is treated as absent (a warning is logged); the rest of the state file loads.
- Terminals that no longer resolve are kept in the file and shown as empty panes (re-resolved on every load).
- No `schema_version` bump (same rule as `last_session`).
