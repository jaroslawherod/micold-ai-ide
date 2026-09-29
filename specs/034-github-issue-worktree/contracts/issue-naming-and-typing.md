# Contract: naming and typing from an issue

**Feature**: [spec.md](../spec.md) · Research [R10](../research.md#r10--the-label-to-type-mapping-lives-in-settingsjson)–[R12](../research.md#r12--matching-issues-reuse-micold_coretypeahead-over-one-row-string)

## 1. `naming::name_from_title` (FR-010)

```rust
pub const ISSUE_NAME_SLUG_MAX: usize = 50;
pub fn name_from_title(title: &str) -> String;
```

| Title | Result | Why |
|---|---|---|
| `Crash when opening empty project` | same | slug 32 ≤ 50 |
| 12 words whose slug is 61 chars | longest whole-word prefix with slug ≤ 50 | word boundary |
| `Supercalifragilistic…` (one 70-char word) + more | first 50 chars of the full slug | first word alone too long |
| `🔥🔥 !!!` | `""` | slugs to nothing — "name required" applies |
| `  Fix  the   thing ` | `Fix the thing` | whitespace normalised to single spaces |

Property held by test: `slugify(&name_from_title(t)).len() <= 50` for every title in the corpus,
and `name_from_title(t) == t.split_whitespace().join(" ")` whenever that already fits.

Tests: `crates/micold-core/tests/naming_from_title.rs`.

## 2. `issue_types` — the mapping (FR-013–FR-021)

```rust
pub struct LabelTypeEntry { pub label: String, pub type_: ConventionalType }
pub fn default_mapping() -> Vec<LabelTypeEntry>;                      // bug→fix, enhancement→feat, documentation→docs
pub fn type_for_labels(mapping: &[LabelTypeEntry], labels: &[String]) -> Option<ConventionalType>;
pub fn validate_mapping(mapping: &[LabelTypeEntry]) -> Result<(), MappingError>;
pub struct MappingError { pub index: usize, pub kind: MappingErrorKind }
pub enum MappingErrorKind { Blank, Duplicate { of: usize } }
```

- `type_for_labels` walks **the mapping** in order and returns the first entry whose label matches
  any issue label, comparing `label.trim().to_lowercase()` on both sides (AS2, AS5). No match →
  `None` (AS3: the reducer clears the type).
- `validate_mapping` returns the first entry, in order, that is blank after trim, or that repeats an
  earlier entry's label ignoring case (`of` = the earlier index).
- Several labels may map to one type (spec Edge Cases) — never an error.

Tests: `crates/micold-core/tests/issue_types.rs` — AS1, AS2 (order wins, including when the issue
lists labels in the other order), AS5 (case), no match, empty mapping, blank, duplicate (`Bug` vs
`bug`), same type twice allowed.

## 3. Settings schema addition

```json
{
  "issue_label_types": [
    { "label": "bug", "type": "fix" },
    { "label": "enhancement", "type": "feat" },
    { "label": "documentation", "type": "docs" }
  ]
}
```

| Stored document | Loaded mapping |
|---|---|
| field absent (every file written before this feature) | `default_mapping()` (FR-021) |
| `[]` | `[]` — the user removed every entry |
| an entry with `"type": "bogus"` | that entry dropped; the rest kept; file **not** moved to `.bak` |
| an entry with a blank or duplicate label (hand edit) | kept as read; `type_for_labels` still first-wins; the Settings view shows it and refuses to save until fixed |

Written only by the client's Settings save, via `SettingsStore::update` (merge into the document on
disk). Tests: `crates/micold-core/tests/settings_issue_mapping.rs` — round trip, absent → default,
unknown token dropped, a daemon-side `update` that does not touch the field preserves it.

## 4. The pick (reducer, `features/worktree_form.rs`)

`Msg::IssuePicked { number, mapping }` on an open, editing, unprompted form whose `issues` is
`Loaded` and holds `number` (loaded or searched). Any other form state, or a `number` the listing
does not hold (a stale pick), is a no-op — tested in `issue_source_state.rs`:

| Field | After |
|---|---|
| `ticket` | `number.to_string()` — no `#` (FR-009); replaces whatever was there (FR-010a, AS8) |
| `name` | `name_from_title(title)` (FR-010); replaces (FR-010a) |
| `type_` | `type_for_labels(&mapping, &labels)` — `Some(t)` replaces, `None` clears (FR-013, FR-014) |
| `error` | `None` |
| `picked_issue` | `Some(number)` |
| `issue_list_open` | `false` |

The ticket, name and type remain ordinary editable fields afterwards (FR-011, FR-015). The shell
builds `mapping` from `caps.settings().load().settings.issue_label_types` at the moment it handles
the pick, or `default_mapping()` when there is no settings store (FR-014a).

Tests: `crates/micold-client/tests/issue_source_state.rs` — US1 AS4/AS5/AS8, US2 AS1–AS4/AS6,
FR-014a (a pick with mapping A then a pick with mapping B uses B; the first pick's type is not
recomputed).

## 5. Matching (FR-005)

`Issue::row_text` = `#{number} {title}` + (`  ·  {labels joined ", "}` if any). The picker calls
`typeahead::rank(&displayed, |i| i.row_text.as_str(), &Query::new(&issue_query))`, where
`displayed` = loaded issues followed by `searched`. Unmatched searched issues are not shown
(invariant 5); an empty query shows every loaded issue in GitHub's order plus no searched ones.

Tests: `crates/micold-client/tests/issue_source_state.rs` (number `42` finds `#42`; a label name
finds the issue; an unmatched searched issue is left out), `crates/micold-core/tests/github_search.rs`
(`merge_searched`; a searched issue matched only in its body is dropped by the rank) and a new release-build case in the
existing `crates/micold-core/tests/typeahead_budget.rs` (1,000 issue rows < 50 ms; SC-003).
