//! A component puts its content where its anatomy entry says, inside a height taller than that
//! content (feature 018, T086/T088 — FR-030a, SC-008a, BUG-001).
//!
//! Every component here fixes a height above the height of what it holds, which is the arrangement
//! that produced BUG-001: the chip (§7.6, a 20dp line box in 32dp) and the small app bar (§7.1, a
//! 28dp title in 64dp). The tag is here as the control case — it is content-sized, so it has no
//! slack and cannot misplace anything, and this is what would notice if it grew a height.
//!
//! In-crate rather than in `tests/`, for the reason `form_field_anatomy` and `style_snapshot`
//! record: `material` is `pub(crate)`, so a `ToggleChip` cannot be constructed from outside the
//! crate at all. tasks.md names `crates/micold-client/tests/gates/`, following `containment`; that
//! path can reach the layout apparatus but not the component, and the component is what is under
//! test.
//!
//! # This has to be measured in pixels, and that was learned here
//!
//! The first version of this gate read the laid-out tree, on the belief recorded in `BUG-001.md`
//! that the label *node* was sitting at the top of the pill with the slack beneath it. **It is
//! not.** A button lays its content out under `limits.height(Fixed(32))`, which sets the minimum
//! and the maximum together, so the label node is stretched to the full 32dp and its bounds are
//! the pill's bounds. Every band this could measure was 0dp — including for a deliberately
//! top-aligned control built to fail it. The geometry is *identical* either way; what differs is
//! where the glyphs are drawn inside a node that already fills the height, and the rendering
//! stack's text widget defaults that to the top edge.
//!
//! So this rasterises, as `ripple_clipping` does and for the same reason: the defect is in the
//! drawing, and a test of the layout would pass against the broken build. It renders the chip over
//! a backdrop of the chip's own fill colour — so the pill vanishes and the only ink is the label —
//! and compares the ink's vertical position against the *same string* drawn at the top of a box one
//! line high. Centring the 20dp line box in a 32dp pill moves the ink down by exactly half the
//! slack, 6dp, whatever the word is: glyph ink is not symmetric about its own line box (`feat` has
//! ascenders and no descenders), so a reference render is what makes the assertion glyph-independent
//! rather than a number tuned to one label.
//!
//! # Why the defect existed
//!
//! Fixing a component's height above its content's creates slack, and *something* has to decide
//! where the slack goes. Here two defaults decided it in the same direction and neither was chosen:
//! the button's limits stretch the label node to the full height, and the text widget draws its
//! glyphs at the top of whatever node it is given. Zero vertical padding on the button reads as
//! "size me by my content" — true while a chip had no height of its own, false the moment §7.6's
//! 32dp landed on it.

use std::borrow::Cow;

use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::advanced::{layout, mouse, renderer::Style, Layout, Renderer as _};
use iced::{Color, Element, Length, Size};
use micold_core::tokens::{self, anatomy, Roles};

use super::{style, Tag, Text, ToggleChip, TypeRole};
use crate::showcase::state::Message;

/// The label every render in this module draws. Ascenders, no descenders — deliberately *not*
/// vertically symmetric, so a gate that quietly assumed symmetric ink would be wrong about it.
const LABEL: &str = "feat";

/// One pixel. The two renders shape the same glyphs with the same face at the same size, so the
/// only difference between their ink rows is the offset under test; this allows for the subpixel
/// rounding of that offset and nothing else. The defect it separates from correct is 6dp.
const TOLERANCE: f32 = 1.0;

/// How far a channel must move from the backdrop before a pixel counts as ink. Well above the
/// rasteriser's noise, well below a glyph's antialiased edge.
const INK: u8 = 24;

fn roles() -> Roles {
    tokens::roles(micold_core::theme::ColorScheme::Light)
}

/// The rows of `pixels` that carry ink, as `(first, last)`, or `None` if nothing was drawn.
///
/// `rows` bounds the search to the component's own box. It is the whole buffer for a chip; the app
/// bar draws a hairline separator beneath itself, which is ink, and is not part of the 64dp the
/// title is placed within.
fn ink_rows(
    pixels: &[u8],
    size: (u32, u32),
    backdrop: Color,
    rows: std::ops::Range<u32>,
) -> Option<(u32, u32)> {
    let (width, _) = size;
    let channel = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let back = [
        channel(backdrop.r),
        channel(backdrop.g),
        channel(backdrop.b),
    ];

    let mut first = None;
    let mut last = None;
    for y in rows {
        let inked = (0..width).any(|x| {
            let i = ((y * width + x) * 4) as usize;
            (0..3).any(|c| pixels[i + c].abs_diff(back[c]) > INK)
        });
        if inked {
            first.get_or_insert(y);
            last = Some(y);
        }
    }
    first.zip(last)
}

/// Lay `element` out inside `bounds`, draw it over `backdrop`, and return the buffer with the size
/// it was drawn at.
///
/// `text_color` is the inherited foreground — what a `Text` with no colour of its own draws in. The
/// chip supplies its own through its button style and ignores it; the reference render needs it.
fn render(
    element: Element<'_, Message>,
    bounds: Size,
    backdrop: Color,
    text_color: Color,
) -> (Vec<u8>, (u32, u32)) {
    let mut element = element;
    let mut renderer = super::test_support::renderer();
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);

    let size = node.bounds().size();
    let viewport = iced::Rectangle::with_size(size);
    renderer.reset(viewport);
    element.as_widget().draw(
        &tree,
        &mut renderer,
        &iced::Theme::Light,
        &Style { text_color },
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );

    let pixels = (size.width.ceil() as u32, size.height.ceil() as u32);
    (
        renderer.screenshot(Size::new(pixels.0, pixels.1), 1.0, backdrop),
        pixels,
    )
}

/// Where the content's ink sits, and how tall the thing drawing it is: `(first row, last row,
/// height)`. `box_height`, when given, is the component's own box within a taller buffer.
fn ink_of(
    element: Element<'_, Message>,
    bounds: Size,
    backdrop: Color,
    on: Color,
    box_height: Option<f32>,
) -> (u32, u32, u32) {
    let (pixels, size) = render(element, bounds, backdrop, on);
    let height = box_height.map(|h| h as u32).unwrap_or(size.1);
    let (first, last) = ink_rows(&pixels, size, backdrop, 0..height)
        .expect("nothing was drawn, so there is no content to place");
    (first, last, height)
}

/// The same label drawn at the top of a box exactly one line high — the position the ink takes when
/// there is no slack at all, and therefore the baseline every offset below is measured from.
fn reference_ink(role: TypeRole, backdrop: Color, on: Color) -> (u32, u32, u32) {
    // No `.tint()`: the reference must draw in the *inherited* foreground, which is the colour the
    // component's own style hands its label, so both renders shape the same ink.
    let label: Element<'_, Message> = Text::new(Cow::Borrowed(LABEL), role, roles()).into();
    ink_of(
        label,
        Size::new(400.0, role.line_height_dp()),
        backdrop,
        on,
        None,
    )
}

/// How far the content's ink sits below where the same content sits with no slack, against how far
/// centring it would put it: `(shift, expected)`.
fn shift_against_centred(ink: (u32, u32, u32), reference: (u32, u32, u32)) -> ((f32, f32), f32) {
    let (first, last, height) = ink;
    let (ref_first, ref_last, ref_height) = reference;
    assert!(
        ref_height < height,
        "the reference line box ({ref_height}dp) is not shorter than the component ({height}dp), \
         so there is no slack and the assertion would be vacuous"
    );
    (
        (
            first as f32 - ref_first as f32,
            last as f32 - ref_last as f32,
        ),
        (height - ref_height) as f32 / 2.0,
    )
}

/// §7.6: the label is centred within the chip's height, not resting against one of its edges.
#[test]
fn a_toggle_chips_label_is_centred_within_the_pill() {
    let r = roles();
    // The chip's own fill as the backdrop, so the pill is invisible and every inked pixel is the
    // label. Drawn active for the same reason: an outlined chip would put its 1dp border into the
    // top and bottom rows, which is exactly where the measurement is taken.
    let backdrop = style::color(r.primary);
    let on = style::color(r.on_primary);

    let chip: Element<'_, Message> = ToggleChip::new(LABEL, Message::NoOp, r)
        .active(true)
        .accent(r.primary, r.on_primary)
        .into();
    let ink = ink_of(chip, Size::new(400.0, 400.0), backdrop, on, None);
    let reference = reference_ink(TypeRole::Action, backdrop, on);
    let (first, last, height) = ink;
    let (ref_first, ref_last, ref_height) = reference;

    assert_eq!(
        height,
        anatomy::chip::HEIGHT as u32,
        "the chip did not resolve at §7.6's height, so this is measuring the wrong thing"
    );

    // Centring the line box moves the ink down by half the slack — the same for both edges of the
    // ink, which is what makes this independent of the word's ascenders and descenders.
    let ((top_shift, bottom_shift), expected) = shift_against_centred(ink, reference);

    assert!(
        (top_shift - expected).abs() < TOLERANCE && (bottom_shift - expected).abs() < TOLERANCE,
        "a chip's label is not centred in the pill: its ink sits {top_shift:.1}dp below where the \
         same label sits at the top of a {ref_height}dp line box, and centring it in a {height}dp \
         chip would put it {expected:.1}dp below. Ink rows {first}–{last} of {height}; reference \
         {ref_first}–{ref_last} of {ref_height}. §7.6 states the label is centred within the height \
         (FR-030a): a fixed height and an unstated alignment leave the glyphs at the top edge.",
    );
}

/// The gate can fail. A centring assertion that cannot tell a top-aligned label from a centred one
/// is decoration, so this rebuilds the arrangement the chip had while BUG-001 was open — fixed
/// height, zero vertical padding, no alignment — and measures it the same way.
#[test]
fn a_top_aligned_label_is_reported_as_off_centre() {
    let r = roles();
    let backdrop = style::color(r.primary);
    let on = style::color(r.on_primary);
    let role = TypeRole::Action;

    let sabotage: Element<'_, Message> = iced::widget::button(
        iced::widget::text(LABEL)
            .size(role.size())
            .font(role.font())
            .line_height(role.line_height()),
    )
    .padding(iced::Padding {
        top: 0.0,
        bottom: 0.0,
        left: anatomy::chip::PADDING,
        right: anatomy::chip::PADDING,
    })
    .height(Length::Fixed(anatomy::chip::HEIGHT))
    .style(move |_: &iced::Theme, _| iced::widget::button::Style {
        background: Some(iced::Background::Color(backdrop)),
        text_color: on,
        ..Default::default()
    })
    .into();

    let ink = ink_of(sabotage, Size::new(400.0, 400.0), backdrop, on, None);
    let reference = reference_ink(TypeRole::Action, backdrop, on);
    let ((shift, _), expected) = shift_against_centred(ink, reference);

    assert!(
        (shift - expected).abs() >= TOLERANCE,
        "the deliberately top-aligned control measured as centred (ink {shift:.1}dp below the \
         reference, centring would be {expected:.1}dp), so the assertion above cannot fail and \
         proves nothing"
    );
    assert!(
        shift < expected,
        "the sabotage was supposed to leave its label high in the pill, and did not"
    );
}

/// §7.1's small app bar is the other component that fixes a height above its content's — a 28dp
/// `title_large` in 64dp — so it is the other place BUG-001 could have happened, and the sweep that
/// followed the fix (T088) had to end in a measurement rather than in a reading of the source.
///
/// It failed when first measured, which is the answer to whether the sweep was worth running. The
/// bar's `row!` states `align_y(Center)`, and that centres the row's children *against each other*
/// — not the row against a height imposed on it from outside — so with the container itself
/// unaligned the row went to the top and the title with it.
///
/// **Nothing looked wrong, and the reason is the interesting part.** The shipped bar holds a
/// full-height action, which stretches the row to the whole 64dp, and a row that already fills its
/// container cannot be misaligned by one. The application's own app bar was therefore correct *by
/// accident* — correct for exactly as long as some child happens to fill — and the layout fixture
/// is unchanged by the fix for the same reason. A bar with only a title sat 18dp high of centre.
/// That is why the class this module gates is "content whose placement is unstated" rather than
/// "content that visibly moved".
#[test]
fn the_app_bars_title_is_centred_within_the_bar() {
    let r = roles();
    // The bar's own surface as the backdrop, so only the title is ink. The resting bar is used
    // rather than the elevated one: an elevated bar draws a shadow into the rows the measurement
    // reads, while the resting bar's only extra ink is the hairline separator *below* its 64dp,
    // which is what `box_height` excludes.
    let backdrop = style::color(r.surface);
    let on = style::color(r.on_surface);

    let bar: Element<'_, Message> = super::Toolbar::new(LABEL, r).into();
    let ink = ink_of(
        bar,
        Size::new(800.0, 400.0),
        backdrop,
        on,
        Some(anatomy::app_bar::HEIGHT),
    );
    let reference = reference_ink(TypeRole::Title, backdrop, on);
    let ((top_shift, bottom_shift), expected) = shift_against_centred(ink, reference);

    assert!(
        (top_shift - expected).abs() < TOLERANCE && (bottom_shift - expected).abs() < TOLERANCE,
        "the app bar's title is not centred in its {height}dp: its ink sits {top_shift:.1}dp below \
         where the same title sits at the top of its line box, and centring would be \
         {expected:.1}dp (§7.1, FR-030a)",
        height = anatomy::app_bar::HEIGHT,
    );
}

/// The worktree tag has no height of its own, so it has no slack and no alignment question — which
/// is why BUG-001 spared it. Asserted rather than assumed: giving `Tag` a height later would
/// reintroduce exactly this defect, and this is what would notice.
#[test]
fn a_tag_is_sized_by_its_label_and_so_has_no_slack_to_misplace() {
    let r = roles();
    let tag: Element<'_, Message> = Tag::new(LABEL, r.primary).role(TypeRole::SidebarTag).into();

    let mut element = tag;
    let renderer = super::test_support::renderer();
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(400.0, 400.0)),
    );

    let height = node.bounds().height;
    let line = TypeRole::SidebarTag.line_height_dp();
    assert!(
        (height - line).abs() < TOLERANCE,
        "the tag resolved at {height:.1}dp against a {line:.1}dp line box — it has grown a height \
         of its own, and with it the alignment question FR-030a answers for the chip"
    );
}
