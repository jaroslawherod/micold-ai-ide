//! `Button` — the library's wrapper around the rendering stack's button (Principle VIII).
//!
//! Three variants are in use today, and every call site picked one by naming a style function
//! directly. That is the leak this feature closes: a call site that can name `style::filled` can
//! also name `style::outlined` for a confirm action, or forget the style entirely, and nothing
//! stops it. A call site now names the *variant* and cannot reach the style layer at all.
//!
//! Icon-only buttons keep their own component ([`IconButton`](super::IconButton)) — they carry a
//! glyph rather than a label, and a disabled glyph needs colouring the label path does not.
//!
//! Parity: each variant resolves to exactly the style its call sites use today (FR-005).

use crate::give_way::{Floor, GiveWay, Group, Lane, Slot};
use crate::glyph::icon;
use crate::icons::Icon;
use crate::keyboard_focus::{Indicator, TakesTheKeyboard};
use crate::style;
use crate::text::{Text, TypeRole};
use iced::widget::{button, container};
use iced::{keyboard, Alignment, Element, Length, Padding};
use micold_core::tokens::{anatomy, density, shape, spacing, Rgb, Roles};

/// Each `impl Fn` returned by the style layer is a distinct opaque type, so the variants are boxed
/// behind one signature to be chosen at runtime.
type ButtonStyleFn = Box<dyn Fn(&iced::Theme, button::Status) -> button::Style>;

/// How much emphasis the button carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// The primary action of a screen or dialog. One per group.
    Filled,
    /// A secondary action of equal standing — cancel beside confirm.
    Outlined,
    /// A low-emphasis action that sits inside other content: a menu entry, a list row, a tab.
    Text,
}

impl Variant {
    /// Every variant, so a gate that reads the set cannot silently miss one added later.
    pub const ALL: [Variant; 3] = [Variant::Filled, Variant::Outlined, Variant::Text];

    /// The content colour this variant draws its label in — and therefore the colour its ripple
    /// takes, since a state layer is the content colour over the container (contract §5).
    ///
    /// `pub` for controls *nested inside* a button, which have to match it: a `Button` sets
    /// `text_color` for its own label, but a nested `IconButton` sets an explicit glyph colour that
    /// overrides anything inherited, and its default is the roles' `on_surface` — wrong on any
    /// variant that paints its own fill. Asking the variant is what keeps the two in step; feature
    /// 012 BUG-001 is what happens when they are not.
    /// `host` is what the button is standing on, where anything told it (FR-004a). An accent fill
    /// substitutes its own pair for the accent roles: the filled variant's container becomes the
    /// host's foreground and its label the host's fill, and the two variants that draw no container
    /// take the host's foreground for their content.
    pub fn content(self, roles: Roles, host: Option<style::Host>) -> micold_core::tokens::Rgb {
        match (self, host.and_then(style::Host::imposed)) {
            (Variant::Filled, Some((fill, _))) => fill,
            (Variant::Outlined | Variant::Text, Some((_, on_fill))) => on_fill,
            (Variant::Filled, None) => roles.on_primary,
            (Variant::Outlined | Variant::Text, None) => roles.primary,
        }
    }

    pub fn style(self, roles: Roles, host: Option<style::Host>) -> ButtonStyleFn {
        match self {
            Variant::Filled => Box::new(style::filled(roles, host)),
            Variant::Outlined => Box::new(style::outlined(roles, host)),
            Variant::Text => Box::new(style::text_button(roles, host)),
        }
    }

    /// §7.3's horizontal padding for this variant — 24 for the two that draw a container, 12 for
    /// the text variant, "because a text button has no container to balance against".
    ///
    /// Read from the contract rather than left to the rendering stack, which insets a button by its
    /// own `DEFAULT_PADDING` of 10dp. That is what every labelled button in the application took
    /// while all three of these constants were referenced by nothing.
    fn padding(self) -> f32 {
        match self {
            Variant::Filled => anatomy::button::PADDING_FILLED,
            Variant::Outlined => anatomy::button::PADDING_OUTLINED,
            Variant::Text => anatomy::button::PADDING_TEXT,
        }
    }
}

/// A labelled button. Builder form (Principle VIII):
/// `Button::filled("Create", roles).on_press(Message::Create).into()`.
///
/// Without an `on_press` the button renders disabled, matching the rendering stack's own rule —
/// so "this action is unavailable" is expressed by having no message to send, not by a flag that
/// could disagree with one.
pub struct Button<'a, M> {
    content: Element<'a, M>,
    variant: Variant,
    roles: Roles,
    on_press: Option<M>,
    padding: Option<Padding>,
    width: Option<Length>,
    shape: Option<f32>,
    density: i8,
    leading: Option<(Icon, Option<Rgb>)>,
    trailing: Option<Icon>,
    trailing_mark: Option<(usize, String)>,
    tooltip: Option<String>,
    mark: Option<super::UnreadMark<'a, M>>,
    selected: bool,
    host: Option<style::Host>,
    id: Option<iced::advanced::widget::Id>,
}

impl<'a, M: Clone + 'a> Button<'a, M> {
    /// The primary action, carrying `label` at the body role.
    pub fn filled(label: impl Into<String>, roles: Roles) -> Self {
        Self::labelled(label, Variant::Filled, roles)
    }

    /// A secondary action of equal standing, carrying `label` at the body role.
    pub fn outlined(label: impl Into<String>, roles: Roles) -> Self {
        Self::labelled(label, Variant::Outlined, roles)
    }

    /// A low-emphasis action, carrying `label` at the body role.
    pub fn text(label: impl Into<String>, roles: Roles) -> Self {
        Self::labelled(label, Variant::Text, roles)
    }

    /// A button wrapping arbitrary `content` — a row of icon plus label, a tag chip, a tree row.
    /// The variant still decides the appearance; only what sits inside differs.
    pub fn with_content(
        content: impl Into<Element<'a, M>>,
        variant: Variant,
        roles: Roles,
    ) -> Self {
        Self {
            content: content.into(),
            variant,
            roles,
            on_press: None,
            padding: None,
            leading: None,
            trailing: None,
            trailing_mark: None,
            tooltip: None,
            mark: None,
            selected: false,
            width: None,
            shape: None,
            density: micold_core::tokens::density::STANDARD,
            host: None,
            id: None,
        }
    }

    fn labelled(label: impl Into<String>, variant: Variant, roles: Roles) -> Self {
        let label: Element<'a, M> = Text::new(label.into(), TypeRole::Action, roles).into();
        Self::with_content(label, variant, roles)
    }

    /// Name the button so the application can move the keyboard to it with `focus(id)`, as when a
    /// dialog closes and focus returns to the control that opened it (feature 1092, FR-009).
    pub fn id(mut self, id: impl Into<iced::advanced::widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// The message emitted on press. Omit it and the button is disabled.
    pub fn on_press(mut self, message: M) -> Self {
        self.on_press = Some(message);
        self
    }

    /// The press message from an `Option` — disabled when `None`. For an action that is available
    /// only in some states, so the call site expresses the condition once.
    pub fn on_press_maybe(mut self, message: Option<M>) -> Self {
        self.on_press = message;
        self
    }

    /// Override the button's padding.
    ///
    /// **A parity affordance, not a design decision.** Today's call sites use four different
    /// paddings for the same variant, and reproducing that exactly is what makes this feature
    /// reviewable. Feature 018 assigns each variant a height from the density scale and this step
    /// goes away with the last caller.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Lay the button out at a step of the density scale (`density::DENSE` for a compact one); the
    /// default is `density::STANDARD`, the 40dp of §7.3. The label and glyph keep their size and
    /// look; only the height changes.
    pub fn density(mut self, step: i8) -> Self {
        self.density = step;
        self
    }

    /// Lay the button out at a given width — `Length::Fill` for a full-width list row.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Override the corner radius of this button's **state layer** — the shape it draws on hover
    /// and press, and nothing else (feature 026 FR-015).
    ///
    /// Every button is a pill by default, which is right for a button and wrong for a tab: a tab's
    /// highlight is rectangular and spans the tab, and one built as a `Variant::Text` inherits the
    /// pill from two places at once — this variant's own `shape::FULL` border radius under the
    /// hover and press fill, and the `shape::FULL` the ripple is clipped to. Both are set from this
    /// one value, so a caller cannot change half of it and leave a rounded ripple inside a square
    /// fill.
    ///
    /// A step rather than a variant, because it is not a difference of emphasis: `Variant::Text` is
    /// exactly the right variant for a tab — no container, the accent for its content — and the
    /// only thing a tab disagrees with is the corner.
    pub fn shape(mut self, radius: f32) -> Self {
        self.shape = Some(radius);
        self
    }

    /// A leading icon before the label, drawn at §7.3's [`anatomy::button::LEADING_ICON`].
    ///
    /// The slot belongs to the component because the figure does. Two call sites built
    /// `row![Glyph::new(icon, TypeRole::Action, r), Text::new(..)]` by hand, which sized the glyph
    /// to the *label's* 14dp — the same shape as the icon button's own glyph, and as the menu item
    /// BUG-003's T103 found. §7.3 gives a leading icon 18dp: smaller than an icon button's 24,
    /// because it is an accent to a label rather than the whole content.
    ///
    /// The tint comes from the **variant**, like the label's does — `on_primary` on a filled
    /// button, the accent on an outlined or text one. A leading icon is part of a button's content,
    /// and content is one colour.
    ///
    /// It used to be a required argument, on the reasoning that "inferring it here would change what
    /// they draw today". It would have, and that was the defect: three outlined buttons passed
    /// `on_surface` beside a label the variant draws in `primary`, so each was one control in two
    /// colours. No geometry gate could see it — the glyph was 18dp, in the leading slot, in the
    /// wrong tone — and it was found by looking at the running application (018's BUG-007, T139).
    ///
    /// For the rare glyph whose colour *means* something on its own, use
    /// [`leading_tinted`](Self::leading_tinted).
    pub fn leading(mut self, glyph: Icon) -> Self {
        self.leading = Some((glyph, None));
        self
    }

    /// A trailing glyph after the label, at the leading slot's [`anatomy::button::LEADING_ICON`]
    /// size and in the variant's content colour (feature 1127, R4): the drop-down glyph of the
    /// title bar's project segment, saying the control opens a list. It sits between the label and
    /// any mark, whichever builder step came first, and never shortens.
    pub fn trailing(mut self, glyph: Icon) -> Self {
        self.trailing = Some(glyph);
        self
    }

    /// A hover tooltip over the whole button, `tooltip` its first line (feature 1127, contract
    /// C2). With a [`trailing_mark`](Self::trailing_mark) the mark's own words follow on the next
    /// line, so the button keeps one tooltip rather than two stacked over each other.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// The unread mark and `unread` after the label, inside the button: `● 3` (feature 039,
    /// FR-023; contract `unread-mark.md`). `tooltip` says what the number counts and is shown on
    /// hover of the whole button. A count of zero draws no mark and no tooltip.
    ///
    /// The number takes the label's role and the variant's content colour, so it reads as part of
    /// the label; the mark before it is what tells it from the name. For the text and outlined
    /// variants: the mark is filled in `primary`, which is the filled variant's own container.
    pub fn trailing_mark(mut self, unread: usize, tooltip: impl Into<String>) -> Self {
        self.trailing_mark = (unread > 0).then(|| (unread, tooltip.into()));
        self
    }

    /// A mark the caller has configured, after the label inside the button: the Overview's
    /// badge, tinted and worded by its host (feature 813, FR-012). Unlike
    /// [`trailing_mark`](Self::trailing_mark) it is drawn exactly as given. It adds no height.
    pub fn mark(mut self, mark: super::UnreadMark<'a, M>) -> Self {
        self.mark = Some(mark);
        self
    }

    /// Draw the button toggled on: the selected state layer over its content colour (feature 813,
    /// FR-012). A layer and not a size, so toggling moves nothing.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// The style the rendering stack's button is given: the variant's own, with the corner
    /// overridden if the caller asked, and the selected layer where it is toggled on.
    fn style_fn(&self) -> ButtonStyleFn {
        // A decorator rather than a parameter threaded through `style.rs`: the radius and the
        // selected layer are the only things an override touches, and every other call site keeps
        // the style function it has always had.
        let base = self.variant.style(self.roles, self.host);
        let (radius, selected) = (self.shape, self.selected);
        let layer = style::color(self.variant.content(self.roles, self.host));
        Box::new(move |theme, status| {
            let mut style = base(theme, status);
            if let Some(radius) = radius {
                style.border.radius = radius.into();
            }
            // Pressed keeps its own, heavier layer; every other status of a toggled button shows
            // the selected one.
            if selected && !matches!(status, button::Status::Pressed | button::Status::Disabled) {
                style.background = Some(iced::Background::Color(style::state_fill(
                    layer,
                    micold_core::tokens::state::SELECTED,
                )));
            }
            style
        })
    }

    /// A leading icon in a stated tint, for a glyph carrying its own meaning — the destructive
    /// `error` red on "Forget", which is saying something the label's accent does not.
    ///
    /// Deliberately the longer name: an override should read as one at the call site, so that the
    /// ordinary case cannot be written by accident.
    pub fn leading_tinted(mut self, glyph: Icon, tint: Rgb) -> Self {
        self.leading = Some((glyph, Some(tint)));
        self
    }

    /// What this button is standing on (FR-004a, FR-027b; contract §7.3 "Host surface").
    ///
    /// §7.3's `Container` and `Label / icon` rows describe a button on a **neutral surface**. On an
    /// accent-filled one — a banner's `error`, a filled tag, a snackbar's inverse surface — they do
    /// not hold: `primary` is 1.00:1 on `error` in the light scheme and 1.01:1 in the dark, because
    /// both roles read their ramps at the tone their scheme assigns and two roles at the same tone
    /// have the same luminance by construction. The host's own paired foreground is what reads
    /// there, and it is the component's business rather than the call site's, so the label, the
    /// border, the state layer, the leading glyph and the ripple all move together — the fix for
    /// one control in two colours cannot be four matching edits.
    ///
    /// A [`Host`](style::Host) that is **not** an accent fill imposes nothing, so a call site may
    /// hand over whatever it stands on without first deciding whether it matters. Saying nothing at
    /// all keeps §7.3's table exactly as it is, which is what makes this a parameter rather than a
    /// migration.
    pub fn on_host(mut self, host: style::Host) -> Self {
        self.host = Some(host);
        self
    }
}

impl<'a, M: Clone + 'a> From<Button<'a, M>> for Element<'a, M> {
    fn from(b: Button<'a, M>) -> Self {
        // §7.3's 40dp, the first row of the variant table and the same for all three. It was stated
        // in this file's own documentation ("Feature 018 assigns each variant a height from the
        // density scale") and applied nowhere: `density::BUTTON_BASE` was referenced by no call
        // site, and a filled button laid out at 30dp — its label plus the rendering stack's default
        // padding. Found by `anatomy_size.rs`, which is the check BUG-002 added for exactly this
        // shape: a figure that is right in the token module and never reaches the component.
        //
        // The centring wrapper is not optional, and FR-030a is why. Fixing a height above the
        // content's creates slack, and `button` lays its content out under `limits.height(Fixed)`,
        // which sets the minimum with the maximum — so the content node is stretched to 40dp and
        // draws at the top of it unless something says otherwise. That was BUG-001 exactly, one
        // component over. A wrapper rather than the content's own `align_y` because the content is
        // an arbitrary `Element` here, not a `Text` this type can reach into.
        //
        // A leading icon joins the label here rather than at the call site, so §7.3's 18dp is the
        // component's business — see [`Button::leading`].
        //
        // The unread count joins it here too, for the same reason: the gap before it and the role
        // and colour of its number are the component's, not the app bar's.
        let style_fn = b.style_fn();
        let (mark, tooltip) = match b.trailing_mark {
            Some((unread, tooltip)) => (
                Some(
                    super::UnreadMark::new(b.roles)
                        .count(unread)
                        .role(TypeRole::Action)
                        .tint(b.variant.content(b.roles, b.host)),
                ),
                Some(tooltip),
            ),
            None => (b.mark, None),
        };
        // The caller's tooltip first, then what the mark counts (contract C2).
        let tooltip = match (b.tooltip, tooltip) {
            (Some(own), Some(marked)) => Some(format!("{own}\n{marked}")),
            (own, marked) => own.or(marked),
        };
        let inner: Element<'a, M> = if b.leading.is_none() && b.trailing.is_none() && mark.is_none()
        {
            b.content
        } else {
            // A give-way lane rather than a `row!` (feature 1127): in a narrowed button the label
            // is what shortens, and the glyph and the mark keep their width. A row lays its
            // children out in order, so the label took the whole room and left the mark none.
            let kept = || Slot {
                give_way: GiveWay::Keep,
                group: Group::Leading,
                floor: Floor::Natural,
            };
            let mut inner = Lane::hugging(spacing::XS);
            if let Some((glyph, tint)) = b.leading {
                // The variant's own content colour unless the call site meant something by
                // the glyph's tone — one control, one colour, by default.
                let tint = tint.unwrap_or_else(|| b.variant.content(b.roles, b.host));
                inner = inner.push(icon(glyph, anatomy::button::LEADING_ICON, tint), kept());
            }
            let label = Slot {
                give_way: GiveWay::Shorten(0),
                group: Group::Leading,
                floor: Floor::Squeezed,
            };
            inner = inner.push(b.content, label);
            if let Some(glyph) = b.trailing {
                let tint = b.variant.content(b.roles, b.host);
                inner = inner.push(icon(glyph, anatomy::button::LEADING_ICON, tint), kept());
            }
            if let Some(mark) = mark {
                // One more `XS` before the mark than between the glyph and the label: the glyph
                // belongs to the name, and the count is a second thing said after it.
                let mark = container(mark).padding(Padding {
                    left: spacing::XS,
                    ..Padding::ZERO
                });
                inner = inner.push(mark.into(), kept());
            }
            inner.into()
        };
        let content = container(inner)
            .height(Length::Fill)
            .align_y(Alignment::Center);
        let mut widget = button(content)
            .height(Length::Fixed(density::height(density::BUTTON_BASE, b.density)))
            .style(style_fn);
        // §7.3's horizontal padding, from the variant table. Vertical padding is zero because the
        // height above is what makes the button 40dp — padding on this axis would add to a figure
        // the contract fixes, and the centring wrapper is what places the content inside it.
        //
        // The caller's override still wins: today's list rows and terminal controls are `Button`s
        // whose inset belongs to the row they sit in, not to §7.3.
        widget = widget.padding(b.padding.unwrap_or(Padding {
            top: 0.0,
            bottom: 0.0,
            left: b.variant.padding(),
            right: b.variant.padding(),
        }));
        if let Some(width) = b.width {
            widget = widget.width(width);
        }
        let focusable = pressable(
            widget,
            b.on_press,
            b.variant.content(b.roles, b.host),
            // The ripple and the focus ring take the fill's corner, or a pill when nobody said
            // otherwise.
            b.shape.unwrap_or(shape::FULL),
            b.roles.secondary,
            b.id,
        );
        // Outermost, so the whole button is the tooltip's trigger. It adds no layout node.
        match tooltip {
            Some(tooltip) => super::Tooltip::new(focusable, tooltip, b.roles).into(),
            None => focusable,
        }
    }
}

/// What makes a rendering-stack button one of the library's: a ripple and the keyboard, both only
/// while it can be pressed (FR-024c, FR-030). Shared by [`Button`] and
/// [`FieldButton`](crate::FieldButton), so a pressable control cannot have one without the other.
///
/// The ripple is in `tint`, the content colour (a state layer is the content colour), clipped to
/// `radius`: a wrapper cannot see the shape its child draws, so the corner comes from the same value
/// as the fill's. A disabled control gets no ripple: one would report a press that will never
/// happen, the opposite of what its disabled styling says.
///
/// The keyboard wrapper sits outside the ripple, so the focus indicator is drawn over the state
/// layer and not under it. Enter and Space both press, as a button answers everywhere it exists
/// (WAI-ARIA names both); a *focused* button answering Enter takes nothing from a dialog, since the
/// field whose `on_submit` saves the form does not hold the keyboard while this does. §5's ring is
/// `ring` against every variant; the layer under it is `tint`. `id` names it for `focus(id)`.
pub(crate) fn pressable<'a, M: Clone + 'a>(
    mut widget: button::Button<'a, M>,
    on_press: Option<M>,
    tint: Rgb,
    radius: f32,
    ring: Rgb,
    id: Option<iced::advanced::widget::Id>,
) -> Element<'a, M> {
    let pressed: Element<'a, M> = match on_press.clone() {
        Some(message) => {
            widget = widget.on_press(message);
            super::Ripple::new(widget, tint, radius).into()
        }
        None => widget.into(),
    };
    let mut focusable = TakesTheKeyboard::new(pressed, on_press.is_some());
    if let Some(id) = id {
        focusable = focusable.id(id);
    }
    if let Some(message) = on_press {
        focusable = focusable
            .key(keyboard::key::Named::Enter, message.clone())
            .key(keyboard::key::Named::Space, message);
    }
    focusable
        .indicator(Indicator {
            outline: ring,
            layer: tint,
            radius,
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::super::test_support::roles;
    use super::*;
    use crate::test_support::Message;
    use crate::UnreadMark;
    use iced::advanced::layout::{self, Layout};
    use iced::advanced::widget::Tree;
    use iced::{Rectangle, Size};

    const ROOM: Size = Size::new(400.0, 300.0);

    /// The switcher's button as the app bar builds it, with `unread` sessions in other projects.
    fn switcher_button(unread: usize) -> Element<'static, Message> {
        Button::text("micold-ai-ide", roles())
            .leading(Icon::OpenProject)
            .trailing_mark(unread, format!("{unread} unread"))
            .on_press(Message::NoOp)
            .into()
    }

    /// The button's own box, and the boxes of what its content row holds, in order.
    fn parts(element: Element<'_, Message>) -> (Rectangle, Vec<Rectangle>) {
        let mut element = element;
        let renderer = crate::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, ROOM),
        );
        let button = Layout::new(&node);
        let row = button
            .children()
            .next()
            .expect("the button holds its centring container")
            .children()
            .next()
            .expect("the container holds the content row");
        let content: Vec<Rectangle> = row.children().map(|child| child.bounds()).collect();
        let bounds = button.bounds();
        (bounds, content)
    }

    /// U156 (FR-023): `● n` after the label, inside the button.
    #[test]
    fn a_trailing_mark_renders_the_count_after_the_label_inside_the_button() {
        let (button, content) = parts(switcher_button(3));

        let [_glyph, label, mark] = content[..] else {
            panic!(
                "a button with a leading icon and an unread count holds three boxes, not {}",
                content.len()
            );
        };
        assert!(
            mark.x >= label.x + label.width,
            "the mark starts at {}dp, before the label ends at {}dp",
            mark.x,
            label.x + label.width
        );
        assert!(
            mark.width > 8.0,
            "the mark is {}dp wide: the dot alone, with no number after it",
            mark.width
        );
        let inset = (button.x + button.width) - (mark.x + mark.width);
        assert!(
            (inset - anatomy::button::PADDING_TEXT).abs() < 0.5,
            "the mark ends {inset}dp inside the button's trailing edge, not at its {}dp padding",
            anatomy::button::PADDING_TEXT
        );
    }

    /// Feature 1127 (contract B4): in a narrowed button the label shortens and the glyph and the
    /// mark keep their place inside it — a row gave the label the whole room and the mark none.
    #[test]
    fn a_narrowed_button_shortens_its_label_and_keeps_its_mark() {
        use super::super::test_support::{leaves, Mounted};
        let long = "a-project-with-a-name-sixty-characters-long-for-the-title-ba";
        let label = crate::Ellipsized::at_role(long, TypeRole::Action, roles().primary).shrink();
        let button = Button::<String>::with_content(label, Variant::Text, roles())
            .leading(Icon::OpenProject)
            .trailing_mark(3, "3 unread")
            .on_press("go".to_string());
        let room = Size::new(200.0, 100.0);
        let mut m = Mounted::new_in(button, room);
        let outer = m.node.bounds();
        assert!(outer.width <= room.width, "{outer:?}");
        for leaf in leaves(&m.node) {
            assert!(
                leaf.x + leaf.width <= outer.x + outer.width + 0.5,
                "{leaf:?} leaves {outer:?}"
            );
        }
        let painted = m.painted_text();
        let words: Vec<&str> = painted.iter().map(|p| p.content.as_str()).collect();
        assert!(words.contains(&"3"), "the mark's count is drawn: {words:?}");
        let label = painted
            .iter()
            .find(|p| p.content.ends_with('…'))
            .unwrap_or_else(|| panic!("the label shortens: {words:?}"));
        assert!(
            label.bounds.width > 40.0,
            "the label keeps the room left beside the glyph and the mark: {:?}",
            label.bounds
        );
    }

    /// T023 (feature 1127, R4): a trailing glyph sits after the label at the leading slot's size,
    /// and before the mark whichever builder step came first.
    #[test]
    fn a_trailing_glyph_follows_the_label_before_the_mark_in_either_order() {
        let glyph_first = Button::text("micold-ai-ide", roles())
            .trailing(Icon::SelectChevron)
            .trailing_mark(3, "3 unread")
            .on_press(Message::NoOp);
        let mark_first = Button::text("micold-ai-ide", roles())
            .trailing_mark(3, "3 unread")
            .trailing(Icon::SelectChevron)
            .on_press(Message::NoOp);
        let (one, first) = parts(glyph_first.into());
        let (two, second) = parts(mark_first.into());
        assert_eq!(
            (one, &first),
            (two, &second),
            "the builder order changes nothing"
        );
        let [label, glyph, mark] = first[..] else {
            panic!("label, glyph and mark are three boxes, not {}", first.len());
        };
        assert!(
            glyph.x >= label.x + label.width,
            "the glyph follows the label"
        );
        assert!(
            mark.x >= glyph.x + glyph.width,
            "the mark follows the glyph"
        );
        assert_eq!(
            glyph.width,
            anatomy::button::LEADING_ICON,
            "the glyph is the leading slot's size"
        );
    }

    /// T023: without a mark the trailing glyph is the button's last part, `PADDING_TEXT` inside its
    /// trailing edge, and the button grows by it.
    #[test]
    fn a_trailing_glyph_alone_ends_the_button() {
        let (plain, _) = parts(
            Button::text("Projects", roles())
                .on_press(Message::NoOp)
                .into(),
        );
        let (with, content) = parts(
            Button::text("Projects", roles())
                .trailing(Icon::SelectChevron)
                .on_press(Message::NoOp)
                .into(),
        );
        let [_label, glyph] = content[..] else {
            panic!("label and glyph are two boxes, not {}", content.len());
        };
        let inset = (with.x + with.width) - (glyph.x + glyph.width);
        assert!(
            (inset - anatomy::button::PADDING_TEXT).abs() < 0.5,
            "{inset}"
        );
        assert!(with.width > plain.width + anatomy::button::LEADING_ICON - 0.5);
    }

    /// Contract C2: a button's own tooltip and its mark's words are one tooltip, own line first.
    #[test]
    fn a_buttons_tooltip_and_its_marks_words_share_one_tooltip() {
        use super::super::test_support::Mounted;
        let button = Button::<String>::text("micold-ai-ide", roles())
            .tooltip("Switch project")
            .trailing_mark(3, "3 unread elsewhere")
            .on_press("go".to_string());
        let mut m = Mounted::new_in(button, ROOM);
        let centre = m.node.bounds().center();
        let floated = m.floated_text(centre).join("\n");
        assert!(
            floated.contains("Switch project") && floated.contains("3 unread elsewhere"),
            "{floated:?}"
        );
        assert!(
            floated.find("Switch project") < floated.find("3 unread"),
            "{floated:?}"
        );
    }

    /// U156, FR-023: no unread count when the total is zero.
    #[test]
    fn a_trailing_mark_of_zero_renders_the_button_without_it() {
        let (marked, content) = parts(switcher_button(0));
        let (plain, _) = parts(
            Button::text("micold-ai-ide", roles())
                .leading(Icon::OpenProject)
                .on_press(Message::NoOp)
                .into(),
        );

        assert_eq!(content.len(), 2, "the leading icon and the label, no mark");
        assert_eq!(
            marked.width, plain.width,
            "a total of zero leaves no gap where the mark would stand"
        );
    }

    /// T033 (FR-012): a toggled button draws the selected state layer and keeps its geometry.
    #[test]
    fn a_selected_button_draws_the_selected_layer_and_keeps_its_geometry() {
        let plain = || Button::<Message>::text("Overview", roles()).leading(Icon::Overview);
        let draw = |b: Button<'_, Message>, status| b.style_fn()(&iced::Theme::Light, status);

        let resting = draw(plain(), button::Status::Active);
        let selected = draw(plain().selected(true), button::Status::Active);
        let unselected = draw(plain().selected(false), button::Status::Active);

        assert_eq!(
            resting.background, None,
            "a resting text button has no layer"
        );
        assert_eq!(unselected.background, resting.background);
        let Some(iced::Background::Color(layer)) = selected.background else {
            panic!(
                "a selected button draws a layer, not {:?}",
                selected.background
            );
        };
        assert!(
            (layer.a - micold_core::tokens::state::SELECTED).abs() < 1e-6,
            "the layer is the selected opacity, not {}",
            layer.a
        );
        assert_eq!(
            parts(plain().on_press(Message::NoOp).into()).0,
            parts(plain().selected(true).on_press(Message::NoOp).into()).0,
            "toggling changes no size"
        );
    }

    /// T033 (FR-012): a configured mark is placed trailing and the button keeps its height.
    #[test]
    fn a_configured_mark_sits_trailing_without_changing_the_height() {
        let plain = || {
            Button::<Message>::text("Overview", roles())
                .leading(Icon::Overview)
                .on_press(Message::NoOp)
        };
        let (bare, _) = parts(plain().into());
        let (marked, content) = parts(
            plain()
                .mark(
                    UnreadMark::new(roles())
                        .icon(Icon::CardNeedsAction)
                        .count(2)
                        .role(TypeRole::Action),
                )
                .into(),
        );

        let [_glyph, label, mark] = content[..] else {
            panic!(
                "icon, label and mark are three boxes, not {}",
                content.len()
            );
        };
        assert!(mark.x >= label.x + label.width, "the mark trails the label");
        let inset = (marked.x + marked.width) - (mark.x + mark.width);
        assert!(
            (inset - anatomy::button::PADDING_TEXT).abs() < 0.5,
            "the mark ends {inset}dp inside the trailing edge"
        );
        assert_eq!(marked.height, bare.height, "the mark adds no height");
        assert!(marked.width > bare.width, "the mark adds width");
    }
}
