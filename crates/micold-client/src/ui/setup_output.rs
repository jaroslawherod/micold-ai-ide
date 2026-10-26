//! The setup output view (feature 1289, contract U4; FR-016, FR-021): a [`ScrollingDialog`] with
//! the command and the run's state above a [`VirtualRows`] list of the output lines, which builds
//! only the rows on screen, so 10,000 lines cost what a screenful does.
//!
//! The lines, the follow and the scroll position are `features::setup`'s [`OutputView`]; this only
//! draws them. The list's height is fixed from the window's, so the dialog's own body never scrolls
//! and the action row stays in view at the smallest window (geometry gate G2).

use crate::app::{Message, State};
use crate::features::setup::{self, Msg, OutputView, ROW_HEIGHT};
use crate::ui::material::{dialog, Button, ScrollingDialog, Text, Tooltip, TypeRole, VirtualRows};
use iced::widget::{column, container, row, Space};
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Roles};

/// The output list's scrollable, named so the shell's scroll to the end finds it across frames.
pub static SETUP_OUTPUT_SCROLL_ID: std::sync::LazyLock<iced::advanced::widget::Id> =
    std::sync::LazyLock::new(|| iced::advanced::widget::Id::new("setup-output"));

/// The most lines of the command the header shows; its tooltip holds the rest (U4).
const COMMAND_LINES: f32 = 3.0;

/// What the window keeps for everything but the list: the dialog's padding, title and actions, the
/// modal's margin, and the header's command and state lines with their gaps.
const RESERVED: f32 = 300.0;

/// The list's height bounds: room for a few lines at the smallest window, and no taller than a
/// comfortable read on a large one.
const LIST_MIN: f32 = 120.0;
const LIST_MAX: f32 = 480.0;

/// This dialog's body, built from the state that opened it: `None` when no view is open, or the
/// run record it shows has gone.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let view = state.setup.view.as_ref()?;
    let (daemon, project, location) = &view.key;
    let summary = state.setup.summary(*daemon, project, location)?;
    let r = tokens::roles(scheme);
    let names = setup::names(state, project, location);
    let tag = setup::row_tag(summary, |secs| {
        micold_core::local_time::UtcOffset::local_at(i64::try_from(secs).unwrap_or(i64::MAX))
    });
    let (_, height) = state.window.window_size;
    let window = if height > 0 {
        f32::from(height)
    } else {
        crate::app::MIN_WINDOW_SIZE.height
    };

    let text = summary.run.command().as_str().to_owned();
    let line = TypeRole::Body.line_height_dp();
    let command = container(
        Text::new(text.clone(), TypeRole::Body, r)
            .monospace()
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
    )
    .width(Length::Fill)
    .max_height(COMMAND_LINES * line)
    .clip(true);
    let body = column![
        Tooltip::new(command, text, r),
        Text::new(tag.tooltip, TypeRole::Caption, r).muted(),
        container(lines(view, r))
            .width(Length::Fill)
            .height(Length::Fixed((window - RESERVED).clamp(LIST_MIN, LIST_MAX))),
    ]
    .spacing(spacing::SM);

    let actions = dialog::actions(row![
        Button::outlined("Copy output", r).on_press(Message::Setup(Msg::CopyRequested)),
        Button::filled("Close", r).on_press(Message::Setup(Msg::ViewClosed)),
    ]);
    let title = format!("Setup \u{2014} {}", names.location);
    Some(
        ScrollingDialog::new(title, body, actions, r)
            .max_height(window)
            .into(),
    )
}

/// The output lines (U4): the kept lines in monospace, under a row counting the earlier lines not
/// kept, or *No output yet.* when there is nothing.
fn lines(view: &OutputView, r: Roles) -> Element<'_, Message> {
    if view.rows() == 0 {
        return Text::new("No output yet.", TypeRole::Body, r)
            .muted()
            .into();
    }
    let dropped = view.dropped();
    let lead = usize::from(dropped > 0);
    VirtualRows::new(
        view.rows(),
        ROW_HEIGHT,
        move |index| {
            let content: Element<'_, Message> = match index.checked_sub(lead) {
                None => Text::new(
                    format!("{dropped} earlier lines not kept"),
                    TypeRole::Body,
                    r,
                )
                .muted()
                .wrapping(iced::widget::text::Wrapping::None)
                .into(),
                Some(at) => match view.lines.get(at) {
                    Some(text) => Text::new(text.as_str(), TypeRole::Body, r)
                        .monospace()
                        .wrapping(iced::widget::text::Wrapping::None)
                        .into(),
                    None => Space::new().into(),
                },
            };
            container(content)
                .width(Length::Fill)
                .height(Length::Fixed(ROW_HEIGHT))
                .clip(true)
                .into()
        },
        r,
    )
    .offset(view.offset)
    .viewport(view.viewport)
    .id(SETUP_OUTPUT_SCROLL_ID.clone())
    .on_scroll(|offset, viewport| Message::Setup(Msg::Scrolled { offset, viewport }))
    .into()
}
