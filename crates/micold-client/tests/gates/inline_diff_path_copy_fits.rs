//! An inline diff's path copy control fits beside the path at the narrowest pane (feature 1550,
//! T045, C-3, FR-017).
//!
//! The showcase's diffs (the MultiEdit's path is longer than the pane) are laid out in both
//! schemes at the narrowest pane a split leaves (`MIN_PANE_COLS` cells of 8 px). The control's
//! glyph is drawn inside the pane, and the path wraps before it: no line of the path reaches
//! under the control, and none is clipped.
//!
//! Compiled into the `layout_snapshot` binary beside `inline_diff_marker_fits`.

use crate::support::layout as lay;
use iced::{Element, Size};
use micold_client::icons::Icon;
use micold_client::showcase::sections::review::{inline_diff_sample, DiffPose};
use micold_core::pane_layout::MIN_PANE_COLS;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The narrowest pane: `MIN_PANE_COLS` terminal cells of 8 px, as `split_view` measures a pane.
const NARROWEST: Size = Size::new(MIN_PANE_COLS as f32 * 8.0, 4_000.0);

fn check<M>(what: &str, path: &str, element: Element<'_, M>) {
    let mut renderer = lay::renderer();
    let (painted, _) = lay::painted_text_settled_at(element, &mut renderer, NARROWEST);
    let glyph = Icon::Copy.glyph().to_string();
    let control = painted
        .iter()
        .find(|p| p.content == glyph)
        .unwrap_or_else(|| panic!("{what}: the copy glyph is painted: {painted:?}"));
    let control_end = control.origin.x + control.natural_width.min(control.allowed_width);
    assert!(
        control_end <= NARROWEST.width + TOLERANCE,
        "{what}: the control ends at {control_end}, past the pane's {}",
        NARROWEST.width
    );
    let shown = painted
        .iter()
        .find(|p| p.content == path)
        .unwrap_or_else(|| panic!("{what}: the path is painted: {painted:?}"));
    assert!(
        shown.natural_width <= shown.allowed_width + TOLERANCE,
        "{what}: the path is clipped, not wrapped: {shown:?}"
    );
    assert!(
        shown.origin.x + shown.allowed_width <= control.origin.x + TOLERANCE,
        "{what}: the path reaches under the control: {shown:?} {control:?}"
    );
}

#[test]
fn the_path_copy_control_fits_beside_the_path_at_the_narrowest_pane() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let roles = tokens::roles(scheme);
        for pose in [
            DiffPose::Edit,
            DiffPose::MultiEdit,
            DiffPose::Write,
            DiffPose::Large,
        ] {
            let what = format!("{scheme:?}, {pose:?}");
            let mut renderer = lay::renderer();
            let (painted, _) = lay::painted_text_settled_at(
                inline_diff_sample(pose, roles),
                &mut renderer,
                NARROWEST,
            );
            // The posed path is the first monospace text the diff paints.
            let path = painted
                .iter()
                .find(|p| p.content.contains('.') && p.content.contains('/'))
                .unwrap_or_else(|| panic!("{what}: a path is painted: {painted:?}"))
                .content
                .clone();
            check(&what, &path, inline_diff_sample(pose, roles));
        }
    }
}
