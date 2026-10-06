//! `TreeView` — a reusable, theme-aware collapsible tree primitive (Constitution Principle VIII).
//!
//! Renders a flat list of [`TreeItem`]s as an indented, selectable tree with expand/collapse
//! toggles and optional trailing actions. The sidebar consumes it for worktrees → sessions;
//! any future hierarchical navigation should reuse it rather than fork a bespoke widget.

use crate::icons::Icon;
use crate::ui::cdk::context_area::ContextArea;
use crate::ui::material::glyph::icon;
use crate::ui::material::style;
use crate::ui::material::TypeRole;
use iced::widget::{button, column, container, mouse_area, row, Row, Space};
use iced::{Alignment, Element, Length};
use micold_core::tokens::{anatomy, density, shape, spacing, Rgb, Roles};
use std::num::NonZeroUsize;

/// What a row's right-click becomes: a message built from the press point, in window pixels.
///
/// The same shape `cdk::ContextArea` publishes, deliberately — a row and a terminal tab answer the
/// same gesture, and a second shape for it is how the sidebar came to answer it differently
/// (BUG-008).
type OnRightPress<'a, M> = Box<dyn Fn((u16, u16)) -> M + 'a>;

/// What a row says about unread sessions (feature 039 FR-018, feature 575).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum RowUnread {
    /// Nothing.
    #[default]
    Read,
    /// A session row that is unread: the bare mark, and the emphasised label role.
    Unread,
    /// A location row holding `n` unread sessions: the mark with its count.
    Count(NonZeroUsize),
}

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
    /// What a right-click of the row becomes, built from the **press point** in window
    /// coordinates (feature 008 context menu, FR-013; the point since BUG-008, FR-029d).
    ///
    /// A message rather than a function was the whole of BUG-008: with no point to anchor to, both
    /// sidebar menus opened at a constant corner, and the constant then acquired a doc comment
    /// explaining that a row's position "the view does not know" — which was a description of this
    /// missing parameter. `cdk::ContextArea` has published the point since feature 012's BUG-005.
    pub on_right_press: Option<OnRightPress<'a, M>>,
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
    /// A short muted label rendered **on the same line**, after the name and before any trailing
    /// action (feature 026, FR-016 — the session row's AI CLI name).
    ///
    /// Not [`Self::tags`], and the difference is the whole reason this exists: tags open a *second
    /// line*, and a session row is always one line — `features/sidebar.rs::row_heights` says so and
    /// the scroll arithmetic believes it, silently. This changes the row's content and never its
    /// height.
    ///
    /// It is fixed-width by consequence rather than by declaration: it is short, and the name
    /// beside it is [`Ellipsized`](super::Ellipsized), so a narrow row shortens the *name*. That
    /// ordering is deliberate — the annotation is the identification, and it must not be what a
    /// narrow row drops.
    pub annotation: Option<(String, Rgb)>,
    /// Whether the row is unread (feature 039, FR-018): the [`UnreadMark`](super::UnreadMark) at
    /// the trailing edge, before any trailing action, and the label in the view's emphasised role
    /// ([`TreeView::selected_label_role`]). The badge slot is not touched, and the row keeps its
    /// height: the name is what a narrow row shortens (contract `unread-mark.md` U8).
    ///
    /// A location row instead carries the mark **with a count** (feature 575): the number of its
    /// unread sessions, in the slot the bare mark takes, and its label keeps the view's label
    /// role. Private, so a row is read, unread or counted and never two at once; the last of
    /// [`unread`](Self::unread) and [`unread_count`](Self::unread_count) wins.
    unread: RowUnread,
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
            annotation: None,
            unread: RowUnread::Read,
            _marker: std::marker::PhantomData,
        }
    }

    /// Show the number of a location's unread sessions at the trailing edge (feature 575).
    /// `0` draws nothing, as a read row (feature 575, FR-001).
    pub fn unread_count(mut self, count: usize) -> Self {
        self.unread = match NonZeroUsize::new(count) {
            Some(n) => RowUnread::Count(n),
            None => RowUnread::Read,
        };
        self
    }

    /// The role this row's label is drawn in: the view's emphasised role for a selected or an
    /// unread row, else the view's label role.
    fn label_role(&self, label_role: TypeRole, selected_label_role: Option<TypeRole>) -> TypeRole {
        match selected_label_role {
            Some(role) if self.selected || self.unread == RowUnread::Unread => role,
            _ => label_role,
        }
    }

    /// Mark the row unread (feature 039, FR-018). See [`TreeItem::unread`].
    pub fn unread(mut self, unread: bool) -> Self {
        self.unread = if unread {
            RowUnread::Unread
        } else {
            RowUnread::Read
        };
        self
    }

    /// Set a small badge shown between the leading icon and the label (e.g. the activity dot).
    pub fn badge(mut self, element: impl Into<Element<'a, M>>) -> Self {
        self.badge = Some(element.into());
        self
    }

    /// A short muted label on the same line, after the name (feature 026, FR-016). See
    /// [`TreeItem::annotation`] for why this is not a tag chip.
    pub fn annotation(mut self, text: impl Into<String>, tint: Rgb) -> Self {
        self.annotation = Some((text.into(), tint));
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

    /// Emit `f(point)` when the row is right-clicked, `point` being where the press landed in
    /// window coordinates — what a menu anchor takes (feature 008 context menu; FR-029d).
    pub fn on_right_press(mut self, f: impl Fn((u16, u16)) -> M + 'a) -> Self {
        self.on_right_press = Some(Box::new(f));
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
    selected_label_role: Option<TypeRole>,
    density: i8,
}

impl<'a, M: Clone + 'a> TreeView<'a, M> {
    /// A tree from a flat, pre-ordered `items` list, themed by `roles`.
    pub fn new(items: Vec<TreeItem<'a, M>>, roles: Roles) -> Self {
        Self {
            items,
            roles,
            label_role: TypeRole::Body,
            selected_label_role: None,
            density: density::STANDARD,
        }
    }

    /// The row density: `density::STANDARD` (48dp) or `density::DENSE` (36dp) — §7.2's two named
    /// densities, and no third (FR-026, FR-026a).
    ///
    /// A step on the shared scale rather than a height, so a list cannot invent its own compactness.
    /// The sidebar takes `DENSE`; every other list stays standard.
    pub fn density(mut self, step: i8) -> Self {
        self.density = step;
        self
    }

    /// Override the label + leading-icon role (e.g. the sidebar's 80% scale, FR-011).
    ///
    /// A role rather than a size: the sidebar's reduced density is a named, auditable decision in
    /// the scale, and taking an `f32` here let a caller pass any number at all.
    pub fn label_role(mut self, role: TypeRole) -> Self {
        self.label_role = role;
        self
    }

    /// The role the **selected** row's label is set at, when it should differ from the rest
    /// (feature 024, FR-003a).
    ///
    /// Unset by default, so a list that does not ask for it renders every row at
    /// [`label_role`](Self::label_role) exactly as before.
    ///
    /// It exists because a selected row is otherwise marked by fill alone, and fill is a colour —
    /// unavailable to a reader with a colour-vision deficit, and indistinguishable from the hover
    /// state layer that shares the row. Passing a role rather than a weight keeps the difference
    /// inside the type scale, where it is auditable, instead of letting a call site name a number.
    ///
    /// A view-level setting rather than a per-item one on purpose: which row is selected is
    /// already `TreeItem::selected`, and a second per-row way to say the same thing is a second
    /// thing to keep in step.
    ///
    /// It is also the role of an **unread** row's label (feature 039, FR-018): the view's one
    /// emphasised role. A view that does not set it draws an unread row's label as the others.
    pub fn selected_label_role(mut self, role: TypeRole) -> Self {
        self.selected_label_role = Some(role);
        self
    }
}

/// The colour of a location row's unread count (feature 575, contract A6).
///
/// Always the theme's text colour, whatever the row's own tint: on a missing worktree's
/// error-tinted row the number is read as a count, not as part of the error (R4).
pub(crate) fn count_tint(r: Roles, _row_tint: Rgb) -> Rgb {
    r.on_surface
}

impl<'a, M: Clone + 'a> From<TreeView<'a, M>> for Element<'a, M> {
    fn from(tv: TreeView<'a, M>) -> Self {
        let TreeView {
            items,
            roles: r,
            label_role,
            selected_label_role,
            density: step,
        } = tv;
        // The row's own height and horizontal padding both follow the density (§7.2): a dense row
        // is shorter *and* tighter, which is what keeps the sidebar's worktree count on screen.
        let (row_padding, icon_gap) = if step == density::DENSE {
            (
                anatomy::list_row::DENSE_PADDING,
                anatomy::list_row::DENSE_ICON_GAP,
            )
        } else {
            (
                anatomy::list_row::STANDARD_PADDING,
                anatomy::list_row::STANDARD_ICON_GAP,
            )
        };
        // The twisty glyph, and the width of the slot that stands in for it on a row that cannot
        // expand — one number, so labels align down the column whether or not a row has a twisty.
        let twisty_size = TypeRole::Label.size();
        let mut col = column![].spacing(spacing::XS).width(Length::Fill);

        for item in items {
            // Minimal base indent (feature 008, FR-009): depth-0 rows sit flush with the
            // sidebar's small left padding; each level nests by one step.
            let indent = f32::from(item.depth) * spacing::MD;
            let has_tags = !item.tags.is_empty();
            // Read before any field is moved out of `item`; see where the label is pushed below.
            let row_label_role = item.label_role(label_role, selected_label_role);
            // The indent spacer indents, and nothing else. It used to carry §7.2's height floor as
            // well, and that was BUG-005: this spacer's width *is* the indent, so on a depth-0 row
            // it is `Fixed(0)` — void — and iced drops a void child outright, floor and all. The
            // height therefore applied to nested rows only, which looked from any single specimen
            // like the component having no height at all, and it was then deleted on that reading.
            // The floor now lives on the row (below), where it belongs and where depth cannot reach
            // it (FR-026d).
            let mut line = row![Space::new().width(Length::Fixed(indent))]
                .spacing(icon_gap)
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
                        .style(style::text_button(r, None));
                    if let Some(msg) = item.on_toggle.clone() {
                        twisty = twisty.on_press(msg);
                    }
                    line = line.push(twisty);
                }
                None => line = line.push(Space::new().width(Length::Fixed(twisty_size))),
            }

            if let Some(glyph) = item.icon {
                // A **fixed slot**, not the glyph's own width. A glyph's advance is whatever the
                // face gives it — `AddWorktree` measures 14dp where the role says 16 — so a
                // free-width icon makes the label's column depend on which glyph a row happens to
                // carry. Two things follow from pinning it: labels line up down the column whether
                // or not their glyphs are the same width (the property the twisty slot beside it
                // already states), and the second line's indent below becomes arithmetic over
                // tokens rather than over font metrics, which is what BUG-006 was.
                //
                // `width` then `align_x` rather than `center_x`, which would set the length as
                // well and discard the slot — BUG-002's exact shape.
                line = line.push(
                    container(icon(glyph, label_role.size(), item.tint))
                        .width(Length::Fixed(label_role.size()))
                        .align_x(Alignment::Center),
                );
            }

            // A badge's width is a caller's element rather than a token, so it cannot be folded
            // into the second line's indent below. It never has to be: a badge marks a *session*
            // row and tags mark a *worktree* row, and no row is both. Stated rather than assumed,
            // because the day one is both the chips would drift and nothing would say why.
            debug_assert!(
                !(item.badge.is_some() && has_tags),
                "a row carrying both a badge and tags cannot align its second line: the badge's \
                 width is not a token this component knows"
            );
            // The per-session activity dot sits between the icon and the name (feature 010 US2).
            if let Some(badge) = item.badge {
                line = line.push(badge);
            }

            // One line, ending in an ellipsis when the name is longer than the space the row's
            // controls leave it. This used to be a plain `text(...).wrapping(Wrapping::None)`,
            // which keeps the single line but does not clip: a long session name was measured at
            // its full width and drawn straight over the close button beside it.
            // The selected row may carry a heavier role than its siblings (FR-003a). Only the
            // *label* takes it: the leading icon and the second line's indent stay on
            // `label_role`, so a row changing emphasis cannot shift the column its name starts in.
            //
            // An unread row takes the same role (feature 039, FR-018): the view has one emphasised
            // role, and the unread mark is told from the activity badge by this weight as well as
            // by its place in the row.
            line = line.push(super::Ellipsized::at_role(
                item.label,
                row_label_role,
                item.tint,
            ));

            // The row's own short annotation (feature 026): after the name, before the actions, on
            // the same line. It takes its natural width and the name takes the remainder, which is
            // what makes the *name* what a narrow row shortens.
            if let Some((text, tint)) = item.annotation {
                line = line.push(super::Text::new(text, TypeRole::Caption, r).tint(tint));
            }

            // The unread mark (feature 039): at the trailing edge, before any trailing action. It
            // has a fixed size and the name beside it is `Ellipsized`, so a narrow row shortens the
            // name and keeps the mark.
            //
            // A location row's count (feature 575) takes the same slot, in the row's label role
            // and the theme's text colour, never worded: the tooltip carries the words (FR-005).
            match item.unread {
                RowUnread::Read => {}
                RowUnread::Unread => line = line.push(super::UnreadMark::new(r)),
                RowUnread::Count(n) => {
                    line = line.push(
                        super::UnreadMark::new(r)
                            .count(n.get())
                            .role(row_label_role)
                            .tint(count_tint(r, item.tint)),
                    )
                }
            }

            if let Some(custom) = item.trailing_custom {
                line = line.push(custom);
            } else if let Some((glyph, msg)) = item.trailing {
                let btn = button(icon(glyph, twisty_size, item.tint))
                    .padding(spacing::XS)
                    .style(style::text_button(r, None))
                    .on_press(msg);
                let trailing: Element<'a, M> = match item.trailing_tooltip {
                    Some(tip) => super::Tooltip::new(btn, tip, r).into(),
                    None => btn.into(),
                };
                line = line.push(trailing);
            }

            // The row body: the name line, plus an optional second line of tag chips aligned
            // beneath the label (feature 008, FR-001). A worktree with tags becomes two lines —
            // and is, for §7.2's purposes, Material's *two-line* list item rather than a one-line
            // row that happens to be taller.
            let (content, base): (Element<'a, M>, f32) = if item.tags.is_empty() {
                (line.into(), density::LIST_ROW_BASE)
            } else {
                // Where the label starts, derived from the same leading run the line above is
                // built from rather than guessed at (BUG-006, feature 008 FR-001).
                //
                // The old expression was `indent + label_role.size() + spacing::SM` — one icon and
                // one gap — against a row that is `indent → twisty → gap → icon → gap → label`.
                // It was never a near-miss of the right formula; it was a different formula that
                // happened to land within 4dp in the sidebar (no leading icon, 8dp gaps, a 12dp
                // label role) and 47dp out in the gallery. Being right in one corner of its own
                // matrix is what kept it invisible.
                //
                // Each term below is the width of a child actually pushed above, in order:
                let tag_indent = {
                    // The indent spacer is `Fixed(indent)`, and at depth 0 that is `Fixed(0)` —
                    // *void*, so iced drops it and the gap that would have followed it with it.
                    // The same rule that hid §7.2's row height in BUG-005, here deciding whether
                    // the leading run starts with a spacer at all.
                    let lead = if indent > 0.0 { indent + icon_gap } else { 0.0 };
                    // A twisty is a button carrying `spacing::XS` on every side; the slot that
                    // stands in for it on a row that cannot expand is the bare glyph width.
                    let twisty = if item.expandable.is_some() {
                        twisty_size + 2.0 * spacing::XS
                    } else {
                        twisty_size
                    };
                    let leading_icon = if item.icon.is_some() {
                        label_role.size() + icon_gap
                    } else {
                        0.0
                    };
                    lead + twisty + icon_gap + leading_icon
                };
                // The indent is **padding**, not a leading spacer child. A spacer would sit one
                // `spacing::XS` to the left of the first chip — the row's own between-chip gap
                // applies after it — so the chips would land 4dp right of the label however
                // carefully `tag_indent` was computed. Padding also cannot go void at depth 0.
                let mut tag_row: Row<'a, M> = Row::new()
                    .spacing(spacing::XS)
                    .align_y(Alignment::Center)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: tag_indent,
                    });
                for (label, accent) in item.tags {
                    tag_row = tag_row.push(super::Tag::new(label, accent));
                }
                (
                    column![line, tag_row].spacing(2).width(Length::Fill).into(),
                    density::LIST_ROW_TWO_LINE_BASE,
                )
            };

            // §7.2's row height, as a **minimum on the row** (FR-026d). Three things about this
            // one line, each of which was wrong before:
            //
            // - It is on the *row*, not on the name line. Flooring the line makes a tagged row
            //   `floor + gap + tags` instead of `max(floor, content)` — which is how BUG-005's
            //   predecessor computed a ~30% cost for a change that costs 7.7%, and then deleted the
            //   contract's figure to avoid the number it had just invented.
            // - The spacer's **width stays `Shrink`**. A `Fixed(0)` on either axis makes a child
            //   void and iced deletes it, taking the floor with it; that is what happened when the
            //   floor rode on the indent spacer, and it is the fourth time this trap has bitten
            //   here after `FormField`'s slots and the snackbar's own minimum height. `snackbar.rs`
            //   solves it the same way and says so.
            // - Nothing consults `item.depth`. A row's height is a property of its density and its
            //   line count, and of nothing else.
            //
            // `Length::Shrink` on the row below keeps this a floor rather than a cap: a row holding
            // more than its figure grows instead of clipping.
            let body: Element<'a, M> = row![
                Space::new().height(Length::Fixed(density::height(base, step))),
                content
            ]
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .into();

            // The whole row is a low-emphasis button when it has a press action, so selection
            // and hover feedback are consistent.
            let row_el: Element<'a, M> = if let Some(msg) = item.on_press.clone() {
                // Ripples from wherever the row was clicked. A row is the most-pressed surface in
                // this application, so it is the one where a flat colour swap is most obviously not
                // Material (FR-024c).
                super::Ripple::new(
                    button(body)
                        // §7.2's horizontal padding follows the density, so a dense row is tighter
                        // as well as shorter. Vertical padding is zero deliberately: the height is
                        // the row's floor (above), and padding here would add to it rather than
                        // fill it, putting the row above its density's figure for no stated reason.
                        .padding(iced::Padding {
                            top: 0.0,
                            bottom: 0.0,
                            left: row_padding,
                            right: row_padding,
                        })
                        .width(Length::Fill)
                        .height(Length::Shrink)
                        .style(style::text_button(r, None))
                        .on_press(msg),
                    r.on_surface,
                    shape::FULL,
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

            // Hover-reveal of the row's actions (feature 008) is a `mouse_area`; the right-click
            // is not, because a `mouse_area`'s `on_right_press` takes a bare message and throws the
            // press point away. `cdk::ContextArea` keeps it, which is what lets the menu open where
            // the user pressed (BUG-008, FR-029d).
            let hovering: Element<'a, M> = if item.on_hover.is_some() || item.on_unhover.is_some() {
                let mut area = mouse_area(styled);
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
            let interactive: Element<'a, M> = match item.on_right_press {
                Some(build) => ContextArea::new(hovering).on_secondary_press(build).into(),
                None => hovering,
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

#[cfg(test)]
mod tests {
    //! Feature 039 (contract `unread-mark.md`, hosts table and U8): a row with the unread mark.

    use super::*;
    use crate::ui::material::ActivityBadge;
    use iced::advanced::layout;
    use iced::advanced::widget::Tree;
    use iced::{Rectangle, Size};
    use micold_core::protocol::messages::ActivitySignal;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens;

    /// The sidebar's width at its narrowest, and a roomy one.
    const NARROW: f32 = 160.0;
    const ROOMY: f32 = 320.0;
    /// The unread mark's diameter (U1).
    const MARK: f32 = 8.0;
    /// A trailing action of a size nothing else in a row has.
    const ACTION: f32 = 21.0;
    const TOLERANCE: f32 = 0.5;

    const LONG_NAME: &str =
        "Refactor the session supervisor so that restarts keep their scrollback";

    fn roles() -> Roles {
        tokens::roles(ColorScheme::Light)
    }

    /// A session row as the sidebar builds it: depth 1, with the activity badge.
    fn session_row(label: &str) -> TreeItem<'static, ()> {
        TreeItem::new(1, label, roles().on_surface).badge(ActivityBadge::<()>::new(
            ActivitySignal::AwaitingInput,
            roles(),
        ))
    }

    /// Lay one row out in a dense tree `width` wide, as the sidebar does, and return every node of
    /// it in window coordinates, the row's own first.
    fn laid_out(item: TreeItem<'static, ()>, width: f32) -> Vec<Rectangle> {
        let mut element: Element<'static, ()> = TreeView::new(vec![item], roles())
            .density(density::DENSE)
            .label_role(TypeRole::SidebarName)
            .selected_label_role(TypeRole::SidebarSessionCurrent)
            .into();
        let renderer = super::super::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, Size::new(width, 600.0));
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);

        fn walk(node: &layout::Node, origin: (f32, f32), out: &mut Vec<Rectangle>) {
            let bounds = node.bounds();
            let here = (origin.0 + bounds.x, origin.1 + bounds.y);
            out.push(Rectangle {
                x: here.0,
                y: here.1,
                ..bounds
            });
            for child in node.children() {
                walk(child, here, out);
            }
        }
        let mut nodes = Vec::new();
        walk(&node.children()[0], (0.0, 0.0), &mut nodes);
        nodes
    }

    fn sized(nodes: &[Rectangle], width: f32, height: f32) -> Vec<Rectangle> {
        nodes
            .iter()
            .filter(|n| {
                (n.width - width).abs() < TOLERANCE && (n.height - height).abs() < TOLERANCE
            })
            .copied()
            .collect()
    }

    /// The mark's box: the one 8dp square of the row.
    fn mark(nodes: &[Rectangle]) -> Rectangle {
        let marks = sized(nodes, MARK, MARK);
        assert!(
            !marks.is_empty(),
            "the row has no 8dp mark; its nodes are {nodes:?}"
        );
        marks[0]
    }

    /// U149 (U8, FR-032).
    #[test]
    fn an_unread_row_has_the_height_of_a_read_one() {
        for label in ["feat-short", LONG_NAME] {
            let read = laid_out(session_row(label), NARROW)[0];
            let unread = laid_out(session_row(label).unread(true), NARROW)[0];

            assert!(
                sized(&laid_out(session_row(label), NARROW), MARK, MARK).is_empty(),
                "precondition: a read row has no mark"
            );
            assert_eq!(
                unread.height, read.height,
                "the mark and the emphasised label leave the row's height as it was ({label:?})"
            );
        }
    }

    /// U150 (U8, FR-018): the name gives way, the mark does not.
    #[test]
    fn a_long_label_is_cut_short_before_the_mark_is_pushed_out() {
        let nodes = laid_out(session_row(LONG_NAME).unread(true), NARROW);
        let (row, mark) = (nodes[0], mark(&nodes));

        assert!(
            mark.x + mark.width <= row.x + row.width + TOLERANCE,
            "the mark is inside the row: it ends at {} and the row at {}",
            mark.x + mark.width,
            row.x + row.width
        );
    }

    /// Hosts table: the mark is in the trailing slot, before any trailing action.
    #[test]
    fn the_mark_trails_the_label_and_comes_before_a_trailing_action() {
        let action = || -> Element<'static, ()> {
            Space::new()
                .width(Length::Fixed(ACTION))
                .height(Length::Fixed(ACTION))
                .into()
        };
        let nodes = laid_out(
            session_row("feat-short")
                .unread(true)
                .trailing_element(action()),
            ROOMY,
        );
        let mark = mark(&nodes);
        let action = sized(&nodes, ACTION, ACTION)[0];

        assert!(
            mark.x + mark.width <= action.x + TOLERANCE,
            "the mark ends at {} and the trailing action starts at {}",
            mark.x + mark.width,
            action.x
        );
        assert!(
            mark.x > ROOMY / 2.0,
            "the mark is at the row's trailing edge, not beside the badge: x = {}",
            mark.x
        );
    }

    /// U151 (FR-018, FR-032): the activity badge is where it was, and the size it was.
    #[test]
    fn the_badge_slot_is_the_same_node_with_and_without_the_mark() {
        let slot = TypeRole::SidebarTag.size();
        let badge = |nodes: &[Rectangle]| -> Rectangle {
            *nodes
                .iter()
                .find(|n| (n.width - slot).abs() < TOLERANCE)
                .expect("the row has a badge slot")
        };
        for label in ["feat-short", LONG_NAME] {
            let read = badge(&laid_out(session_row(label), NARROW));
            let unread = badge(&laid_out(session_row(label).unread(true), NARROW));

            assert_eq!(
                unread, read,
                "the activity badge's box is unchanged by the unread mark ({label:?})"
            );
        }
    }

    // --- Feature 575 (contract `attention-indicator.md` A4-A6, A10): a location row's count ---

    /// A location row as the sidebar builds it: depth 0, expandable, optionally with tag chips.
    fn location_row(label: &str, tagged: bool) -> TreeItem<'static, ()> {
        let row = TreeItem::new(0, label, roles().on_surface).expandable(false, ());
        if tagged {
            row.tags(vec![("feat".to_string(), roles().primary)])
        } else {
            row
        }
    }

    /// The width of the mark and its number drawn on their own, in the sidebar's label role.
    fn indicator_width(count: usize) -> f32 {
        let element: Element<'static, ()> = super::super::UnreadMark::new(roles())
            .count(count)
            .role(TypeRole::SidebarName)
            .into();
        let renderer = super::super::test_support::renderer();
        let mut element = element;
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, Size::new(600.0, 600.0));
        element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits)
            .bounds()
            .width
    }

    /// The node that starts where the mark does and is as wide as the whole indicator.
    fn indicator(nodes: &[Rectangle], count: usize) -> Rectangle {
        let (mark, width) = (mark(nodes), indicator_width(count));
        *nodes
            .iter()
            .find(|n| (n.x - mark.x).abs() < TOLERANCE && (n.width - width).abs() < TOLERANCE)
            .unwrap_or_else(|| panic!("no {width}dp indicator starting at the mark: {nodes:?}"))
    }

    /// The trailing cluster as the sidebar reserves it: always laid out, shown on hover.
    fn cluster(shown: bool) -> Element<'static, ()> {
        super::super::HoverReveal::new(
            Space::new()
                .width(Length::Fixed(ACTION))
                .height(Length::Fixed(ACTION)),
            roles().surface,
        )
        .shown(shown)
        .into()
    }

    /// U-575 (A4, FR-004): the count keeps a location row at its height, one-line and two-line.
    #[test]
    fn a_counted_location_row_has_the_height_of_a_plain_one() {
        for tagged in [false, true] {
            for label in ["feat-short", LONG_NAME] {
                let plain = laid_out(location_row(label, tagged), NARROW)[0];
                let counted = laid_out(location_row(label, tagged).unread_count(2), NARROW);

                let _ = mark(&counted);
                assert_eq!(
                    counted[0].height, plain.height,
                    "the count leaves the row's height as it was ({label:?}, tagged: {tagged})"
                );
            }
        }
    }

    /// U-575 (A4, U8): a narrow row shortens the name and the indicator keeps its full width.
    #[test]
    fn a_long_name_is_cut_short_before_the_count_is() {
        let nodes = laid_out(location_row(LONG_NAME, false).unread_count(12), NARROW);
        let (row, indicator) = (nodes[0], indicator(&nodes, 12));

        assert!(
            indicator.x + indicator.width <= row.x + row.width + TOLERANCE,
            "the whole mark and number are inside the row: it ends at {} and the row at {}",
            indicator.x + indicator.width,
            row.x + row.width
        );
    }

    /// U-575 (A4, A5, FR-004): before the trailing cluster, clear of it, and still on hover.
    #[test]
    fn the_count_comes_before_the_row_actions_and_does_not_move_on_hover() {
        let at = |shown: bool| {
            let nodes = laid_out(
                location_row("feat-short", true)
                    .unread_count(2)
                    .trailing_element(cluster(shown)),
                ROOMY,
            );
            let action = sized(&nodes, ACTION, ACTION)[0];
            (indicator(&nodes, 2), action)
        };
        let (rest, action) = at(false);
        let (hovered, hovered_action) = at(true);

        assert!(
            rest.x + rest.width <= action.x + TOLERANCE,
            "the count ends at {} and the row actions start at {}",
            rest.x + rest.width,
            action.x
        );
        assert_eq!(
            hovered, rest,
            "hovering the row does not move or resize the count"
        );
        assert_eq!(hovered_action, action, "the cluster's width is reserved");
    }

    /// U-575 (A6, R4): a counted row's name keeps the view's label role; only `Unread` emphasises.
    #[test]
    fn a_counted_row_keeps_its_label_role() {
        let (plain, emphasised) = (TypeRole::SidebarName, Some(TypeRole::SidebarSessionCurrent));

        assert_eq!(
            location_row("feat-short", false)
                .unread_count(2)
                .label_role(plain, emphasised),
            plain,
            "a location row with a count is not emphasised"
        );
        assert_eq!(
            session_row("feat-short")
                .unread(true)
                .label_role(plain, emphasised),
            TypeRole::SidebarSessionCurrent,
            "an unread session row still is (039 FR-018)"
        );
    }

    /// U-575 (A1, U2): a count of zero draws nothing.
    #[test]
    fn a_count_of_zero_draws_nothing() {
        let plain = laid_out(location_row("feat-short", false), ROOMY);
        let zero = laid_out(location_row("feat-short", false).unread_count(0), ROOMY);

        assert!(sized(&zero, MARK, MARK).is_empty(), "no mark at zero");
        assert_eq!(
            zero, plain,
            "a row with no unread session is laid out as before"
        );
    }

    /// U-575 (A10, data-model "Row unread state"): the last of `.unread` and `.unread_count` wins.
    #[test]
    fn the_last_unread_builder_call_wins() {
        let counted = laid_out(location_row("feat-short", false).unread_count(2), ROOMY);

        assert_eq!(
            laid_out(
                location_row("feat-short", false)
                    .unread(true)
                    .unread_count(2),
                ROOMY
            ),
            counted,
            "`.unread_count(2)` after `.unread(true)` draws the count"
        );
        assert_eq!(
            laid_out(
                location_row("feat-short", false)
                    .unread_count(2)
                    .unread(false),
                ROOMY
            ),
            laid_out(location_row("feat-short", false), ROOMY),
            "`.unread(false)` after `.unread_count(2)` draws nothing"
        );
        assert_eq!(
            laid_out(
                location_row("feat-short", false)
                    .unread_count(2)
                    .unread(true),
                ROOMY
            ),
            laid_out(location_row("feat-short", false).unread(true), ROOMY),
            "`.unread(true)` after `.unread_count(2)` draws the bare mark"
        );
    }

    /// U-575 (A6, R4): on an error-tinted row the count is in the theme's text colour.
    #[test]
    fn the_count_is_drawn_in_the_text_colour_on_an_error_tinted_row() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);
            assert_eq!(
                count_tint(r, r.error),
                r.on_surface,
                "a missing worktree's count is read as a number, not as part of the error ({scheme:?})"
            );
            assert_eq!(count_tint(r, r.on_surface), r.on_surface);
        }
    }
}
