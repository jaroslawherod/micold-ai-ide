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
