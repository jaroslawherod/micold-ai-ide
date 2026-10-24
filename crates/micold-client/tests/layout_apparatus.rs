//! The measuring apparatus (feature 019, T004/T005/T008/T009).
//!
//! Four properties the layout snapshot rests on, each of which would otherwise be assumed:
//!
//! * it resolves layout with no display, no GPU and no window manager (FR-001);
//! * it measures text against a font we ship, not one the host happens to have (FR-006);
//! * it is deterministic (FR-005);
//! * it samples at a defined, reproducible point in every animation and scroll (FR-010, FR-011).
//!
//! The third and fourth come "for free" from building a fresh widget tree. Free is not the same as
//! checked — a later refactor that reuses a tree between walks would break both silently, and
//! nothing else in the suite would notice.

mod support;

use iced::advanced::layout;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::{Element, Size};

use micold_client::app::State;
use micold_client::features::connection::ConnectionStatus;
use micold_client::ui;
use micold_core::env_include::EnvIncludeOutcome;

use support::layout::{self as lay, Layer};

/// The application's main view for a given state, as the snapshot resolves it.
fn view(state: &State) -> Element<'_, micold_client::app::Message> {
    ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    )
}

/// The string every width assertion here is taken against.
const REFERENCE_STRING: &str = "Roboto layout parity 0123";

/// Shape one string at the renderer's default size and return its measured width.
fn measure(content: &str, font: iced::Font) -> f32 {
    lay::measure(content, font, lay::DEFAULT_TEXT_SIZE)
}

// --- T004 — headless construction -------------------------------------------------------------

/// The renderer must construct without a GPU, a window or a display server.
///
/// The backend hint is what makes this true rather than lucky: `iced_wgpu`'s `Headless::new`
/// returns `None` on its first line when the hint is not `"wgpu"`, before it builds a
/// `wgpu::Instance` or requests an adapter. Without the hint, this would probe the GPU and its
/// behaviour would depend on the machine.
#[test]
fn the_renderer_constructs_with_no_gpu_and_no_window() {
    let renderer = lay::renderer();
    assert_eq!(
        Headless::name(&renderer),
        "tiny-skia",
        "the snapshot must resolve on the CPU rasteriser; a wgpu renderer here means the backend \
         hint stopped working and CI is now depending on a GPU"
    );
}

/// A layout actually resolves, and is not a degenerate empty tree.
#[test]
fn a_real_view_resolves_to_a_non_trivial_tree() {
    let renderer = lay::renderer();
    let state = State::default();
    let records = lay::resolve(view(&state), &renderer);

    assert!(
        records.len() > 5,
        "expected a non-trivial layout tree, got {} records",
        records.len()
    );

    let root = &records[0];
    assert_eq!(
        root.path,
        Vec::<usize>::new(),
        "the first record is the root"
    );
    assert!(
        root.width > 0.0 && root.height > 0.0,
        "the root must have real geometry, got {}x{}",
        root.width,
        root.height
    );
}

// --- T005 — the font guard --------------------------------------------------------------------

/// The committed face is the one we think it is.
#[test]
fn the_reference_face_parses_as_roboto_regular() {
    let face = ttf_parser::Face::parse(lay::REFERENCE_FONT_BYTES, 0)
        .expect("the committed reference font must parse");

    // The face carries several name records; the Macintosh-platform ones do not decode through
    // `to_string()`, so take the first that does rather than the first that matches.
    let family = face
        .names()
        .into_iter()
        .filter(|n| n.name_id == ttf_parser::name_id::FAMILY)
        .find_map(|n| n.to_string())
        .expect("the face must carry a decodable family name");

    assert_eq!(family, lay::REFERENCE_FONT_FAMILY);
    assert_eq!(face.weight(), ttf_parser::Weight::Normal);
    assert!(!face.is_italic());
}

/// The icon face is the one we think it is, and it is actually loaded.
///
/// Added after CI caught what local runs could not: icons are glyphs, shaped and measured like any
/// other text, and the apparatus was loading only the Roboto faces. Every icon therefore resolved
/// through the host's fallback and was whatever width that machine happened to offer — on the
/// Ubuntu runner, 8.4px where 6.3px had been recorded, shifting every adjacent label by 2.1px.
///
/// Two halves, for the same reason the Roboto guard has two: parsing proves the committed file is
/// the right face, and measuring proves that face is what the renderer actually used. A glyph
/// resolved from a fallback would still measure *something*.
#[test]
fn the_icon_face_parses_and_is_the_face_that_measures() {
    let face = ttf_parser::Face::parse(lay::REFERENCE_ICON_FONT_BYTES, 0)
        .expect("the committed icon font must parse");

    let family = face
        .names()
        .into_iter()
        .filter(|n| n.name_id == ttf_parser::name_id::FAMILY)
        .find_map(|n| n.to_string())
        .expect("the icon face must carry a decodable family name");
    assert_eq!(family, lay::REFERENCE_ICON_FONT_FAMILY);

    let _ = lay::renderer();
    let icon_font = iced::Font::with_name(lay::REFERENCE_ICON_FONT_FAMILY);

    // A glyph the application actually draws. Its width must come from the committed face rather
    // than from a fallback, so compare against the face's own advance rather than a constant.
    const GLYPH: char = '\u{e5cd}'; // "close"
    let measured = measure(&GLYPH.to_string(), icon_font);
    let expected = face
        .glyph_index(GLYPH)
        .and_then(|id| face.glyph_hor_advance(id))
        .map(|adv| adv as f32 / face.units_per_em() as f32 * 16.0)
        .expect("the committed icon face must contain the glyph");

    assert!(
        (measured - expected).abs() <= 0.5,
        "the icon glyph measured {measured:.1}px but the committed face advances {expected:.1}px. \
         The renderer is shaping it with some other font — the icon face is not loaded, or a host \
         font has won the family lookup. Read a mass geometry difference as this, not as a layout \
         regression."
    );
}

/// `Font::MONOSPACE` measures with the committed monospace face, not the host's (feature 940).
///
/// The same class as the icon face: code lines resolved through the host's monospace family, and
/// CI's Ubuntu runner measured every one of them 0.3–1.1px wider than the machine that recorded
/// the fixture. Measured against the committed face's own advances, so a substitution fails here.
#[test]
fn the_monospace_face_parses_and_is_the_face_that_measures() {
    let face = ttf_parser::Face::parse(lay::REFERENCE_MONO_FONT_BYTES, 0)
        .expect("the shipped Fira Mono Regular must parse");
    let family = face
        .names()
        .into_iter()
        .filter(|n| n.name_id == ttf_parser::name_id::FAMILY)
        .find_map(|n| n.to_string())
        .expect("the shipped Fira Mono must carry a decodable family name");
    assert_eq!(
        family,
        lay::REFERENCE_MONO_FONT_FAMILY,
        "the shipped Fira Mono Regular must report the family the installer pins"
    );
    assert!(
        face.is_monospaced(),
        "the shipped Fira Mono must be fixed-pitch"
    );

    let _ = lay::renderer();

    // Widths alone cannot tell two 0.6 em faces apart (the host this was recorded on had one), so
    // the loop also asks the database which face `Family::Monospace` resolves to: the shipped
    // bytes, not a host file, even one of the same name.
    // Regular and Bold: each weight must resolve to the shipped bytes, and measure with that
    // face's own advances (SC-001). Every message names `Fira Mono` (FR-006).
    use iced::advanced::graphics::text::cosmic_text::fontdb;
    const PROBE: &str = "cargo test -- --nocapture";
    for (weight, db_weight, bytes, what) in [
        (
            iced::font::Weight::Normal,
            fontdb::Weight::NORMAL,
            ui::FIRA_MONO_REGULAR_BYTES,
            "Regular",
        ),
        (
            iced::font::Weight::Bold,
            fontdb::Weight::BOLD,
            ui::FIRA_MONO_BOLD_BYTES,
            "Bold",
        ),
    ] {
        let weight_font = iced::Font {
            weight,
            ..iced::Font::MONOSPACE
        };
        // Resolve under the lock, assert after releasing it: a panic while holding it would poison
        // the global font system for every other test in this binary.
        let resolved = {
            let mut fonts = iced::advanced::graphics::text::font_system()
                .write()
                .expect("the global font system lock was poisoned");
            let db = fonts.raw().db();
            db.query(&fontdb::Query {
                families: &[fontdb::Family::Monospace],
                weight: db_weight,
                ..fontdb::Query::default()
            })
            .map(|id| {
                let shipped = db.with_face_data(id, |data, _| data == bytes) == Some(true);
                let described = db.face(id).map_or_else(String::new, |info| {
                    format!("{:?} at weight {}", info.families, info.weight.0)
                });
                (shipped, described)
            })
        };
        let (is_shipped, described) =
            resolved.unwrap_or_else(|| panic!("Fira Mono {what}: Family::Monospace must resolve"));
        assert!(
            is_shipped,
            "Fira Mono {what}: Family::Monospace resolved to {described}, not the shipped face the \
             installer registers"
        );

        let face = ttf_parser::Face::parse(bytes, 0)
            .unwrap_or_else(|e| panic!("shipped Fira Mono {what} does not parse: {e}"));
        let measured = measure(PROBE, weight_font);
        let expected: f32 = PROBE
            .chars()
            .map(|ch| {
                let id = face
                    .glyph_index(ch)
                    .unwrap_or_else(|| panic!("Fira Mono {what} lacks a glyph for {ch:?}"));
                f32::from(
                    face.glyph_hor_advance(id)
                        .unwrap_or_else(|| panic!("Fira Mono {what} lacks an advance for {ch:?}")),
                ) * lay::DEFAULT_TEXT_SIZE
                    / f32::from(face.units_per_em())
            })
            .sum();

        assert!(
            (measured - expected).abs() <= 0.05,
            "Fira Mono {what}: Font::MONOSPACE measured {measured:.2}px but the face advances \
             {expected:.2}px. The renderer is shaping code with some other font — the face is not \
             loaded, the monospace family is not set, or a host font of the same name has won the \
             lookup."
        );
    }
}

/// There is exactly one copy of Fira Mono in the repository, in `assets/fonts/` (FR-005, SC-003).
///
/// A second copy (a test fixture, a showcase asset) is a second thing to keep in step with the
/// licence and provenance record, and the one the renderer loads could silently stop being the one
/// that ships.
#[test]
fn there_is_one_copy_of_fira_mono_in_the_repository() {
    use std::path::{Path, PathBuf};

    fn walk(dir: &Path, root: &Path, found: &mut Vec<PathBuf>) {
        for entry in
            std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        {
            let entry = entry.expect("read a directory entry");
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_owned();
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                if name.starts_with("target")
                    || name == ".git"
                    || rel == Path::new(".claude/worktrees")
                {
                    continue;
                }
                walk(&path, root, found);
            } else if name.starts_with("FiraMono")
                && (name.ends_with(".ttf") || name.ends_with(".otf"))
            {
                found.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    walk(&root, &root, &mut found);
    let strays: Vec<_> = found
        .iter()
        .filter(|p| !p.starts_with("assets/fonts"))
        .collect();
    assert!(
        strays.is_empty(),
        "Fira Mono must exist once, in assets/fonts/; also found: {strays:?}"
    );
    assert!(
        found.iter().any(|p| p.starts_with("assets/fonts")),
        "the shipped Fira Mono was not found in assets/fonts/ (walked {})",
        root.display()
    );
}

/// **The load-bearing half of the guard.**
///
/// The global font database still loads the host's fonts, so a machine with its own `Roboto`
/// installed can win the family-name lookup and shift every measurement at once. That failure would
/// otherwise present as a mass layout regression across every covered state — the most misleading
/// possible symptom.
///
/// Pinning one measurement of one string makes it name itself instead. If this fails and the
/// snapshot also fails, believe this one: it is a font problem, not a layout problem.
///
/// This risk survives feature 018. Roboto is a common system font name, so shipping it as the
/// application typeface does not retire this test.
#[test]
fn shaped_glyphs_match_the_committed_face_exactly() {
    let _ = lay::renderer(); // loads the reference face into the global font system
    let face = ttf_parser::Face::parse(lay::REFERENCE_FONT_BYTES, 0).unwrap();
    let upem = f32::from(face.units_per_em());

    for ch in ['R', 'o', 'M', 'i', '0'] {
        let glyph = face.glyph_index(ch).expect("the face must cover ASCII");
        let advance = f32::from(
            face.glyph_hor_advance(glyph)
                .expect("the face must carry advances"),
        );
        let expected = advance * lay::DEFAULT_TEXT_SIZE / upem;
        let measured = measure(&ch.to_string(), lay::reference_font());

        assert!(
            (measured - expected).abs() < 0.001,
            "{ch:?} shaped to {measured:.3}px but the committed face's own advance says \
             {expected:.3}px. The text being measured is not coming from the file in \
             tests/fixtures/ — a font of the same family name on this host has won the lookup."
        );
    }
}

/// And the pin is doing something: an unresolvable family falls back to the host's sans-serif,
/// which measures differently. If these ever agreed, the guard above would be vacuous.
#[test]
fn the_reference_font_differs_from_the_host_fallback() {
    let _ = lay::renderer();

    let pinned = measure(REFERENCE_STRING, lay::reference_font());
    let fallback = measure(REFERENCE_STRING, iced::Font::with_name("NoSuchFamilyXYZ"));

    assert!(
        (pinned - fallback).abs() > 1.0,
        "the pinned face and the host fallback measured the same ({pinned:.1}px). Either this \
         host's default sans-serif is Roboto, or the reference font is not being applied at all — \
         and in the second case the fixture is not portable."
    );
}

/// Byte-stability: the whole reference string measures to a pinned width.
///
/// Regenerate deliberately if and only if the reference font file changes; see
/// `tests/fixtures/FONT-PROVENANCE.md`. If this fails alongside the layout snapshot, believe this
/// one — it is a font problem, not a layout problem, and regenerating the fixture would bake the
/// wrong measurements in.
///
/// **182.2 to 182.1 on 2026-08-04**, when the gate stopped measuring against its own copy of
/// Roboto and started measuring against the faces the application ships (feature 018). The two
/// files were different builds with different bytes; a tenth of a pixel over this string is the
/// whole visible difference between them, which is precisely why this pin exists rather than a
/// trust that "it is Roboto either way".
#[test]
fn the_reference_string_measures_to_a_pinned_width() {
    let _ = lay::renderer();
    let measured = measure(REFERENCE_STRING, lay::reference_font());

    const EXPECTED_WIDTH: f32 = 182.1;
    assert!(
        (measured - EXPECTED_WIDTH).abs() <= 0.1,
        "the reference string measured {measured:.1}px, expected {EXPECTED_WIDTH:.1}px"
    );
}

// --- T008 — determinism -----------------------------------------------------------------------

/// Two walks of the same state must agree exactly (FR-005).
#[test]
fn two_walks_of_the_same_state_are_identical() {
    let renderer = lay::renderer();

    let first = lay::resolve(view(&State::default()), &renderer);
    let second = lay::resolve(view(&State::default()), &renderer);

    assert_eq!(
        first, second,
        "the same state resolved twice produced different geometry"
    );
    assert!(!first.is_empty());
}

/// And the text form must be identical too — normalisation must not introduce its own variance.
#[test]
fn the_rendered_form_is_identical_across_walks() {
    let renderer = lay::renderer();

    let render = |records: &[lay::LayoutRecord]| {
        records
            .iter()
            .map(lay::format_record)
            .collect::<Vec<_>>()
            .join("\n")
    };

    let a = render(&lay::resolve(view(&State::default()), &renderer));
    let b = render(&lay::resolve(view(&State::default()), &renderer));

    assert_eq!(a, b);
}

// --- T009 — the reproducible sampling point ---------------------------------------------------

/// A freshly built tree is the sampling point: every animation at rest, every scrollable at the
/// top (FR-010, FR-011, research R6/R7).
///
/// Asserted behaviourally rather than by inspecting widget state, which is private: laying out a
/// fresh tree twice from the same state must agree, and must also agree with a tree that has been
/// laid out repeatedly. If anything were mid-transition, the repeated pass would differ — that is
/// what "in flight" means.
#[test]
fn a_fresh_tree_samples_at_rest() {
    let renderer = lay::renderer();
    let state = State::default();

    let mut element = view(&state);
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, lay::WINDOW);

    let first = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let settled = lay::walk(layout::Layout::new(&first), Layer::Base);

    // Lay the same tree out repeatedly. Nothing advances a clock, so nothing may move.
    for pass in 1..=5 {
        let again = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        let records = lay::walk(layout::Layout::new(&again), Layer::Base);
        assert_eq!(
            settled, records,
            "geometry changed on repeat layout pass {pass} — something is animating, so the \
             sampling point is not reproducible"
        );
    }
}
