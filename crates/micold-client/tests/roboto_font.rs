//! The shipped typeface is real, and it is the two weights the type scale needs
//! (feature 018, T013 — FR-008a, FR-009, SC-012).
//!
//! Feature 018 ships Roboto so the application looks the same on every platform rather than
//! inheriting whatever UI font the OS provides (FR-008). That guarantee rests entirely on the
//! binaries in `assets/fonts/` being what they claim, and a font asset is the easiest thing in a
//! repository to get quietly wrong: a truncated download still commits, a variable font shipped
//! where a static instance was intended still renders, and a file named `Roboto-Medium.ttf` that
//! actually carries weight 400 renders *almost* right — every label a shade too light, with nothing
//! to point at.
//!
//! So this parses the shipped bytes and asks them what they are. It needs no GUI: `ttf-parser`
//! reads the tables directly.

/// The two faces the Material 3 type scale uses. Weights 400 and 500 are the only ones it
/// specifies (contract §2.1), which is why exactly two static instances ship rather than a variable
/// font — the smallest binary that expresses every role faithfully.
const REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Regular.ttf");
const MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Medium.ttf");

/// The family name the GUI selects the font with. Must match `src/main.rs`.
const EXPECTED_FAMILY: &str = "Roboto";

fn face<'a>(bytes: &'a [u8], what: &str) -> ttf_parser::Face<'a> {
    ttf_parser::Face::parse(bytes, 0).unwrap_or_else(|e| panic!("{what} must parse as a font: {e}"))
}

/// The family a font is selected by.
///
/// The **typographic** family (name ID 16) when present, else the family (ID 1). That order is not
/// a preference — it is how font matching works, and it is the whole reason a "Roboto Medium" file
/// belongs to the Roboto family rather than to a family of its own.
///
/// By convention a weighted instance puts `Roboto Medium` in ID 1, for the benefit of old software
/// that can only express a family plus regular/bold, and keeps the real family `Roboto` in ID 16.
/// Matching on ID 1 alone would conclude the two shipped faces are unrelated typefaces.
fn family_of(face: &ttf_parser::Face<'_>) -> Option<String> {
    let named = |id| {
        face.names()
            .into_iter()
            .find(|n| n.name_id == id)
            .and_then(|n| n.to_string())
    };
    named(ttf_parser::name_id::TYPOGRAPHIC_FAMILY).or_else(|| named(ttf_parser::name_id::FAMILY))
}

#[test]
fn both_faces_parse() {
    face(REGULAR, "Roboto-Regular.ttf");
    face(MEDIUM, "Roboto-Medium.ttf");
}

/// The weights the roles resolve to. A `Medium` file carrying 400 is the failure this catches, and
/// it is invisible short of comparing screenshots letter by letter.
#[test]
fn the_two_faces_report_the_weights_the_type_scale_asks_for() {
    assert_eq!(
        face(REGULAR, "Regular").weight().to_number(),
        400,
        "Roboto-Regular must report weight 400 — it backs every display, headline and body role"
    );
    assert_eq!(
        face(MEDIUM, "Medium").weight().to_number(),
        500,
        "Roboto-Medium must report weight 500 — it backs the title and label roles, and a file that \
         reports 400 renders every one of them a shade too light with nothing to point at"
    );
}

/// Both advertise the family the GUI pins, or `.default_font(Font::with_name(\"Roboto\"))` silently
/// falls back to the platform font and FR-008's whole guarantee is void — while still looking
/// perfectly reasonable on the machine that built it.
#[test]
fn both_faces_advertise_the_family_the_gui_pins() {
    for (bytes, what) in [(REGULAR, "Regular"), (MEDIUM, "Medium")] {
        let f = face(bytes, what);
        assert_eq!(
            family_of(&f).as_deref(),
            Some(EXPECTED_FAMILY),
            "Roboto-{what} advertises a different family name; the GUI selects fonts by name, so a \
             mismatch falls back to the platform font without erroring"
        );
    }
}

/// Static instances, not variable fonts. A variable font still renders, but at its default weight
/// for every role — so the 400/500 distinction the scale is built on silently disappears.
#[test]
fn both_faces_are_static_instances() {
    for (bytes, what) in [(REGULAR, "Regular"), (MEDIUM, "Medium")] {
        let f = face(bytes, what);
        assert!(
            !f.is_variable(),
            "Roboto-{what} is a variable font. It would render, but every role would take its \
             default weight and the type scale's 400/500 distinction would vanish (contract §2.1 \
             ships two static instances deliberately)"
        );
    }
}

/// Enough coverage for the interface's **own** text. A subset font that dropped Latin-1 would show
/// tofu in any label with an accented character.
///
/// Deliberately not a claim that Roboto covers everything on screen. It does not — it has no CJK
/// and no arrows — and it does not need to: a worktree named in Japanese is *user data*, and FR-013
/// says such text falls back to a font that does cover it rather than rendering boxes. This asserts
/// the part that must come from Roboto itself, which is the part a bad vendoring would break.
#[test]
fn the_faces_cover_the_interfaces_own_text() {
    // Latin, digits and the punctuation the UI composes labels and paths from, plus the handful of
    // symbols that appear in its own strings (an em dash, a copyright sign, the comparison and
    // status marks). Every one verified present in the shipped file.
    const REQUIRED: &str = "AZaz09 .,:;/-_()[]{}'\"!?@#%&*+=<>|~`^$É—©×≈≥○●";
    for (bytes, what) in [(REGULAR, "Regular"), (MEDIUM, "Medium")] {
        let f = face(bytes, what);
        for ch in REQUIRED.chars() {
            assert!(
                f.glyph_index(ch).is_some(),
                "Roboto-{what} has no glyph for {ch:?} (U+{:04X}) — the interface's own text would \
                 render as tofu",
                ch as u32
            );
        }
    }
}

/// The two files are genuinely different. Copying one over the other is a plausible slip during
/// vendoring, and every other assertion here would still pass on the copy that was *not* checked.
#[test]
fn the_two_faces_are_not_the_same_file() {
    assert_ne!(
        REGULAR, MEDIUM,
        "the two shipped faces are byte-identical — one was copied over the other during vendoring"
    );
}
