//! The shipped typeface is real, and it is the weights the type scale and the Overview card need
//! (feature 018, T013 — FR-008a, FR-009, SC-012), each with its italic twin (feature 1259, FR-008).
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
/// specifies (contract §2.1), which is why static instances ship rather than a variable font — the
/// smallest binary that expresses every role faithfully.
const REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Regular.ttf");
const MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Medium.ttf");
/// The Overview card's semibold face (BUG-1298): its scoped roles ask for weight 600.
const SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Roboto-SemiBold.ttf");
/// The italic twin of each upright weight (feature 1259, FR-002): markdown emphasis draws in them,
/// so a role or a bold run that gains emphasis keeps its weight.
const ITALIC: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Italic.ttf");
const MEDIUM_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/Roboto-MediumItalic.ttf");
const SEMIBOLD_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/Roboto-SemiBoldItalic.ttf");

/// Every shipped Roboto face: its file name, its bytes, the weight it must report and whether it
/// must be italic.
const FACES: [(&str, &[u8], u16, bool); 6] = [
    ("Roboto-Regular.ttf", REGULAR, 400, false),
    ("Roboto-Medium.ttf", MEDIUM, 500, false),
    ("Roboto-SemiBold.ttf", SEMIBOLD, 600, false),
    ("Roboto-Italic.ttf", ITALIC, 400, true),
    ("Roboto-MediumItalic.ttf", MEDIUM_ITALIC, 500, true),
    ("Roboto-SemiBoldItalic.ttf", SEMIBOLD_ITALIC, 600, true),
];

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
/// Matching on ID 1 alone would conclude the shipped faces are unrelated typefaces.
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
fn the_faces_parse() {
    for (file, bytes, _, _) in FACES {
        face(bytes, file);
    }
}

/// The weights the roles resolve to, and the style emphasis selects. A `Medium` file carrying 400
/// is the failure this catches, and it is invisible short of comparing screenshots letter by
/// letter; an italic file that reports upright would never be chosen for emphasis.
#[test]
fn the_faces_report_the_weights_and_styles_the_type_scale_asks_for() {
    for (file, bytes, weight, italic) in FACES {
        let f = face(bytes, file);
        assert_eq!(
            f.weight().to_number(),
            weight,
            "{file} must report weight {weight} — a face that reports another weight renders every \
             role it backs a shade off, with nothing to point at"
        );
        assert_eq!(
            f.is_italic(),
            italic,
            "{file} must report is_italic() == {italic} — emphasis selects the bundled faces by \
             their style"
        );
    }
}

/// Every face advertises the family the GUI pins, or `.default_font(Font::with_name(\"Roboto\"))`
/// silently falls back to the platform font and FR-008's whole guarantee is void — while still
/// looking perfectly reasonable on the machine that built it.
#[test]
fn the_faces_advertise_the_family_the_gui_pins() {
    for (file, bytes, _, _) in FACES {
        let f = face(bytes, file);
        assert_eq!(
            family_of(&f).as_deref(),
            Some(EXPECTED_FAMILY),
            "{file} advertises a different family name; the GUI selects fonts by name, so a \
             mismatch falls back to the platform font without erroring"
        );
    }
}

/// Static instances, not variable fonts. A variable font still renders, but at its default weight
/// for every role — so the 400/500 distinction the scale is built on silently disappears.
#[test]
fn the_faces_are_static_instances() {
    for (file, bytes, _, _) in FACES {
        let f = face(bytes, file);
        assert!(
            !f.is_variable(),
            "{file} is a variable font. It would render, but every role would take its default \
             weight and the type scale's 400/500 distinction would vanish (contract §2.1 ships \
             static instances deliberately)"
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
    for (file, bytes, _, _) in FACES {
        let f = face(bytes, file);
        for ch in REQUIRED.chars() {
            assert!(
                f.glyph_index(ch).is_some(),
                "{file} has no glyph for {ch:?} (U+{:04X}) — the interface's own text would render \
                 as tofu",
                ch as u32
            );
        }
    }
}

/// The files are genuinely different. Copying one over another is a plausible slip during
/// vendoring, and every other assertion here would still pass on the copy that was *not* checked.
#[test]
fn the_faces_are_not_the_same_file() {
    for (i, (file_a, a, _, _)) in FACES.iter().enumerate() {
        for (file_b, b, _, _) in &FACES[i + 1..] {
            assert_ne!(
                a, b,
                "{file_a} and {file_b} are byte-identical — one was copied over the other during \
                 vendoring"
            );
        }
    }
}
