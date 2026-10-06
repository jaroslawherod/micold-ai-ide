//! The one mapping from a notification kind to its icon (feature 613, I2, FR-015).
//!
//! Render-free: Settings draws the icon in a checkbox row and the desktop backends read the same
//! mapping, so a kind can never wear a different glyph in two places.

use crate::icons::Icon;
use micold_core::attention::NotificationKind;

/// The icon for `kind`. One-to-one over [`NotificationKind::ALL`].
pub const fn icon(kind: NotificationKind) -> Icon {
    match kind {
        NotificationKind::NeedsPermission => Icon::NeedsPermission,
        NotificationKind::SessionError => Icon::SessionError,
        NotificationKind::LongTaskFinished => Icon::LongTaskFinished,
        NotificationKind::TurnFinished => Icon::TurnFinished,
    }
}
