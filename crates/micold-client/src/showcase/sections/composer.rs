//! The structured session composer's shared parts (feature 941, T054): `AttachmentChip`,
//! `QueueRow` and `BusyIndicator`, and `TextArea`'s height limit (posed in `review::text_area`
//! beside the area's other states, from [`text_area_sample`]).
//!
//! The samples are public so `tests/gates/composer_parts_fit.rs` lays out exactly what the gallery
//! poses.

use iced::widget::image::Handle;
use iced::widget::{container, text_editor};
use iced::{Element, Length};
use micold_core::tokens::Roles;
use std::sync::LazyLock;

use crate::showcase::catalogue::Layout;
use crate::showcase::gallery::{arrange, posed};
use crate::showcase::state::{Message, Showcase};
use crate::ui::material::{AttachmentChip, BusyIndicator, QueueRow, TextArea};

/// The height the posed text area stops growing at: about eight lines.
pub const TEXT_AREA_LIMIT: f32 = 220.0;

/// The width the composer's parts are posed at: a narrow pane's.
const WIDTH: f32 = 520.0;

/// The busy pose's words.
pub const BUSY_LABEL: &str = "Claude is answering…";

/// The idle pose's words.
pub const IDLE_LABEL: &str = "Ready";

/// The posed picture's file name.
pub const ATTACHMENT_NAME: &str = "screenshot.png";

/// A long file name, cut by the chip.
const LONG_ATTACHMENT_NAME: &str = "Screenshot 2026-10-22 at 14.03.51 from the build failure.png";

/// A queued message longer than its row, over two lines.
pub const QUEUED_TEXT: &str = "Then run the whole test suite again, and if anything still fails, \
     read the failure before changing the code\nand tell me what you found";

/// The posed preview: the application's icon, as the window holds a dropped picture's bytes.
static PREVIEW: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_bytes(&include_bytes!("../../../../../assets/icon/png/icon-64.png")[..])
});

/// A text area over `content`, limited to [`TEXT_AREA_LIMIT`], at the posed width.
pub fn text_area_sample(content: &text_editor::Content, roles: Roles) -> Element<'_, Message> {
    container(
        TextArea::new(content, roles)
            .placeholder("Message Claude")
            .max_height(TEXT_AREA_LIMIT)
            .on_action(|_| Message::NoOp),
    )
    .width(Length::Fixed(WIDTH))
    .into()
}

/// A busy indicator, busy or idle.
pub fn busy_indicator_sample<'a>(busy: bool, roles: Roles) -> Element<'a, Message> {
    BusyIndicator::new(if busy { BUSY_LABEL } else { IDLE_LABEL }, roles)
        .busy(busy)
        .into()
}

/// The second queued message, with three pictures and every action.
pub fn queue_row_sample<'a>(roles: Roles) -> Element<'a, Message> {
    container(
        QueueRow::new(2, QUEUED_TEXT, roles)
            .pictures(3)
            .on_edit(Message::NoOp)
            .on_remove(Message::NoOp)
            .on_move_to_composer(Some(Message::NoOp)),
    )
    .width(Length::Fixed(WIDTH))
    .into()
}

/// A chip for [`ATTACHMENT_NAME`] with its remove control.
pub fn attachment_chip_sample<'a>(roles: Roles) -> Element<'a, Message> {
    AttachmentChip::new(ATTACHMENT_NAME, PREVIEW.clone(), roles)
        .on_remove(Message::NoOp)
        .into()
}

/// `AttachmentChip` — a dropped picture in the draft: its preview, its name, and the remove
/// control (hover it for its label); then a name too long for the chip, cut.
pub fn attachment_chip<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "a picture with its preview",
                attachment_chip_sample(roles),
                roles,
            ),
            posed(
                "a long file name",
                AttachmentChip::new(LONG_ATTACHMENT_NAME, PREVIEW.clone(), roles)
                    .on_remove(Message::NoOp),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `QueueRow` — the first queued message with its three actions; a long two-line message with
/// pictures, cut to one line; and a row without Move to composer, as while the draft holds text.
pub fn queue_row<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let sized = |row: QueueRow<'a, Message>| container(row).width(Length::Fixed(WIDTH));
    arrange(
        vec![
            posed(
                "first, with its actions",
                sized(
                    QueueRow::new(1, "Now add the tests", roles)
                        .on_edit(Message::NoOp)
                        .on_remove(Message::NoOp)
                        .on_move_to_composer(Some(Message::NoOp)),
                ),
                roles,
            ),
            posed("a long text with pictures", queue_row_sample(roles), roles),
            posed(
                "without Move to composer",
                sized(
                    QueueRow::new(3, "And update the guide", roles)
                        .on_edit(Message::NoOp)
                        .on_remove(Message::NoOp)
                        .on_move_to_composer(None),
                ),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}

/// `BusyIndicator` — busy, its ring turning beside its words, and idle, its words alone where the
/// ring was.
pub fn busy_indicator<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed("busy", busy_indicator_sample(true, roles), roles),
            posed("idle", busy_indicator_sample(false, roles), roles),
        ],
        Layout::Inline,
    )
}
