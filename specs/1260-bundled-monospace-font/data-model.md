# Data model: Bundled monospace typeface

No persisted state, no session state (Principle II: nothing per session; one process-wide font
registration). The entities are compile-time assets and one in-process registration.

## Bundled monospace face

| Field | Value | Validated by |
|---|---|---|
| Family (name ID 1) | `Fira Mono` (`MONO_FAMILY`) | bytes guard (FR-001) |
| Faces | Regular (400), Bold (700) | bytes guard (FR-001, FR-011) |
| Files | `assets/fonts/FiraMono-Regular.ttf`, `assets/fonts/FiraMono-Bold.ttf` | one-copy test (FR-005, SC-003) |
| Byte constants | `FIRA_MONO_REGULAR_BYTES`, `FIRA_MONO_BOLD_BYTES` (`include_bytes!`, `ui/material/mono.rs`) | layout guard measures these (FR-005) |
| Advance | 600 / 1000 units per em for every ASCII glyph | bytes guard; terminal cell test (FR-007) |
| Monospaced flag | `post.isFixedPitch` set | bytes guard |
| Licence | SIL OFL 1.1, `assets/fonts/LICENSE-FiraMono-OFL.txt` | packaging test (FR-008) |
| Provenance | upstream, commit, version 3.206, SHA-256 per file | `assets/fonts/PROVENANCE.md`, quickstart §A (FR-008) |

Relationships: belongs to the shipped font set with Roboto (`ROBOTO_*_BYTES`) and Material Symbols
(`MATERIAL_SYMBOLS_BYTES`); listed in the same `PROVENANCE.md` table.

## Monospace registration (in-process)

The state of iced's global `fontdb::Database` after `install_monospace` runs:

| Property | Rule |
|---|---|
| Bundled faces present | both bundled faces are in the database, `Source::Binary` |
| No rival | no face whose family list contains `Fira Mono` has any other source (`pin_monospace` removed them) |
| Monospace family | `db.family_name(&Family::Monospace) == "Fira Mono"` |
| Other faces untouched | every face of another family stays (fallback, FR-004) |

Transitions: `unregistered` (host default, `Noto Sans Mono`) → `installed` once, before the first
text is shaped. `install_monospace` is idempotent: a second call loads nothing new (iced dedupes
borrowed bytes by address) and removes nothing.

## Code-like text

Every text run whose font is `Font::MONOSPACE`, with any weight or style. Resolution:

| Request | Resolves to |
|---|---|
| `Font::MONOSPACE` | Fira Mono Regular |
| `Font::MONOSPACE` + `Weight::Bold` | Fira Mono Bold |
| `Font::MONOSPACE` + `Style::Italic` | Fira Mono (upright) of the requested weight |
| a character Fira Mono lacks | the host fallback face for that character |

Call sites (unchanged): `diff_view.rs` (diff and inline lines), `text.rs` (`Text::monospace`),
`link_text.rs` (inline code spans), `code_block.rs`, and `terminal.rs::cell_font` (FR-007).

## Font licence set (release bundles)

| File | Covers | Destination per bundle (research R5) |
|---|---|---|
| `LICENSE-FiraMono-OFL.txt` | Fira Mono | `fonts/` licence directory |
| `LICENSE-Roboto-OFL.txt` | Roboto | same |
| `LICENSE` | Material Symbols (Apache-2.0) | same |
| `PROVENANCE.md` | the file-to-licence map | same |
