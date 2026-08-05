//! `TreeView` — a reusable, theme-aware collapsible tree primitive (Constitution Principle VIII).
//!
//! Renders a flat list of [`TreeItem`]s as an indented, selectable tree with expand/collapse
//! toggles and optional trailing actions. The sidebar consumes it for worktrees → sessions;
//! any future hierarchical navigation should reuse it rather than fork a bespoke widget.

use crate::icons::Icon;
use crate::ui::material::glyph::icon;
use crate::ui::material::style;
use crate::ui::material::TypeRole;
use iced::widget::{button, column, container, mouse_area, row, Row, Space};
use iced::{Alignment, Element, Length};
use micold_core::tokens::{shape, spacing, Rgb, Roles};

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
    /// Color-coded tag chips rendered on a second line beneath the label as `(label, accent)`
    /// (feature 008, FR-001). Empty ⇒ single-line row.
    pub tags: Vec<(String, Rgb)>,
    /// Message emitted on a right-click of the row (feature 008 context menu, FR-013).
    pub on_right_press: Option<M>,
    /// Message emitted when the pointer enters the row (feature 008 hover-reveal).
    pub on_hover: Option<M>,
    /// Message emitted when the pointer leaves the row (feature 008 hover-reveal).
    pub on_unhover: Option<M>,
    /// A pre-built trailing cluster (e.g. hover-revealed row actions). When set it replaces the
    /// simple `trailing` icon button. Carries its own lifetime `'a`.
    pub trailing_custom: Option<Element<'a, M>>,
    /// A hover tooltip describing the row's own location (feature 010, FR-010) — distinct from
    /// `trailing_tooltip`, which describes only the trailing action button.
    pub row_tooltip: Option<String>,
    /// A small pre-built badge shown between the leading icon and the label (feature 010 US2 —
    /// the per-session activity dot). Carries its own lifetime `'a`.
    pub badge: Option<Element<'a, M>>,
    /// Lifetime marker so borrowed data can be captured by callers if needed.
    pub _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a, M> TreeItem<'a, M> {
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
            tags: Vec::new(),
            on_right_press: None,
            on_hover: None,
            on_unhover: None,
            trailing_custom: None,
            row_tooltip: None,
            badge: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Set a small badge shown between the leading icon and the label (e.g. the activity dot).
    pub fn badge(mut self, element: impl Into<Element<'a, M>>) -> Self {
        self.badge = Some(element.into());
        self
    }

    /// A hover tooltip describing this row's own location (feature 010, FR-010) — shown for
    /// the whole row, not just a trailing action.
    pub fn row_tooltip(mut self, text: impl Into<String>) -> Self {
        self.row_tooltip = Some(text.into());
        self
    }

    /// Emit `on_hover` when the pointer enters the row and `on_unhover` when it leaves
    /// (feature 008 hover-reveal).
    pub fn hover(mut self, on_hover: M, on_unhover: M) -> Self {
        self.on_hover = Some(on_hover);
        self.on_unhover = Some(on_unhover);
        self
    }

    /// Set a pre-built trailing cluster (replaces the simple `trailing` icon).
    pub fn trailing_element(mut self, element: impl Into<Element<'a, M>>) -> Self {
        self.trailing_custom = Some(element.into());
        self
    }

    /// Attach color-coded tag chips shown on a second line beneath the label
    /// (feature 008, FR-001). Each is `(label, accent)`.
    pub fn tags(mut self, tags: Vec<(String, Rgb)>) -> Self {
        self.tags = tags;
        self
    }

    /// Emit `message` when the row is right-clicked (feature 008 context menu).
    pub fn on_right_press(mut self, message: M) -> Self {
        self.on_right_press = Some(message);
        self
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
    label_role: TypeRole,
}

impl<'a, M: Clone + 'a> TreeView<'a, M> {
    /// A tree from a flat, pre-ordered `items` list, themed by `roles`.
    pub fn new(items: Vec<TreeItem<'a, M>>, roles: Roles) -> Self {
        Self {
            items,
            roles,
            label_role: TypeRole::Body,
        }
    }

    /// Override the label + leading-icon role (e.g. the sidebar's 80% scale, FR-011).
    ///
    /// A role rather than a size: the sidebar's reduced density is a named, auditable decision in
    /// the scale, and taking an `f32` here let a caller pass any number at all.
    pub fn label_role(mut self, role: TypeRole) -> Self {
        self.label_role = role;
        self
    }
}

impl<'a, M: Clone + 'a> From<TreeView<'a, M>> for Element<'a, M> {
    fn from(tv: TreeView<'a, M>) -> Self {
        let TreeView {
            items,
            roles: r,
            label_role,
        } = tv;
        // The twisty glyph, and the width of the slot that stands in for it on a row that cannot
        // expand — one number, so labels align down the column whether or not a row has a twisty.
        let twisty_size = TypeRole::Label.size();
        let mut col = column![].spacing(spacing::XS).width(Length::Fill);

        for item in items {
            // Minimal base indent (feature 008, FR-009): depth-0 rows sit flush with the
            // sidebar's small left padding; each level nests by one step.
            let indent = f32::from(item.depth) * spacing::MD;
            let mut line = row![Space::new().width(Length::Fixed(indent))]
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
                    let mut twisty = button(icon(glyph, twisty_size, item.tint))
                        .padding(spacing::XS)
                        .style(style::text_button(r));
                    if let Some(msg) = item.on_toggle.clone() {
                        twisty = twisty.on_press(msg);
                    }
                    line = line.push(twisty);
                }
                None => line = line.push(Space::new().width(Length::Fixed(twisty_size))),
            }

            if let Some(glyph) = item.icon {
                line = line.push(icon(glyph, label_role.size(), item.tint));
            }

            // The per-session activity dot sits between the icon and the name (feature 010 US2).
            if let Some(badge) = item.badge {
                line = line.push(badge);
            }

            // One line, ending in an ellipsis when the name is longer than the space the row's
            // controls leave it. This used to be a plain `text(...).wrapping(Wrapping::None)`,
            // which keeps the single line but does not clip: a long session name was measured at
            // its full width and drawn straight over the close button beside it.
            line = line.push(super::Ellipsized::at_role(
                item.label, label_role, item.tint,
            ));

            if let Some(custom) = item.trailing_custom {
                line = line.push(custom);
            } else if let Some((glyph, msg)) = item.trailing {
                let btn = button(icon(glyph, twisty_size, item.tint))
                    .padding(spacing::XS)
                    .style(style::text_button(r))
                    .on_press(msg);
                let trailing: Element<'a, M> = match item.trailing_tooltip {
                    Some(tip) => super::Tooltip::new(btn, tip, r).into(),
                    None => btn.into(),
                };
                line = line.push(trailing);
            }

            // The row body: the name line, plus an optional second line of tag chips aligned
            // beneath the label (feature 008, FR-001). A worktree with tags becomes two lines.
            let body: Element<'a, M> = if item.tags.is_empty() {
                line.into()
            } else {
                let tag_indent = indent + label_role.size() + spacing::SM;
                let mut tag_row: Row<'a, M> = row![Space::new().width(Length::Fixed(tag_indent))]
                    .spacing(spacing::XS)
                    .align_y(Alignment::Center);
                for (label, accent) in item.tags {
                    tag_row = tag_row.push(super::Tag::new(label, accent));
                }
                column![line, tag_row].spacing(2).width(Length::Fill).into()
            };

            // The whole row is a low-emphasis button when it has a press action, so selection
            // and hover feedback are consistent.
            let row_el: Element<'a, M> = if let Some(msg) = item.on_press.clone() {
                // Ripples from wherever the row was clicked. A row is the most-pressed surface in
                // this application, so it is the one where a flat colour swap is most obviously not
                // Material (FR-024c).
                super::Ripple::new(
                    button(body)
                        .padding(spacing::XS)
                        .width(Length::Fill)
                        .style(style::text_button(r))
                        .on_press(msg),
                    r.on_surface,
                )
                .into()
            } else {
                body
            };

            // The selection pill (contract §7.2): `secondary_container` fill, its own text colour,
            // and the `full` corner.
            //
            // This replaces a half-alpha `surface_variant` wash. Two things were wrong with that.
            // A translucent neutral over whatever the row sat on produced a different colour in
            // each place it was used, so "selected" had no single appearance; and it was close
            // enough to the hover layer that a hovered row and a selected one were hard to tell
            // apart — which is the distinction FR-020 exists to make, since selection persists and
            // hover does not.
            let styled: Element<'a, M> = if item.selected {
                container(row_el)
                    .width(Length::Fill)
                    .style(move |_t: &iced::Theme| iced::widget::container::Style {
                        background: Some(iced::Background::Color(style::color(
                            r.secondary_container,
                        ))),
                        text_color: Some(style::color(r.on_secondary_container)),
                        border: iced::Border {
                            radius: shape::FULL.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into()
            } else {
                row_el
            };

            // Wrap the row in a mouse_area when it needs pointer interactions: a right-click
            // context menu and/or hover-reveal of its actions (feature 008).
            let interactive: Element<'a, M> = if item.on_right_press.is_some()
                || item.on_hover.is_some()
                || item.on_unhover.is_some()
            {
                let mut area = mouse_area(styled);
                if let Some(msg) = item.on_right_press {
                    area = area.on_right_press(msg);
                }
                if let Some(msg) = item.on_hover {
                    area = area.on_enter(msg);
                }
                if let Some(msg) = item.on_unhover {
                    area = area.on_exit(msg);
                }
                area.into()
            } else {
                styled
            };
            // A row-level location tooltip (feature 010, FR-010) wraps everything, including any
            // right-click/hover interaction area above.
            let final_el: Element<'a, M> = match item.row_tooltip {
                Some(tip) => super::Tooltip::new(interactive, tip, r).into(),
                None => interactive,
            };
            col = col.push(final_el);
        }

        col.into()
    }
}
