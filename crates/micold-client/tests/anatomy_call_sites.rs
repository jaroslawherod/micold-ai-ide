//! Every anatomy figure reaches a component (feature 018 — FR-025 – FR-032, contract §7).
//!
//! # What this closes
//!
//! `micold-core/tests/tokens_anatomy.rs` compares each constant against the contract. That proves
//! the number was *transcribed*, not that it was *applied* — and the difference is where every
//! anatomy defect this feature has had came from:
//!
//! - **BUG-002.** `button::MIN_TOUCH_TARGET` was 48.0 and always had been. It reached
//!   `icon_button.rs`, was written into the builder, and was discarded one line later by a
//!   `center_x(Fill)` that sets the length as well as aligning.
//! - **T097.** `density::BUTTON_BASE` was 40.0, referenced by no call site. `button.rs` documented
//!   the opposite — "Feature 018 assigns each variant a height from the density scale" — and laid a
//!   filled button out at 30dp.
//! - **T098.** `density::MENU_ITEM_BASE` was 48.0, applied only by `typeahead.rs`. Menu items were
//!   `spacing::SM` around one `label_large` line, 36dp against §7.5's 48.
//!
//! In all three the constants gate was green, because in all three the constant was right. What was
//! missing was the *binding*: a reference from the component the contract row names. So that is what
//! this asserts, and it is the cheapest of the anatomy checks — it reads source text and needs no
//! renderer, no layout pass and no fixture.
//!
//! `anatomy_size.rs` is the complement, not a substitute. It measures a laid-out box, so it covers
//! the figures that *are* a box: nine sizes, out of §7's forty-six. Paddings, gaps, icon sizes and
//! outline widths are not a component's bounds and no layout assertion reaches them; a binding is
//! the only evidence available that they arrived anywhere at all.
//!
//! # The precedent
//!
//! `type_role_call_sites.rs` is this rule for typography, and its own doc comment records the hole
//! it had to close twice: feature 003's *named* `type_scale::BODY` sailed past a scan looking for
//! numbers, and eleven components sat outside the type system while a test called "no call site
//! states a raw text size" stayed green. Anatomy is the same shape with the arrow reversed. There,
//! the danger is a call site reaching around the scale; here, it is a scale that reaches no call
//! site. Both are a token system that is only a system for as long as everything is joined to it.
//!
//! # What counts as a binding
//!
//! A reference to the constant from the rendering layer, outside comments and outside the in-crate
//! gates. A gate naming `anatomy::chip::HEIGHT` is comparing a measurement against it, which is the
//! opposite of a component being built from it — counting those would let this gate be satisfied by
//! the very tests it exists to backstop.
//!
//! # Not being bound is allowed; being unbound *and* unrecorded is not
//!
//! §7 contains figures this application deliberately does not apply — FR-042 through FR-046 are the
//! accepted fidelity gaps, and `dialog::MIN_WIDTH` is one of them. [`RECORDED`] is where those go,
//! each with the reason, and [`a_recorded_gap_that_became_bound_is_stale`] fails if one is quietly
//! joined up later. The rule is not "every figure is applied". It is T097's wording: a figure must
//! not sit in the state where the requirement is *neither met nor waived*.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use micold_core::tokens::anatomy;

/// §7's heights live in `density`, which owns the base heights and the density-step arithmetic, so
/// `anatomy::ALL` does not list them (see `anatomy.rs`'s "Heights are not here"). They are anatomy
/// figures all the same, and two of the three defects above were exactly these — so the set this
/// gate walks is `anatomy::ALL` plus the four component bases.
///
/// `STEPS`, `STEP_DP`, `STANDARD` and `DENSE` are the scale's own machinery rather than a component
/// measurement, and are not listed.
///
/// `LIST_ROW_BASE` was `gap::WAIVED` here until BUG-005: §7.2's row height reached no component, on
/// a recorded decision that the contract disagreed with itself. It did not — the height had been
/// hung on each row's indent spacer, which is void at depth 0, so it applied to nested rows alone
/// and the arithmetic that condemned it was arithmetic about the wrong node. Both bases are bound
/// now, by `tree_view.rs`, and the waiver is gone rather than reworded.
const DENSITY_BASES: &[&str] = &[
    "density::LIST_ROW_BASE",
    "density::LIST_ROW_TWO_LINE_BASE",
    "density::MENU_ITEM_BASE",
    "density::TEXT_FIELD_BASE",
    "density::BUTTON_BASE",
];

/// Why a figure is allowed to reach no component. Every [`RECORDED`] entry carries one, because the
/// four are not equally acceptable and a single undifferentiated allowlist would hide that.
mod gap {
    /// Waived in `spec.md`'s accepted-fidelity-gaps list. The requirement is met by being declined.
    pub const WAIVED: &str = "waived";
    /// Applied, but through a different token system — so the figure is a second spelling of a
    /// number something else already owns, and binding it would create the two-sources-of-truth
    /// `anatomy.rs`'s own header warns about ("exactly how the sidebar came to be 28dp while the
    /// contract said 36dp").
    pub const CARRIED_ELSEWHERE: &str = "carried elsewhere";
    /// Describes a slot or variant this application has not built. Nothing is wrong and nothing is
    /// deviating; there is simply no call site for it to reach yet.
    pub const NOT_BUILT: &str = "not built";
    /// **A live deviation.** The component exists, the contract states a number, and the component
    /// uses a different one — or the same one under a name that will not follow when §7 changes.
    /// These are the entries that are not meant to stay, and the only reason they are here rather
    /// than fixed is that fixing them is a change to what the application looks like.
    pub const UNAPPLIED: &str = "UNAPPLIED";
}

/// Figures §7 states and this application does not apply, each with its category and its reason.
///
/// This is not a place to park a figure that has not been wired up yet — every entry states which
/// of the four kinds above it is, and [`a_recorded_gap_that_became_bound_is_stale`] fails if one is
/// quietly joined up later. The rule the gate enforces is T097's wording: a figure must not sit in
/// the state where the requirement is *neither met nor waived*. Ten of these are `UNAPPLIED` and are
/// therefore only half-way out of that state — recorded, not met — and are tracked in
/// `specs/018-material3-visual-system/tasks.md` under T113.
///
/// It has already shrunk once without anyone editing it deliberately. BUG-003's T103 and T105
/// applied §7.5's `ITEM_PADDING`, `VERTICAL_PADDING` and `ITEM_ICON` while this gate was in review,
/// and [`a_recorded_gap_that_became_bound_is_stale`] is what reported it — three entries describing
/// a state that had stopped being true.
const RECORDED: &[(&str, &str, &str)] = &[
    // -- Waived in spec.md ------------------------------------------------------------------
    (
        "anatomy::dialog::MIN_WIDTH",
        gap::WAIVED,
        "FR-046 — §7.4's row says \"recorded, not applied\" in the contract itself. `surface.rs` \
         applies MAX_WIDTH and names MIN_WIDTH in the comment saying why it does not",
    ),
    // -- Carried by another token system ----------------------------------------------------
    // Glyphs are font glyphs, sized by the type scale: `Glyph::new(icon, TypeRole::…, roles)`.
    // A dp figure for an icon is therefore a number the type scale already owns.
    (
        "anatomy::button::LEADING_ICON",
        gap::CARRIED_ELSEWHERE,
        "18dp — a leading icon inside a labelled button is a `Glyph` sized by its `TypeRole`",
    ),
    (
        "anatomy::button::ICON_BUTTON_GLYPH",
        gap::CARRIED_ELSEWHERE,
        "24dp — `icon_button.rs` sizes its glyph `TypeRole::Body.size()`, overridable by `.size(role)`",
    ),
    (
        "anatomy::chip::ICON",
        gap::CARRIED_ELSEWHERE,
        "18dp — see `chip::PADDING_WITH_ICON`: the slot does not exist, and would be a `Glyph` if it did",
    ),
    (
        "anatomy::dialog::ICON",
        gap::CARRIED_ELSEWHERE,
        "24dp — the optional centred icon above a dialog title, a `Glyph` where any dialog poses one",
    ),
    (
        "anatomy::text_field::TRAILING_ICON",
        gap::CARRIED_ELSEWHERE,
        "24dp — `text_field.rs`'s trailing slot takes an `IconButton`, which sizes its own glyph",
    ),
    (
        "anatomy::app_bar::ICON_TARGET",
        gap::CARRIED_ELSEWHERE,
        "48dp — §7.1 restates §7.3's target for the bar. The bar's actions are `IconButton`s and \
         `button::MIN_TOUCH_TARGET` is the constant they apply, so the figure is honoured and this \
         is its second spelling. BUG-002 is what it looks like when the honouring stops",
    ),
    // -- Describes something not built ------------------------------------------------------
    (
        "anatomy::chip::PADDING_WITH_ICON",
        gap::NOT_BUILT,
        "8dp — `ToggleChip` has no leading-icon slot at all, so there is no chip with an icon whose \
         padding could differ",
    ),
    (
        "anatomy::menu::DIVIDER",
        gap::NOT_BUILT,
        "1dp — no menu in the application groups its items, so none draws a divider",
    ),
    (
        "anatomy::app_bar::LEADING_ICON_PADDING",
        gap::NOT_BUILT,
        "4dp — `Toolbar` has no leading-icon slot; its leading element is the title, which takes \
         `app_bar::PADDING`",
    ),
    // -- Live deviations (T113) -------------------------------------------------------------
    // Two shapes, and the second is the more dangerous: a component whose number is right today
    // under a name that will not follow when §7 is re-valued. That is `type_scale::BODY` again.
    (
        "anatomy::button::PADDING_FILLED",
        gap::UNAPPLIED,
        "§7.3 states 24; `Button` sets no padding, so a filled button takes iced's DEFAULT_PADDING \
         of 10",
    ),
    (
        "anatomy::button::PADDING_OUTLINED",
        gap::UNAPPLIED,
        "§7.3 states 24; as PADDING_FILLED — iced's 10",
    ),
    (
        "anatomy::button::PADDING_TEXT",
        gap::UNAPPLIED,
        "§7.3 states 12; iced's 10, or `spacing::SM`/`XS` where a call site passes `.padding(..)`",
    ),
    (
        "anatomy::button::PADDING_ICON",
        gap::UNAPPLIED,
        "§7.3 states 8; `icon_button.rs` defaults to `spacing::XS` (4)",
    ),
    (
        "anatomy::button::OUTLINE",
        gap::UNAPPLIED,
        "§7.3 states 1; `style.rs` writes the literal 1.0 — the right number, joined to nothing",
    ),
    (
        "anatomy::chip::OUTLINE",
        gap::UNAPPLIED,
        "§7.6 states 1; `toggle_chip.rs` writes the literal 1.0 — the right number, joined to nothing",
    ),
    (
        "anatomy::dialog::PADDING",
        gap::UNAPPLIED,
        "§7.4 states 24; all seven dialogs pass `spacing::LG`, which is 24 — right number, joined \
         to nothing",
    ),
    (
        "anatomy::dialog::TITLE_TO_BODY",
        gap::UNAPPLIED,
        "§7.4 states 16; the dialog column's `spacing::MD` is 16 — right number, joined to nothing",
    ),
    (
        "anatomy::dialog::BODY_TO_ACTIONS",
        gap::UNAPPLIED,
        "§7.4 states 24, and this one is a real difference: the action row is pushed into the same \
         column as the body, so it takes that column's `spacing::MD` (16). §7.4 makes it wider than \
         TITLE_TO_BODY on purpose, \"so the actions read as a separate region rather than as more \
         body\" — which is exactly what 16 does not do",
    ),
    (
        "anatomy::dialog::ACTION_GAP",
        gap::UNAPPLIED,
        "§7.4 states 8; the action row's `spacing::SM` is 8 — right number, joined to nothing",
    ),
];

/// The categories an entry may claim.
const CATEGORIES: &[&str] = &[
    gap::WAIVED,
    gap::CARRIED_ELSEWHERE,
    gap::NOT_BUILT,
    gap::UNAPPLIED,
];

/// Everything that renders: the shared library, the feature modules, and the showcase — which is
/// held to the same rules as the application it demonstrates (feature 020, FR-023).
fn rendering_dirs() -> Vec<PathBuf> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    vec![src.join("ui"), src.join("showcase")]
}

fn material_mod_rs() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/material/mod.rs");
    fs::read_to_string(path).expect("read material/mod.rs")
}

/// The in-crate gates, read off their own declarations rather than listed here.
///
/// `material` is `pub(crate)`, so several component checks cannot live in `tests/` at all and sit in
/// `src/ui/material/` behind `#[cfg(test)] mod ...;`. That attribute is the crate's own statement of
/// which files are tests, so taking the set from it means a gate added later is excluded the day it
/// lands rather than the day someone remembers this list.
fn in_crate_gates() -> BTreeSet<String> {
    let src = material_mod_rs();
    let mut out = BTreeSet::new();
    let mut armed = false;
    for line in src.lines() {
        let line = line.trim();
        if line.starts_with("#[cfg(test)]") {
            armed = true;
            continue;
        }
        if line.starts_with("///") || line.starts_with("//") || line.is_empty() {
            continue;
        }
        if armed {
            if let Some(name) = line
                .strip_prefix("mod ")
                .or_else(|| line.strip_prefix("pub mod "))
                .and_then(|rest| rest.strip_suffix(';'))
            {
                // Keyed the way `sources()` keys a path: relative to `src`, not to `material`.
                out.insert(format!("ui/material/{name}.rs"));
            }
            armed = false;
        }
    }
    out
}

/// The rendering layer's sources, as `(path relative to src, code)`.
fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path
                    .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                out.push((name, fs::read_to_string(&path).expect("read source")));
            }
        }
    }
    let mut out = Vec::new();
    for dir in rendering_dirs() {
        walk(&dir, &mut out);
    }
    out.sort();
    out
}

/// Strips comments, so a constant *discussed* in prose is not mistaken for one that is used.
///
/// This matters more here than in `type_role_call_sites.rs`: the files that lost a figure are
/// precisely the ones whose comments now explain the loss at length, and `button.rs` names
/// `density::BUTTON_BASE` twice in the comment above the line that finally applies it.
fn code_only(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_block = false;
    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            continue;
        }
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                in_block = true;
            }
            _ => out.push(c),
        }
    }
    out
}

/// Byte offset of the first *inline* `#[cfg(test)]` module, if any.
///
/// The attribute has two jobs in this crate and only one of them starts a test body. `material/
/// mod.rs` uses it fourteen times to declare the out-of-line gates (`#[cfg(test)] mod
/// anatomy_size;`), and truncating there would drop the whole `pub use` block below it — real code,
/// silently unscanned. So a declaration ending in `;` is stepped over and only a block is a
/// boundary. The convention that there is at most one such block per file, at the bottom, is what
/// [`the_test_module_convention_this_relies_on_holds`] pins.
fn inline_test_module(code: &str) -> Option<usize> {
    let mut offset = 0usize;
    let mut lines = code.lines().peekable();
    while let Some(line) = lines.next() {
        let start = offset;
        offset += line.len() + 1;
        if !line.trim_start().starts_with("#[cfg(test)]") {
            continue;
        }
        // The declaration this attribute applies to, skipping doc comments and blank lines.
        let declaration = lines
            .clone()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with("//"));
        match declaration {
            Some(d) if d.ends_with(';') => continue,
            _ => return Some(start),
        }
    }
    None
}

/// Drops a file's trailing `#[cfg(test)] mod tests { .. }` block.
///
/// A constant named only by a unit test is not a component using it, the same way an in-crate gate
/// naming one is not.
fn production_code(src: &str) -> String {
    let code = code_only(src);
    match inline_test_module(&code) {
        Some(i) => code[..i].to_string(),
        None => code,
    }
}

/// Every anatomy figure, named as `anatomy::ALL` names it, plus the density bases.
fn figures() -> Vec<String> {
    anatomy::ALL
        .iter()
        .map(|(name, _)| format!("anatomy::{name}"))
        .chain(DENSITY_BASES.iter().map(|n| (*n).to_string()))
        .collect()
}

/// The rendering-layer files that name `figure`, gates and comments excluded.
fn bindings(figure: &str) -> Vec<String> {
    let gates = in_crate_gates();
    sources()
        .into_iter()
        .filter(|(path, _)| !gates.contains(path))
        .filter(|(_, src)| production_code(src).contains(figure))
        .map(|(path, _)| path)
        .collect()
}

fn is_recorded(figure: &str) -> bool {
    RECORDED.iter().any(|(name, _, _)| *name == figure)
}

#[test]
fn every_anatomy_figure_reaches_a_component_or_is_a_recorded_gap() {
    let unbound: Vec<String> = figures()
        .into_iter()
        .filter(|f| !is_recorded(f) && bindings(f).is_empty())
        .map(|f| format!("  {f}"))
        .collect();

    let count = unbound.len();
    let subject = if count == 1 {
        "one anatomy figure is".to_string()
    } else {
        format!("{count} anatomy figures are")
    };
    assert!(
        unbound.is_empty(),
        "{subject} stated in contract §7, transcribed into the token module, and reaches no \
         component:\n{}\n\nA constant nothing references is a requirement that is neither met nor \
         waived. `tokens_anatomy.rs` is green on it, because the number is right — that is what \
         BUG-002, T097 and T098 each turned out to be, and `anatomy_size.rs` cannot see it either \
         where the component still lays out at today's number under a name of its own. Apply the \
         figure at the component §7 names, or record it in `RECORDED` with its category and reason.",
        unbound.join("\n")
    );
}

/// A recorded gap that has since been applied is a stale exemption, and a stale exemption hides the
/// next regression at that figure. Same reason `type_role_call_sites.rs` re-checks the one file it
/// exempts.
#[test]
fn a_recorded_gap_that_became_bound_is_stale() {
    let stale: Vec<String> = RECORDED
        .iter()
        .filter(|(name, _, _)| !bindings(name).is_empty())
        .map(|(name, kind, why)| {
            format!(
                "  {name} [{kind}] — recorded as: {why}\n    now used by: {:?}",
                bindings(name)
            )
        })
        .collect();

    assert!(
        stale.is_empty(),
        "these figures are recorded as reaching no component, and reach one:\n{}\n\nRemove the \
         `RECORDED` entry so the binding is guarded from here on, and close whatever the entry \
         pointed at — an accepted-gap note, or T100.",
        stale.join("\n")
    );
}

/// `RECORDED` names figures that exist, exactly once each, in a category that exists.
///
/// A typo would silently exempt nothing and waive the figure it was meant to explain, which is the
/// failure this whole gate is about — a statement that looks like it binds something and does not.
#[test]
fn every_recorded_gap_names_a_real_figure_once_with_a_real_category() {
    let known: BTreeSet<String> = figures().into_iter().collect();
    let unknown: Vec<&str> = RECORDED
        .iter()
        .map(|(name, _, _)| *name)
        .filter(|name| !known.contains(*name))
        .collect();
    assert!(
        unknown.is_empty(),
        "`RECORDED` names figures that are not in `anatomy::ALL` or `DENSITY_BASES`: {unknown:?}"
    );

    let mut seen = BTreeSet::new();
    let duplicated: Vec<&str> = RECORDED
        .iter()
        .map(|(name, _, _)| *name)
        .filter(|name| !seen.insert(*name))
        .collect();
    assert!(
        duplicated.is_empty(),
        "`RECORDED` names these twice, so one reason is dead text: {duplicated:?}"
    );

    let miscategorised: Vec<&str> = RECORDED
        .iter()
        .filter(|(_, kind, why)| !CATEGORIES.contains(kind) || why.trim().is_empty())
        .map(|(name, _, _)| *name)
        .collect();
    assert!(
        miscategorised.is_empty(),
        "these entries have no known category or no reason: {miscategorised:?}"
    );
}

/// The count of live deviations, pinned.
///
/// `UNAPPLIED` is the one category that is not an explanation — it is a component using a number
/// other than the one §7 states, or the right number under a name that will not follow when §7 is
/// re-valued. Without this the list ratchets the wrong way: a new deviation could be added as an
/// entry rather than fixed, and the gate would stay green while §7 emptied out. Lowering this
/// number is the work; raising it needs a deliberate edit here and a reason in the commit.
///
/// Twelve when this gate was written, ten after BUG-003 applied §7.5's item and panel padding.
#[test]
fn the_live_deviations_are_the_ten_that_are_tracked() {
    let live: Vec<&str> = RECORDED
        .iter()
        .filter(|(_, kind, _)| *kind == gap::UNAPPLIED)
        .map(|(name, _, _)| *name)
        .collect();
    assert_eq!(
        live.len(),
        10,
        "the recorded live deviations are now {}, not 10:\n  {}\n\nIf one was fixed, drop its \
         `RECORDED` entry and lower this number. If one was added, it belongs in the code rather \
         than in this list — the point of the list is that it shrinks (T113).",
        live.len(),
        live.join("\n  ")
    );
}

/// The gates are excluded by reading their `#[cfg(test)]` declarations, so the exclusion is only
/// real if those names match the keys [`sources`] uses. They did not, once: the parse built
/// `material/anatomy_size.rs` while a source is keyed `ui/material/anatomy_size.rs`, so every gate
/// was scanned as production code and a figure referenced only by `anatomy_size.rs` read as bound.
/// The gate was green and guarding nothing — this file's own subject matter, one level up.
///
/// So this checks the names against the source list rather than against the parse, which is the
/// side that can be wrong.
#[test]
fn the_in_crate_gates_are_found_and_actually_excluded() {
    let gates = in_crate_gates();
    let known: BTreeSet<String> = sources().into_iter().map(|(path, _)| path).collect();

    for expected in [
        "ui/material/anatomy_size.rs",
        "ui/material/content_placement.rs",
        "ui/material/test_support.rs",
    ] {
        assert!(
            gates.contains(expected),
            "the `#[cfg(test)] mod` scan did not find `{expected}`; it found {gates:?}"
        );
    }

    let unmatched: Vec<&String> = gates.iter().filter(|g| !known.contains(*g)).collect();
    assert!(
        unmatched.is_empty(),
        "these gate names match no scanned source, so excluding them excludes nothing and their \
         references count as components using the figure: {unmatched:?}\n\nScanned keys look like \
         {:?}.",
        known.iter().take(3).collect::<Vec<_>>()
    );
}

/// Truncating at the first inline `#[cfg(test)]` module is only safe while each file has at most
/// one, at the bottom. If a file grows a second, or puts one in the middle, the scan starts dropping
/// production code and reporting figures as unbound that are not.
#[test]
fn the_test_module_convention_this_relies_on_holds() {
    let offenders: Vec<String> = sources()
        .into_iter()
        .filter_map(|(path, src)| {
            let code = code_only(&src);
            let tail = inline_test_module(&code).map(|i| &code[i..])?;
            // A second inline module inside the tail means the first was not the file's last word.
            inline_test_module(&tail[1..]).map(|_| format!("  {path}"))
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "these files have more than one inline `#[cfg(test)]` module, so truncating at the first \
         drops real code:\n{}",
        offenders.join("\n")
    );
}

/// The declaration-versus-block distinction the truncation turns on, both ways round.
#[test]
fn an_out_of_line_gate_declaration_is_not_a_truncation_point() {
    let declares = "#[cfg(test)]\nmod anatomy_size;\npub use button::Button;\n";
    assert!(production_code(declares).contains("pub use button::Button;"));

    let documented = "#[cfg(test)]\n/// A gate.\nmod content_placement;\npub use tag::Tag;\n";
    assert!(production_code(documented).contains("pub use tag::Tag;"));

    let opens = "pub use tag::Tag;\n#[cfg(test)]\nmod tests {\n    const X: f32 = 1.0;\n}\n";
    assert!(production_code(opens).contains("pub use tag::Tag;"));
    assert!(!production_code(opens).contains("const X"));
}

/// A scan that reads nothing passes trivially.
#[test]
fn the_scan_actually_reads_the_rendering_layer() {
    let sources = sources();
    assert!(
        sources.len() > 25,
        "found only {} sources — the check above would be near-vacuous",
        sources.len()
    );
    assert!(
        figures().len() == anatomy::ALL.len() + DENSITY_BASES.len(),
        "the figure set is not the whole contract"
    );
}

/// The synthetic Red: the binding scan finds a real use, and is not fooled by a mention in prose or
/// by a test naming the figure.
///
/// This is BUG-002's shape in miniature. `button.rs` names `density::BUTTON_BASE` in the comment
/// explaining that it used to reach nothing, directly above the line that applies it — so a scan
/// that read comments would have called it bound throughout the period it was not.
#[test]
fn the_scan_tells_a_use_from_a_mention() {
    let used = "            .height(Length::Fixed(density::BUTTON_BASE))";
    let mentioned = "        // applied nowhere: `density::BUTTON_BASE` was referenced by no call";
    let block = "/* density::BUTTON_BASE is 40dp */";
    let tested = "#[cfg(test)]\nmod tests {\n    use density::BUTTON_BASE;\n}";

    assert!(production_code(used).contains("density::BUTTON_BASE"));
    assert!(!production_code(mentioned).contains("density::BUTTON_BASE"));
    assert!(!production_code(block).contains("density::BUTTON_BASE"));
    assert!(!production_code(tested).contains("density::BUTTON_BASE"));
}
