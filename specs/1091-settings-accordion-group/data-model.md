# Data Model: Settings accordions form a group

View state only; nothing persisted.

## `AccordionGroup<K: Copy + Eq>` (new, `ui/material/accordion_group.rs`)

| Field | Type | Meaning |
|---|---|---|
| `choice` | `Choice<K>` | `Default` (nothing pressed since the last reset), `Closed` (the user closed the open section), `Open(K)` |

API (all render-free): `new()`, `reset()`, `open_key(default: Option<K>) -> Option<K>`,
`is_open(key, default) -> bool`, `toggle(key, default)`.

### Transitions (`d` = the caller's default key)

| From | Press `k` | Result |
|---|---|---|
| any, with `open_key(d) == Some(k)` | `k` | `Closed` |
| any, with `open_key(d) != Some(k)` | `k` | `Open(k)` |
| any | `reset()` | `Default` |

Invariants: at most one key is open (SC-001); two groups share nothing (FR-003); `K` values of
another group are never compared.

## `features::daemons::State` (changed)

`flipped_sections: BTreeSet<Section>` becomes `sections: AccordionGroup<Section>`; `default` is
`Section::of(draft.runtime).first()`.

- `section_open(s)` = `sections.is_open(s, first) || held_open(s)`.
- `Msg::SectionToggled(s)`: ignored when `held_open(s)` (FR-006), else `sections.toggle(s, first)`.
- Reset (`sections.reset()`) at the three places `flipped_sections.clear()` runs today and on a
  `RuntimeChosen` that changes the kind (FR-007).
- Held sections are not recorded, so when their error clears they show the group's choice.

## Showcase state (changed)

`showcase::state::Showcase` gains `group: AccordionGroup<usize>`; `Message::GroupSectionToggled(usize)`
calls `group.toggle(i, None)`.
