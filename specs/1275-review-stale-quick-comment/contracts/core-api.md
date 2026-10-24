# Contract: core API (`micold-core`)

The pure rules M1 and M2 build test-first. Types: [data-model.md](../data-model.md) §1–§9. Each
numbered rule is at least one unit test. `row(path, kind, added, removed, identity)` stands for a
`ChangedFile`.

## Identity (FR-001a) — `review::changes`, `review::git`, `review::pr_diff`

- **I1** `parse_name_status` reads the old and new ids; an all-zero id is `None`.
- **I2** `assemble` sets `Sides { old, new }`: `Object(id)` for a present id, `Absent` for an
  added file's old side or a deleted file's new side.
- **I3** (real git) A committed range (`BaseToHead`) needs no `hash-object`: both sides are
  `Object`.
- **I4** (real git) A modified working-tree file edited again with the same `+/-` counts gets a
  different identity (US1 AS12); edited back to its content at the first read, the same identity
  (AS4). A stat-clean file and the same content hashed compare equal.
- **I5** (real git) An untracked file's new side is `Object` (hashed); a working-tree file over
  `MAX_VERSION_BYTES` is `Stat`, and touching it without changing bytes changes the identity (the
  accepted cost in Edge *Large files*).
- **I6** (real git) One `change_list` read runs at most one `hash-object` process.
- **I7** (real git) `change_list_of(.., &[p])` returns exactly `change_list`'s row for `p` (and
  for a rename, given `[new, from]`, the renamed row), or no row when `p` has no change.
- **I8** `split_pr_diff`: two sections differing in any byte have different `Patch` values; equal
  sections, equal values.

## Stale set (FR-001–FR-004) — `review::stale`

- **S1** Equal lists → empty set; `observe` returns `Same`.
- **S2** A row whose kind, `added`, `removed` or identity differs is in the set; a difference only
  in `content`, `large` or `origin` is not.
- **S3** A path in `latest` only (new file) and a path in `shown` only (reverted or removed) are in
  the set (AS5).
- **S4** A latest row `Renamed { from: X }` at path Y, with X in `shown` and Y not: the set is
  `{Y}`; X is not counted separately. If `shown` also lists Y, Y counts by S2 and X by S3 unless it
  is the `from` of a latest rename.
- **S5** Count = set size; repeated observations of the same latest change count each file once
  (FR-002, AS3).
- **S6** `Held::observe` with a latest whose row for P equals `shown`'s again drops P (FR-003,
  AS4); at an empty set it returns `Same`.
- **S7** `Held::apply` makes `shown` equal `latest` (FR-004).
- **S8** `Held::adopt(shown, P, fresh)` with `fresh` listing P: `shown`'s and `latest`'s P rows
  equal `fresh`'s, P leaves the set, every other path keeps its state (AS10).
- **S9** `Held::adopt` with `fresh` not listing P: `shown` unchanged (P's row stays), `latest`
  drops P, P stays in the set (AS11).
- **S10** `PrDiff`'s `Listing::put` copies the patch with the row and keeps GitHub's file order;
  `ChangeList`'s keeps path order.

## Drag (FR-015–FR-018) — `review::drag`

- **D1** `gutter_line(Some(o), Some(n)) == Some((New, n))`; `(Some(o), None) → (Old, o)`;
  `(None, None) → None`.
- **D2** Press at (New, 12), release with no `over`: `Click { New, 12 }` (FR-018).
- **D3** Press (New, 12), over rows with new 13…18, release: `Range { New, 12, 18 }`; `span` is
  12–18 (FR-015, FR-016).
- **D4** Press (New, 18), over 17…12: `Range { New, 18, 12 }`; span 12–18 (either direction).
- **D5** Press (New, 12), over 13, then a row with only an old number, then a row with no
  numbers, release: `Range { New, 12, 13 }`: such rows leave `last` at the last new-side line passed
  and never change the side (FR-017).
- **D6** Press (Old, 5), over context rows (both numbers): `last` follows their old numbers.
- **D7** Press (New, 12), over 13, back over 12, release: `Range { New, 12, 12 }` (moved, so it
  opens the composer; research R7).
- **D8** `span` after each `over` equals the lines from `pressed` to `last` inclusive.

## Chips (FR-023, FR-024) and the setting (FR-022) — `review::chips`, `settings`

- **K1** `CHIPS` labels are *Rename*, *Add test*, *Why?*, in that order, with the spec's texts.
- **K2** `insert("", chip) == chip.text`; `insert("  \n", chip) == chip.text`.
- **K3** `insert("Looks off", chip) == "Looks off\n" + chip.text`.
- **K4** `insert("Looks off\n", chip) == "Looks off\n" + chip.text` (no blank line added).
- **K5** `Settings::default().suggestion_chips == false`; a `settings.json` without the key reads
  false; true round-trips.
