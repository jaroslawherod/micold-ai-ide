//! Font-asset integrity (SC-005): every `Icon` codepoint resolves to a real glyph in the
//! shipped font, and the font advertises the family name the GUI pins. Prevents "tofu" from
//! ever reaching the running UI and catches a font swap that drops a glyph or renames the
//! family. Parses the `.ttf` directly (via `ttf-parser`), so it needs no GUI/iced and runs
//! under `cargo test --no-default-features`.

use micold_client::icons::Icon;

const FONT: &[u8] = include_bytes!("../../../assets/fonts/MaterialSymbolsOutlined.ttf");

/// Must match the constant the GUI selects the font with (see `src/main.rs`).
const EXPECTED_FAMILY: &str = "Material Symbols Outlined";

#[test]
fn every_icon_codepoint_has_a_glyph() {
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    for &icon in Icon::ALL {
        assert!(
            face.glyph_index(icon.glyph()).is_some(),
            "{icon:?} (U+{:04X}) has no glyph in the shipped font — would render as tofu",
            icon.glyph() as u32
        );
    }
}

/// The pull request indicator's seven glyphs (feature 040, FR-009): each is in the shipped font, and
/// no two share a codepoint, so the shape alone tells them apart. `Icon::Close` is not one of them:
/// the failing mark is `cancel`, because `close` is already that variant's codepoint.
#[test]
fn the_seven_pull_request_glyphs_are_in_the_font_and_distinct() {
    let seven = [
        Icon::PrOpen,
        Icon::PrDraft,
        Icon::PrMerged,
        Icon::PrClosed,
        Icon::ChecksPassing,
        Icon::ChecksPending,
        Icon::ChecksFailing,
    ];
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    let mut glyphs = std::collections::BTreeSet::new();
    for icon in seven {
        let id = face
            .glyph_index(icon.glyph())
            .unwrap_or_else(|| panic!("{icon:?} has no glyph in the shipped font"));
        assert!(
            glyphs.insert(id),
            "{icon:?} draws the same glyph as another of the seven"
        );
    }
    assert_eq!(glyphs.len(), 7);
    assert!(
        !glyphs.contains(&face.glyph_index(Icon::Close.glyph()).unwrap()),
        "no pull request glyph may be the close glyph"
    );
    assert!(
        !glyphs.contains(&face.glyph_index(Icon::Rename.glyph()).unwrap()),
        "no pull request glyph may be the rename glyph"
    );
}

/// The Overview card menu's *Open* entry (feature 972, R10) is Material's `login`, is in the shipped
/// font, and is no other variant's glyph.
#[test]
fn open_is_login_and_distinct() {
    assert_eq!(Icon::Open.glyph(), '\u{ea77}');
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    let id = face
        .glyph_index(Icon::Open.glyph())
        .expect("login is in the shipped font");
    for &other in Icon::ALL.iter().filter(|i| **i != Icon::Open) {
        assert_ne!(
            face.glyph_index(other.glyph()),
            Some(id),
            "{other:?} draws the same glyph as Open"
        );
    }
}

/// **Open pull request**'s icon (feature 040, §4) is Material's `open_in_new`, is in the shipped
/// font, and is no other variant's glyph.
#[test]
fn open_in_browser_is_open_in_new_and_distinct() {
    assert_eq!(Icon::OpenInBrowser.glyph(), '\u{e89e}');
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    let id = face
        .glyph_index(Icon::OpenInBrowser.glyph())
        .expect("open_in_new is in the shipped font");
    for &other in Icon::ALL.iter().filter(|i| **i != Icon::OpenInBrowser) {
        assert_ne!(
            face.glyph_index(other.glyph()),
            Some(id),
            "{other:?} draws the same glyph as OpenInBrowser"
        );
    }
}

#[test]
fn font_advertises_the_pinned_family_name() {
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    let has_family = face.names().into_iter().any(|name| {
        name.name_id == ttf_parser::name_id::FAMILY
            && name.to_string().as_deref() == Some(EXPECTED_FAMILY)
    });
    assert!(
        has_family,
        "font must advertise family '{EXPECTED_FAMILY}' so the GUI can select it"
    );
}

/// The shipped font must be a **static** instance at the pinned axis values (weight 400 /
/// FILL 0 / GRAD 0 / opsz 24 — see `assets/fonts/PROVENANCE.md`), not the full upstream
/// variable font. Regenerating via the documented pipeline without the `varLib.instancer`
/// step (research R6) would still pass every other test here — both only check glyph/name
/// presence, which the variable font also satisfies — so this guards specifically against
/// that regression, which would otherwise ship a several-times-larger binary silently.
#[test]
fn font_is_a_static_instance_not_the_variable_font() {
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    assert!(
        !face.is_variable(),
        "shipped font must be a static instance (fonttools varLib.instancer), \
         not the upstream variable font — see assets/fonts/PROVENANCE.md"
    );
}

/// Loose upper bound on the shipped font's size: full upstream coverage as a static instance
/// is expected in the low single-digit-MB range (research R6). A much larger file would
/// indicate the variable font (or an unsubsetted/unstripped one) was shipped by mistake.
#[test]
fn font_size_is_within_the_expected_static_instance_range() {
    const MAX_BYTES: usize = 5 * 1024 * 1024;
    assert!(
        FONT.len() < MAX_BYTES,
        "shipped font is {} bytes, expected a static instance under {MAX_BYTES} bytes",
        FONT.len()
    );
}

/// Feature 613, I1: the four notification-kind icons (`pan_tool`, `error`, `task_alt`,
/// `chat_bubble`) are in `Icon::ALL` and each has a glyph in the bundled font.
#[test]
fn the_notification_kind_icons_have_glyphs_in_the_bundled_font() {
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    for (icon, cp) in [
        (Icon::NeedsPermission, '\u{e925}'),
        (Icon::SessionError, '\u{e000}'),
        (Icon::LongTaskFinished, '\u{e2e6}'),
        (Icon::TurnFinished, '\u{e0cb}'),
    ] {
        assert!(Icon::ALL.contains(&icon), "{icon:?} must be in Icon::ALL");
        assert_eq!(icon.glyph(), cp);
        assert!(face.glyph_index(cp).is_some(), "{icon:?} has no glyph");
    }
}

/// Feature 905 (FR-007): the three worktree-state glyphs each have a glyph in the bundled font, and
/// three different ones, so the shape alone tells the states apart.
#[test]
fn the_state_icons_have_three_distinct_glyphs_in_the_bundled_font() {
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    let glyphs: Vec<_> = [Icon::StateRunning, Icon::StateBlocked, Icon::StateIdle]
        .into_iter()
        .map(|icon| {
            assert!(Icon::ALL.contains(&icon), "{icon:?} must be in Icon::ALL");
            face.glyph_index(icon.glyph())
                .unwrap_or_else(|| panic!("{icon:?} has no glyph in the shipped font"))
        })
        .collect();
    assert!(
        glyphs[0] != glyphs[1] && glyphs[1] != glyphs[2] && glyphs[0] != glyphs[2],
        "three states, three glyphs: {glyphs:?}"
    );
}

/// Feature 813 (research R11): the Overview's ten icons each have a glyph in the bundled font.
#[test]
fn the_overview_icons_have_glyphs_in_the_bundled_font() {
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    for icon in [
        Icon::Overview,
        Icon::CardNeedsAction,
        Icon::CardNeedsAttention,
        Icon::CardDone,
        Icon::TypeFeature,
        Icon::TypeBugfix,
        Icon::TypeChore,
        Icon::TypeOther,
        Icon::TypeIncluded,
        Icon::TypeOrphan,
    ] {
        assert!(Icon::ALL.contains(&icon), "{icon:?} must be in Icon::ALL");
        assert!(
            face.glyph_index(icon.glyph()).is_some(),
            "{icon:?} has no glyph in the shipped font"
        );
    }
}

/// Feature 939 (research R11): the transcript's seven icons are in the bundled font and draw seven
/// different glyphs, so a card's kind and status are told apart by shape (FR-012).
#[test]
fn the_transcript_icons_have_seven_distinct_glyphs_in_the_bundled_font() {
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    let mut glyphs = std::collections::BTreeSet::new();
    for icon in [
        Icon::Thinking,
        Icon::ToolCall,
        Icon::ToolRunning,
        Icon::ToolDone,
        Icon::ToolFailed,
        Icon::ToolNoResult,
        Icon::JumpToLatest,
    ] {
        let id = face
            .glyph_index(icon.glyph())
            .unwrap_or_else(|| panic!("{icon:?} has no glyph in the shipped font"));
        assert!(
            glyphs.insert(id),
            "{icon:?} draws the same glyph as another of the seven"
        );
    }
}
