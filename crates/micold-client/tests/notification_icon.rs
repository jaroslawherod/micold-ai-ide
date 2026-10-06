//! Feature 613, I2 / FR-015: `notification_icon::icon` is the one kind→icon mapping, and it is
//! one-to-one. Pure — no iced.

use micold_client::icons::Icon;
use micold_client::notification_icon::icon;
use micold_core::attention::NotificationKind;

#[test]
fn each_kind_maps_to_its_icon() {
    assert_eq!(
        icon(NotificationKind::NeedsPermission),
        Icon::NeedsPermission
    );
    assert_eq!(icon(NotificationKind::SessionError), Icon::SessionError);
    assert_eq!(
        icon(NotificationKind::LongTaskFinished),
        Icon::LongTaskFinished
    );
    assert_eq!(icon(NotificationKind::TurnFinished), Icon::TurnFinished);
}

#[test]
fn the_mapping_is_one_to_one() {
    let icons: Vec<Icon> = NotificationKind::ALL.iter().map(|&k| icon(k)).collect();
    for (i, a) in icons.iter().enumerate() {
        for b in &icons[i + 1..] {
            assert_ne!(a, b, "two kinds share an icon");
            assert_ne!(a.glyph(), b.glyph());
        }
    }
}

// --- The rasterised icon handed to the OS (feature 613, T046: I3, I4, FR-017) -----------------

use micold_client::notification_icon::{glyph_mask, render, tile_colour};

/// WCAG relative luminance of an sRGB colour.
fn luminance([r, g, b]: [u8; 3]) -> f64 {
    let channel = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

fn contrast(a: f64, b: f64) -> f64 {
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn render_returns_a_square_rgba_image_of_the_size_asked_for() {
    for kind in NotificationKind::ALL {
        for px in [16, 64, 256] {
            let image = render(kind, px);
            assert_eq!((image.width, image.height), (px, px), "{kind:?} at {px}");
            assert_eq!(
                image.pixels.len(),
                (px * px * 4) as usize,
                "{kind:?} at {px}"
            );
        }
    }
}

#[test]
fn each_tile_holds_three_to_one_against_white_and_black_and_under_the_white_glyph() {
    for kind in NotificationKind::ALL {
        let tile = luminance(tile_colour(kind));
        assert!(
            (0.10..=0.30).contains(&tile),
            "{kind:?}: tile luminance {tile:.3} outside [0.10, 0.30]"
        );
        assert!(contrast(tile, 1.0) >= 3.0, "{kind:?} against white");
        assert!(contrast(tile, 0.0) >= 3.0, "{kind:?} against black");
    }
}

#[test]
fn the_tile_is_drawn_in_the_kinds_colour_and_the_glyph_in_white() {
    for kind in NotificationKind::ALL {
        let image = render(kind, 64);
        let at = |x: u32, y: u32| {
            let i = ((y * 64 + x) * 4) as usize;
            [
                image.pixels[i],
                image.pixels[i + 1],
                image.pixels[i + 2],
                image.pixels[i + 3],
            ]
        };
        // Above the glyph, inside the tile.
        let [r, g, b, a] = at(32, 3);
        assert_eq!(([r, g, b], a), (tile_colour(kind), 255), "{kind:?} tile");
        // The tile's rounded corner is transparent, so no square edge shows on the desktop.
        assert_eq!(at(0, 0)[3], 0, "{kind:?} corner");
        assert!(
            image.pixels.chunks(4).any(|p| p == [255, 255, 255, 255]),
            "{kind:?}: no white glyph pixel"
        );
    }
}

#[test]
fn the_glyph_is_not_empty_and_is_centred() {
    for kind in NotificationKind::ALL {
        let image = render(kind, 64);
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0, 0);
        for (i, p) in image.pixels.chunks(4).enumerate() {
            // Ink: the white glyph, well above any tile colour (luminance <= 0.30).
            if p[3] == 255 && p[0] > 200 && p[1] > 200 && p[2] > 200 {
                let (x, y) = (i as u32 % 64, i as u32 / 64);
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
        assert!(min_x <= max_x, "{kind:?}: no ink");
        let centre_x = f64::from(min_x + max_x + 1) / 2.0;
        let centre_y = f64::from(min_y + max_y + 1) / 2.0;
        assert!(
            (centre_x - 32.0).abs() <= 1.0 && (centre_y - 32.0).abs() <= 1.0,
            "{kind:?}: ink centred at ({centre_x}, {centre_y})"
        );
    }
}

#[test]
fn the_four_glyphs_in_one_colour_at_16_px_differ_pairwise_in_a_tenth_of_their_pixels() {
    let masks: Vec<(NotificationKind, Vec<bool>)> = NotificationKind::ALL
        .iter()
        .map(|&kind| (kind, glyph_mask(kind, 16)))
        .collect();
    for (kind, mask) in &masks {
        assert_eq!(mask.len(), 256, "{kind:?}");
        assert!(mask.iter().any(|&on| on), "{kind:?}: empty mask");
    }
    for (i, (a, mask_a)) in masks.iter().enumerate() {
        for (b, mask_b) in &masks[i + 1..] {
            let differ = mask_a.iter().zip(mask_b).filter(|(x, y)| x != y).count();
            assert!(
                differ * 10 >= 256,
                "{a:?} and {b:?} differ in only {differ} of 256 pixels"
            );
        }
    }
}

// --- The icon files for the facilities that take a file (feature 613, T047: I6) ---------------

use micold_client::notification_icon::{file_stem, write_files};

#[test]
fn the_four_icon_files_are_written_under_the_directory_as_256_px_pngs() {
    let dir = tempfile::tempdir().expect("temp dir");
    let files = write_files(dir.path()).expect("a writable directory");
    for kind in NotificationKind::ALL {
        let expected = dir
            .path()
            .join("notification-icons")
            .join(format!("{}.png", file_stem(kind)));
        assert_eq!(files.path(kind), Some(expected.as_path()), "{kind:?}");
        let bytes = std::fs::read(&expected).expect("the file is there");
        let image = tiny_skia::Pixmap::decode_png(&bytes).expect("a PNG");
        assert_eq!((image.width(), image.height()), (256, 256), "{kind:?}");
    }
}

#[test]
fn the_icon_files_overwrite_old_ones() {
    let dir = tempfile::tempdir().expect("temp dir");
    let icons = dir.path().join("notification-icons");
    std::fs::create_dir_all(&icons).expect("icons dir");
    let stale = icons.join(format!("{}.png", file_stem(NotificationKind::SessionError)));
    std::fs::write(&stale, b"not a png").expect("stale file");
    write_files(dir.path()).expect("a writable directory");
    let bytes = std::fs::read(&stale).expect("the file is there");
    assert!(
        tiny_skia::Pixmap::decode_png(&bytes).is_ok(),
        "still the stale bytes"
    );
}

#[test]
fn a_directory_that_cannot_be_written_is_an_error_and_no_paths() {
    // A file where the directory should be: refused even for root.
    let dir = tempfile::tempdir().expect("temp dir");
    let blocked = dir.path().join("blocked");
    std::fs::write(&blocked, b"").expect("a file");
    assert!(write_files(&blocked).is_err());
}

#[test]
fn the_icon_file_names_are_distinct_per_kind() {
    let stems: Vec<&str> = NotificationKind::ALL
        .iter()
        .map(|&k| file_stem(k))
        .collect();
    for (i, a) in stems.iter().enumerate() {
        assert!(!stems[i + 1..].contains(a), "{a} shared");
    }
}
