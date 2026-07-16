//! `TreeView` — a reusable, theme-aware collapsible tree primitive (Constitution Principle VIII).
//!
//! Renders a flat list of [`TreeItem`]s as an indented, selectable tree with expand/collapse
//! toggles and optional trailing actions. The sidebar consumes it for worktrees → sessions;
//! any future hierarchical navigation should reuse it rather than fork a bespoke widget.

use crate::ui::{icon, style};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length};
use micold_ai_ide::icons::Icon;
use micold_ai_ide::tokens::{shape, spacing, type_scale, Rgb, Roles};

/// One row in a [`tree_view`]. Generic over the message type so it is reusable across features.
pub struct TreeItem<'a, M> {
    /// Nesting depth (0 = top level); drives leading indentation.
    pub depth: u16,
    /// A leading icon (e.g. a worktree/session/status glyph).
    pub icon: Option<Icon>,
    /// The row label.
    pub label: String,
    /// Foreground tint for the icon + label (status/theme aware).
    pub tint: Rgb,
    /// Whether this row is the selected one (highlighted).
    pub selected: bool,
    /// `Some(expanded)` when the row can expand/collapse children; drives the twisty.
    pub expandable: Option<bool>,
    /// Message when the twisty is toggled (only used when `expandable` is `Some`).
    pub on_toggle: Option<M>,
    /// Message when the row body is activated (select / open).
    pub on_press: Option<M>,
    /// An optional trailing action (icon + message), e.g. a close button.
    pub trailing: Option<(Icon, M)>,
    /// An optional tooltip describing the trailing action.
    pub trailing_tooltip: Option<String>,
    /// Lifetime marker so borrowed data can be captured by callers if needed.
    pub _marker: std::marker::PhantomData<&'a ()>,
}

impl<M> TreeItem<'_, M> {
    /// A minimal row at `depth` with `label`; fill in the rest with the setters.
    pub fn new(depth: u16, label: impl Into<String>, tint: Rgb) -> Self {
        Self {
            depth,
            icon: None,
            label: label.into(),
            tint,
            selected: false,
            expandable: None,
            on_toggle: None,
            on_press: None,
            trailing: None,
            trailing_tooltip: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Set the leading icon.
    pub fn with_icon(mut self, glyph: Icon) -> Self {
        self.icon = Some(glyph);
        self
    }

    /// Mark selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Make the row expandable with the given current state and toggle message.
    pub fn expandable(mut self, expanded: bool, on_toggle: M) -> Self {
        self.expandable = Some(expanded);
        self.on_toggle = Some(on_toggle);
        self
    }

    /// Set the body activation message.
    pub fn on_press(mut self, message: M) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Add a trailing action with a tooltip describing what it triggers.
    pub fn trailing(mut self, glyph: Icon, message: M, tooltip: impl Into<String>) -> Self {
        self.trailing = Some((glyph, message));
        self.trailing_tooltip = Some(tooltip.into());
        self
    }
}

/// A tree rendered from a flat, pre-ordered list of [`TreeItem`]s (Principle VIII reusable
/// primitive, builder form): `TreeView::new(items, roles).into()`.
pub struct TreeView<'a, M> {
    items: Vec<TreeItem<'a, M>>,
    roles: Roles,
}

impl<'a, M: Clone + 'a> TreeView<'a, M> {
    /// A tree from a flat, pre-ordered `items` list, themed by `roles`.
    pub fn new(items: Vec<TreeItem<'a, M>>, roles: Roles) -> Self {
        Self { items, roles }
    }
}

impl<'a, M: Clone + 'a> From<TreeView<'a, M>> for Element<'a, M> {
    fn from(tv: TreeView<'a, M>) -> Self {
        let TreeView { items, roles: r } = tv;
        let mut col = column![].spacing(spacing::XS).width(Length::Fill);

        for item in items {
            let indent = spacing::MD + item.depth * spacing::MD;
            let mut line = row![Space::with_width(Length::Fixed(indent as f32))]
                .spacing(spacing::XS)
                .align_y(Alignment::Center)
                .width(Length::Fill);

            // Expand/collapse twisty (or a spacer to keep labels aligned).
            match item.expandable {
                Some(expanded) => {
                    let glyph = if expanded {
                        Icon::NavigateUp // rotated visual not available; reuse a chevron-like glyph
                    } else {
                        Icon::OpenProject
                    };
                    let mut twisty = button(icon(glyph, type_scale::LABEL, item.tint))
                        .padding(spacing::XS)
                        .style(style::text_button(r));
                    if let Some(msg) = item.on_toggle.clone() {
                        twisty = twisty.on_press(msg);
                    }
                    line = line.push(twisty);
                }
                None => {
                    line = line.push(Space::with_width(Length::Fixed(type_scale::LABEL as f32)))
                }
            }

            if let Some(glyph) = item.icon {
                line = line.push(icon(glyph, type_scale::BODY, item.tint));
            }

            line = line.push(
                text(item.label)
                    .size(type_scale::BODY)
                    .style(move |_t: &iced::Theme| text::Style {
                        color: Some(style::color(item.tint)),
                    })
                    .width(Length::Fill),
            );

            if let Some((glyph, msg)) = item.trailing {
                let btn = button(icon(glyph, type_scale::LABEL, item.tint))
                    .padding(spacing::XS)
                    .style(style::text_button(r))
                    .on_press(msg);
                let trailing: Element<'a, M> = match item.trailing_tooltip {
                    Some(tip) => super::Tooltip::new(btn, tip, r).into(),
                    None => btn.into(),
                };
                line = line.push(trailing);
            }

            // The whole row is a low-emphasis button when it has a press action, so selection
            // and hover feedback are consistent.
            let row_el: Element<'a, M> = if let Some(msg) = item.on_press.clone() {
                button(line)
                    .padding(spacing::XS)
                    .width(Length::Fill)
                    .style(style::text_button(r))
                    .on_press(msg)
                    .into()
            } else {
                line.into()
            };

            // Selected rows get a subtle surface-variant background.
            if item.selected {
                col = col.push(container(row_el).width(Length::Fill).style(
                    move |_t: &iced::Theme| iced::widget::container::Style {
                        background: Some(iced::Background::Color(iced::Color {
                            a: 0.5,
                            ..style::color(r.surface_variant)
                        })),
                        border: iced::Border {
                            radius: (shape::SM as f32).into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ));
            } else {
                col = col.push(row_el);
            }
        }

        col.into()
    }
}
