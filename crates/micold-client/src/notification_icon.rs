//! The one mapping from a notification kind to its icon (feature 613, I2, FR-015), and that icon
//! drawn for the operating system's notification (I3, I4, research R7).
//!
//! Render-free: Settings draws the icon in a checkbox row and the desktop backends read the same
//! mapping, so a kind can never wear a different glyph in two places. For the system, [`render`]
//! draws the same glyph of the bundled Material Symbols font (outline by `ttf-parser`, fill by
//! `tiny-skia`) in white on a rounded tile of the kind's colour. The tile is what holds 3:1 against
//! a light and a dark notification alike (FR-017); a glyph in one colour could not.

use crate::icons::Icon;
use crate::ui::material::glyph::MATERIAL_SYMBOLS_BYTES;
use micold_core::attention::NotificationKind;
use std::path::{Path, PathBuf};
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};

/// The icon for `kind`. One-to-one over [`NotificationKind::ALL`].
pub const fn icon(kind: NotificationKind) -> Icon {
    match kind {
        NotificationKind::NeedsPermission => Icon::NeedsPermission,
        NotificationKind::SessionError => Icon::SessionError,
        NotificationKind::LongTaskFinished => Icon::LongTaskFinished,
        NotificationKind::TurnFinished => Icon::TurnFinished,
    }
}

/// An image as straight (not premultiplied) RGBA, 8 bits a channel, row after row with no padding:
/// what the freedesktop `image-data` hint carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// The colour of `kind`'s tile. Each has a relative luminance in [0.10, 0.30], so it holds 3:1
/// against white and against black, and the white glyph holds 3:1 on it (I3). Taken from the
/// Material 3 tonal palettes at tone 40 or near it: amber, error red, green and primary blue.
pub const fn tile_colour(kind: NotificationKind) -> [u8; 3] {
    match kind {
        NotificationKind::NeedsPermission => [0x9A, 0x5B, 0x00],
        NotificationKind::SessionError => [0xB3, 0x26, 0x1E],
        NotificationKind::LongTaskFinished => [0x2E, 0x7D, 0x32],
        NotificationKind::TurnFinished => [0x0B, 0x57, 0xD0],
    }
}

/// The name of `kind`'s icon file, without its extension: the kind as the wire spells it.
pub const fn file_stem(kind: NotificationKind) -> &'static str {
    match kind {
        NotificationKind::NeedsPermission => "needs_permission",
        NotificationKind::SessionError => "session_error",
        NotificationKind::LongTaskFinished => "long_task_finished",
        NotificationKind::TurnFinished => "turn_finished",
    }
}

/// How much of the tile the glyph's longer side spans.
const GLYPH_SPAN: f32 = 0.625;
/// The tile's corner radius, as a share of its side.
const CORNER: f32 = 0.22;

/// `kind`'s icon, `px` by `px`: its glyph in white, centred, on a rounded square of its tile colour,
/// transparent outside the tile (I3).
pub fn render(kind: NotificationKind, px: u32) -> Rgba {
    let mut pixmap = canvas(px);
    let [r, g, b] = tile_colour(kind);
    let side = px as f32;
    if let Some(tile) = rounded_square(side, side * CORNER) {
        pixmap.fill_path(
            &tile,
            &solid(r, g, b),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    draw_glyph(&mut pixmap, kind, px, GLYPH_SPAN);
    straight(&pixmap)
}

/// `kind`'s glyph alone, in one colour, its ink box spanning a `px` by `px` square: `px * px`
/// pixels, row after row, each on when the glyph covers at least half of it. What the
/// distinctness gate compares (I4).
pub fn glyph_mask(kind: NotificationKind, px: u32) -> Vec<bool> {
    let mut pixmap = canvas(px);
    draw_glyph(&mut pixmap, kind, px, 1.0);
    pixmap.pixels().iter().map(|p| p.alpha() >= 128).collect()
}

/// `kind`'s icon at `px` encoded as PNG, for a notification facility that takes a file (I6).
pub fn png(kind: NotificationKind, px: u32) -> Vec<u8> {
    let Rgba { pixels, .. } = render(kind, px);
    let mut pixmap = canvas(px);
    for (to, from) in pixmap.pixels_mut().iter_mut().zip(pixels.chunks_exact(4)) {
        *to = tiny_skia::ColorU8::from_rgba(from[0], from[1], from[2], from[3]).premultiply();
    }
    // Encoding a well-formed pixmap into memory has no failure to report.
    pixmap.encode_png().unwrap_or_default()
}

/// The side, in pixels, of the icon files: large enough for the biggest place a system shows one.
pub const FILE_PX: u32 = 256;

/// The kind icons written as files, for a notification facility that takes a path (I6). A kind
/// with no file has no path, and its notification is shown without an icon (FR-016).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IconFiles {
    paths: Vec<(NotificationKind, PathBuf)>,
}

impl IconFiles {
    /// The file of `kind`'s icon, when it was written.
    pub fn path(&self, kind: NotificationKind) -> Option<&Path> {
        self.paths
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, path)| path.as_path())
    }
}

/// Writes every kind's icon, [`FILE_PX`] square, as `<dir>/notification-icons/<kind>.png`,
/// overwriting what is there (I6). The first failure is the error, and no paths are given.
pub fn write_files(dir: &Path) -> std::io::Result<IconFiles> {
    let icons = dir.join("notification-icons");
    std::fs::create_dir_all(&icons)?;
    let mut paths = Vec::with_capacity(NotificationKind::ALL.len());
    for kind in NotificationKind::ALL {
        let path = icons.join(format!("{}.png", file_stem(kind)));
        std::fs::write(&path, png(kind, FILE_PX))?;
        paths.push((kind, path));
    }
    Ok(IconFiles { paths })
}

fn canvas(px: u32) -> Pixmap {
    // `Pixmap::new` refuses only a zero side; the icon is never asked for at zero.
    Pixmap::new(px.max(1), px.max(1)).expect("a non-empty pixmap")
}

fn solid(r: u8, g: u8, b: u8) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, 255);
    paint.anti_alias = true;
    paint
}

fn rounded_square(side: f32, radius: f32) -> Option<tiny_skia::Path> {
    // A circle's quarter as one cubic: the control points sit this share of the radius in.
    const K: f32 = 0.552_284_8;
    let (s, r) = (side, radius);
    let mut path = PathBuilder::new();
    path.move_to(r, 0.0);
    path.line_to(s - r, 0.0);
    path.cubic_to(s - r + K * r, 0.0, s, r - K * r, s, r);
    path.line_to(s, s - r);
    path.cubic_to(s, s - r + K * r, s - r + K * r, s, s - r, s);
    path.line_to(r, s);
    path.cubic_to(r - K * r, s, 0.0, s - r + K * r, 0.0, s - r);
    path.line_to(0.0, r);
    path.cubic_to(0.0, r - K * r, r - K * r, 0.0, r, 0.0);
    path.close();
    path.finish()
}

/// Draws `kind`'s glyph in white on `pixmap`, its ink box centred and its longer side `span` of
/// the side.
fn draw_glyph(pixmap: &mut Pixmap, kind: NotificationKind, px: u32, span: f32) {
    let Ok(face) = ttf_parser::Face::parse(MATERIAL_SYMBOLS_BYTES, 0) else {
        return;
    };
    let Some(id) = face.glyph_index(icon(kind).glyph()) else {
        return;
    };
    let mut outline = Outline(PathBuilder::new());
    let Some(bounds) = face.outline_glyph(id, &mut outline) else {
        return;
    };
    let Some(path) = outline.0.finish() else {
        return;
    };
    let side = px as f32;
    let (width, height) = (
        f32::from(bounds.x_max) - f32::from(bounds.x_min),
        f32::from(bounds.y_max) - f32::from(bounds.y_min),
    );
    let scale = side * span / width.max(height).max(1.0);
    let centre_x = (f32::from(bounds.x_min) + f32::from(bounds.x_max)) / 2.0;
    let centre_y = (f32::from(bounds.y_min) + f32::from(bounds.y_max)) / 2.0;
    // Font units have y up; the pixmap has y down.
    let transform = Transform::from_row(
        scale,
        0.0,
        0.0,
        -scale,
        side / 2.0 - centre_x * scale,
        side / 2.0 + centre_y * scale,
    );
    pixmap.fill_path(
        &path,
        &solid(255, 255, 255),
        FillRule::Winding,
        transform,
        None,
    );
}

/// A glyph outline, as `ttf-parser` walks it, collected into a `tiny-skia` path.
struct Outline(PathBuilder);

impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, y1, x, y);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.cubic_to(x1, y1, x2, y2, x, y);
    }
    fn close(&mut self) {
        self.0.close();
    }
}

/// `pixmap`'s pixels as straight RGBA.
fn straight(pixmap: &Pixmap) -> Rgba {
    let pixels = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    Rgba {
        width: pixmap.width(),
        height: pixmap.height(),
        pixels,
    }
}
