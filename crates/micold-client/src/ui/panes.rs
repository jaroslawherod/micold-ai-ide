//! The terminal area as several panes (feature 484): one `TerminalPane` per leaf of the project's
//! `PaneLayout`, each under a header with the terminal's name, its state and the split buttons.
//!
//! Glue only: what a pane shows, which pane has focus and whether a split is allowed are decided by
//! `micold_core::pane_layout` and `shell::panes`. This builds the widgets from them.

use std::collections::HashMap;

use iced::widget::{column, container, mouse_area, row, stack, Space};
use iced::{Element, Length};
use micold_core::link::LinkContext;
use micold_core::pane_layout::{Axis, Pane, PaneLayout, MIN_PANE_COLS, MIN_PANE_ROWS};
use micold_core::protocol::messages::{SessionProcess, TerminalRef};
use micold_core::session::{SessionLifecycle, ShellLifecycle};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Roles};

use crate::app::{Message, State};
use crate::features::session::{Msg as SessionMsg, PaneMsg};
use crate::grid::GridCache;
use crate::icons::Icon;
use crate::ui::material::{
    pane_focus_mark as focus_mark, pane_notice_fill, Button, ButtonVariant, GridSizeReporter,
    IconButton, SplitView, TerminalPane, Text, Tooltip, TypeRole,
};
use crate::ui::terminal::TERM_FONT_SIZE;
use crate::ui::terminal::{empty_terminal_message, session_title, CellMetrics, TermPalette};

/// What the pane renderer reads besides `State`: the active project's layout and the grids.
pub struct PaneArea<'a> {
    /// The active project's layout.
    pub layout: &'a PaneLayout,
    /// Every streamed terminal's grid.
    pub grids: &'a HashMap<TerminalRef, GridCache>,
    /// Why the last split was refused, if it was (FR-001).
    pub refusal: Option<&'static str>,
}

/// Width of the accent strip that marks the focused pane's header (FR-010).
pub const FOCUS_STRIP: f32 = 2.0;

/// Height of the accent strip: a `Fill` height here would make the header take half the pane,
/// because a row with a `Fill` child is itself `Fill`.
const STRIP_HEIGHT: f32 = 24.0;

/// The smallest pane, in pixels: `MIN_PANE_COLS` x `MIN_PANE_ROWS` characters.
pub fn min_pane() -> (f32, f32) {
    let m = CellMetrics::new(TERM_FONT_SIZE);
    (
        f32::from(MIN_PANE_COLS) * m.width,
        f32::from(MIN_PANE_ROWS) * m.height,
    )
}

/// Every terminal of the active project, in tab order, with the name its pane header shows.
pub fn terminals(state: &State) -> Vec<(TerminalRef, String)> {
    let mut out = Vec::new();
    for s in state.active_sessions() {
        let title = s.label.display().to_string();
        out.push((
            TerminalRef {
                session: s.id,
                process: SessionProcess::Primary,
            },
            title.clone(),
        ));
        for i in &s.shells {
            out.push((
                TerminalRef {
                    session: s.id,
                    process: SessionProcess::Shell(i.id),
                },
                format!("{title} · terminal {}", i.id.0),
            ));
        }
    }
    out
}

/// The state word a pane header shows for its own terminal, so one that exited says so in its pane
/// while the others are unaffected.
pub fn terminal_status(state: &State, t: TerminalRef) -> String {
    let Some(s) = state.active_sessions().iter().find(|s| s.id == t.session) else {
        return "gone".to_string();
    };
    match t.process {
        SessionProcess::Primary => match &s.lifecycle {
            SessionLifecycle::Running => "running",
            SessionLifecycle::Starting => "starting…",
            SessionLifecycle::Restarting { .. } => "restarting…",
            SessionLifecycle::Failed { .. } => "failed",
            SessionLifecycle::Idle => "idle",
            SessionLifecycle::InterruptedResumable => "interrupted",
        },
        SessionProcess::Shell(id) => {
            match s.shells.iter().find(|i| i.id == id).map(|i| i.lifecycle) {
                Some(ShellLifecycle::Running) => "running",
                Some(ShellLifecycle::Starting) => "starting…",
                Some(ShellLifecycle::Exited) => "exited",
                Some(ShellLifecycle::NotStarted) | None => "idle",
            }
        }
    }
    .to_string()
}

fn split_button<'a>(pane: &Pane, axis: Axis, r: Roles) -> Element<'a, Message> {
    let (icon, tip) = match axis {
        Axis::Vertical => (Icon::SplitVertical, "Split into side-by-side panes"),
        Axis::Horizontal => (Icon::SplitHorizontal, "Split into stacked panes"),
    };
    Tooltip::new(
        IconButton::new(icon, r)
            .compact()
            .on_press(Message::Session(SessionMsg::Pane(PaneMsg::Split(
                pane.id(),
                axis,
            )))),
        tip,
        r,
    )
    .into()
}

fn close_button<'a>(pane: &Pane, r: Roles) -> Element<'a, Message> {
    Tooltip::new(
        IconButton::new(Icon::Close, r)
            .compact()
            .on_press(Message::Session(SessionMsg::Pane(PaneMsg::Close(
                pane.id(),
            )))),
        "Close this pane (the terminal keeps running)",
        r,
    )
    .into()
}

/// The two split buttons for `pane`, the pair the bottom bar and every header carry.
pub fn split_buttons<'a>(pane: &Pane, r: Roles) -> Element<'a, Message> {
    row![
        split_button(pane, Axis::Vertical, r),
        split_button(pane, Axis::Horizontal, r)
    ]
    .spacing(spacing::XS)
    .into()
}

fn header<'a>(state: &'a State, pane: &Pane, focused: bool, r: Roles) -> Element<'a, Message> {
    let (fill, strip) = focus_mark(focused, r);
    let (name, status) = match pane.terminal() {
        Some(t) => (
            terminals(state)
                .into_iter()
                .find(|(x, _)| *x == t)
                .map_or_else(|| session_title(state, t.session), |(_, n)| n),
            terminal_status(state, t),
        ),
        None => ("Empty pane".to_string(), String::new()),
    };
    let mark = container(Space::new())
        .width(FOCUS_STRIP)
        .height(STRIP_HEIGHT)
        .style(move |_| container::Style {
            background: strip.map(Into::into),
            ..container::Style::default()
        });
    let content = row![
        mark,
        Text::new(name, TypeRole::Label, r),
        Text::new(status, TypeRole::Label, r).muted(),
        Space::new().width(Length::Fill),
        split_buttons(pane, r),
        close_button(pane, r),
    ]
    .spacing(spacing::SM)
    .align_y(iced::Alignment::Center);
    container(content)
        .width(Length::Fill)
        .padding([spacing::XS, spacing::SM])
        .style(move |_| container::Style {
            background: Some(fill.into()),
            ..container::Style::default()
        })
        .into()
}

/// The empty-pane state: one button per terminal no pane shows (FR-004), so choosing is one press.
fn picker<'a>(
    state: &'a State,
    area: &PaneArea<'a>,
    pane: &Pane,
    r: Roles,
) -> Element<'a, Message> {
    let shown = area.layout.terminals();
    let mut list =
        column![Text::new("Choose a terminal for this pane", TypeRole::Caption, r).muted()]
            .spacing(spacing::SM);
    let mut any = false;
    for (t, name) in terminals(state) {
        if shown.contains(&t) {
            continue;
        }
        any = true;
        list = list.push(
            Button::with_content(Text::new(name, TypeRole::Label, r), ButtonVariant::Text, r)
                .on_press(Message::Session(SessionMsg::Pane(PaneMsg::Show(
                    pane.id(),
                    t,
                )))),
        );
    }
    if !any {
        list =
            list.push(Text::new("Every terminal is already shown.", TypeRole::Caption, r).muted());
    }
    container(list)
        .padding(spacing::LG)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// The panes of `area` tiled by a [`SplitView`] (FR-001). `selection` and `display_offset` belong to
/// the focused pane's terminal; every other pane draws live.
#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    state: &'a State,
    area: &PaneArea<'a>,
    selection: Option<&'a crate::selection::Selection>,
    display_offset: usize,
    scheme: ColorScheme,
    link_context: &LinkContext,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let focused_id = area.layout.focused();
    let keyboard = state.terminal_focused();
    let children: Vec<Element<'a, Message>> = area
        .layout
        .panes()
        .into_iter()
        .map(|pane| {
            let is_focused = pane.id() == focused_id;
            let body: Element<'a, Message> = match pane.terminal() {
                None => picker(state, area, pane, r),
                Some(t) => match area.grids.get(&t) {
                    Some(grid) => {
                        let mut p = TerminalPane::new(grid, TermPalette::from_scheme(scheme))
                            .focused(is_focused && keyboard)
                            .session(t.session)
                            .link_context(link_context.clone())
                            .pane(pane.id());
                        if is_focused {
                            p = p.selection(selection).display_offset(display_offset);
                        }
                        p.into()
                    }
                    None => container(
                        Text::new(
                            empty_terminal_message(state, t.session),
                            TypeRole::Caption,
                            r,
                        )
                        .muted(),
                    )
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into(),
                },
            };
            let body = GridSizeReporter::new(body).pane(pane.id());
            let tile: Element<'a, Message> = column![
                header(state, pane, is_focused, r),
                container(Element::from(body))
                    .width(Length::Fill)
                    .height(Length::Fill)
            ]
            .into();
            if pane.terminal().is_some() {
                // The terminal widget focuses its own pane on a press.
                return tile;
            }
            // An empty pane has no terminal widget to take the press: its header and body do
            // (FR-010). A button inside still handles its own press first.
            mouse_area(tile)
                .on_press(Message::Session(SessionMsg::Pane(PaneMsg::FocusPane(
                    pane.id(),
                ))))
                .into()
        })
        .collect();
    let split = SplitView::new(area.layout, min_pane(), r, children)
        .on_event(|e| Message::Session(SessionMsg::Pane(PaneMsg::Gesture(e))))
        .on_file_drop(|pane, path| {
            Message::Session(SessionMsg::Pane(PaneMsg::FileDropped(pane, path)))
        });
    match area.refusal {
        // Over the panes, not above them: a row above would resize every pane, and the
        // measurement that follows would dismiss the reason before it was read.
        Some(reason) => stack![
            Element::from(split),
            container(
                container(Text::new(reason, TypeRole::Caption, r))
                    .padding(spacing::SM)
                    .style(move |_| container::Style {
                        background: Some(pane_notice_fill(r).into()),
                        ..container::Style::default()
                    })
            )
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .padding(spacing::SM)
        ]
        .into(),
        None => split.into(),
    }
}

#[cfg(test)]
mod tests {
    //! The focus mark (feature 484, T015): visible without hover, not colour alone, both themes.
    use super::*;

    /// T039 (FR-010): an empty pane has no terminal widget to take a press, so its tile must be
    /// wrapped in a `mouse_area` that publishes `FocusPane`; a pane with a terminal must not be.
    #[test]
    fn an_empty_pane_tile_is_pressable_and_a_terminal_pane_is_not() {
        let src = include_str!("panes.rs");
        let body = &src[src.find("let tile: Element").expect("tile")..];
        let body = &body[..body.find(".collect();").expect("end of map")];
        assert!(body.contains("if pane.terminal().is_some()"), "{body}");
        let wrapped = body
            .split("mouse_area(tile)")
            .nth(1)
            .expect("mouse_area(tile)");
        assert!(wrapped.contains(".on_press(") && wrapped.contains("PaneMsg::FocusPane("));
    }

    #[test]
    fn a_focused_header_differs_from_an_unfocused_one_in_both_themes() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);
            let (on_fill, on_strip) = focus_mark(true, r);
            let (off_fill, off_strip) = focus_mark(false, r);
            assert_ne!(on_fill, off_fill, "{scheme:?}: the fill is the colour cue");
            assert!(on_strip.is_some(), "{scheme:?}: the strip is the shape cue");
            assert!(
                off_strip.is_none(),
                "{scheme:?}: an unfocused header has no strip"
            );
            const { assert!(FOCUS_STRIP >= 2.0) };
        }
    }
}
