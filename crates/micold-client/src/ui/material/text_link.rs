//! `TextLink` — a line of text that opens something when clicked (feature 1122, contract U1).
//!
//! Drawn in the theme's `primary`, with a 1 px underline in the same colour while the pointer is on
//! it and the pointing-hand pointer; the focus ring is [`TakesTheKeyboard`]'s, in `secondary`, and
//! only for keyboard focus (FR-015, Principle VIII). A click is a press and a release both on it;
//! Enter and Space activate it while it holds the keyboard.
//!
//! Two switches serve a link sitting on a card that is itself a control. `.focus_on_press(false)`
//! keeps a click from taking the keyboard, so the focus stays where it was (research R7).
//! `.focusable(false)` keeps it out of the Tab order altogether while a click still opens it, for a
//! card whose link focus is not wired yet (research R5).

use crate::ui::cdk::passthrough::{forward_to_child, forward_to_selected};
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{layout, mouse, renderer, Clipboard, Layout, Renderer as _, Shell};
use iced::{keyboard, touch, Background, Element, Event, Rectangle};
use micold_core::tokens::{state, Roles};

use super::keyboard_focus::{Indicator, TakesTheKeyboard};
use super::{style, Text, TypeRole};

/// A text link. See the module documentation.
pub struct TextLink<'a, M> {
    label: String,
    on_press: M,
    roles: Roles,
    role: TypeRole,
    id: Option<iced::advanced::widget::Id>,
    on_keyboard_focus: Option<Box<dyn Fn(bool) -> M + 'a>>,
    on_menu_key: Option<M>,
    on_page: Option<Box<dyn Fn(bool) -> Option<M> + 'a>>,
    focus_on_press: bool,
    focusable: bool,
    inline: bool,
    hit_on_label: bool,
}

impl<'a, M: Clone + 'a> TextLink<'a, M> {
    /// A link reading `label` that publishes `on_press` when clicked or activated by key.
    pub fn new(label: impl Into<String>, on_press: M, roles: Roles) -> Self {
        Self {
            label: label.into(),
            on_press,
            roles,
            role: TypeRole::Body,
            id: None,
            on_keyboard_focus: None,
            on_menu_key: None,
            on_page: None,
            focus_on_press: true,
            focusable: true,
            inline: false,
            hit_on_label: false,
        }
    }

    /// The type role the label is set in; [`TypeRole::Body`] unless said otherwise.
    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = role;
        self
    }

    /// Name the link for `focus(id)` and for finding its bounds.
    pub fn id(mut self, id: iced::advanced::widget::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// The message published when keyboard focus arrives on the link (`true`) or leaves it
    /// (`false`). A click never shows focus, so it reports nothing.
    pub fn on_keyboard_focus(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_keyboard_focus = Some(Box::new(f));
        self
    }

    /// The message the Menu key and Shift+F10 publish while the link holds the keyboard.
    pub fn on_menu_key(mut self, message: M) -> Self {
        self.on_menu_key = Some(message);
        self
    }

    /// What Page Up (`false`) and Page Down (`true`) publish while the link holds the keyboard;
    /// `None` claims no key.
    pub fn on_page(mut self, f: impl Fn(bool) -> Option<M> + 'a) -> Self {
        self.on_page = Some(Box::new(f));
        self
    }

    /// Whether a click takes the keyboard; `true` unless said otherwise.
    pub fn focus_on_press(mut self, takes: bool) -> Self {
        self.focus_on_press = takes;
        self
    }

    /// Whether the link is a Tab stop and answers keys; `true` unless said otherwise. A link that
    /// is not still opens on a click.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// Lay the link out at its label's own box, its gutter spilling over its neighbours, so a link
    /// set in a line of text keeps the line's height and lines up with the text beside and above
    /// it; `false` unless said otherwise. The gutter still takes the pointer and holds the focus
    /// ring, so the neighbours need that much room to spare (feature 1122, the card's branch line).
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }

    /// Take the pointer, a click and a rest on the label's own box only, the gutter round it
    /// keeping its room and the focus ring but leaving the pointer to what it spills over: for
    /// links set closer together, or to the line above, than two gutters (feature 1123, a card's
    /// pull request numbers, review A M2 r1 F2); `false` unless said otherwise.
    pub fn hit_on_label(mut self, label_only: bool) -> Self {
        self.hit_on_label = label_only;
        self
    }
}

impl<'a, M: Clone + 'a> From<TextLink<'a, M>> for Element<'a, M> {
    fn from(link: TextLink<'a, M>) -> Self {
        use keyboard::key::Named;
        let roles = link.roles;
        // The click is the rendering stack's button's: a press and a release both on it, each
        // captured, and the redraw when the pointer comes and goes, which draws the underline.
        let click =
            iced::widget::button(Text::new(link.label, link.role, roles).tint(roles.primary))
                .padding(GUTTER)
                .style(|_: &iced::Theme, _| iced::widget::button::Style::default())
                .on_press(link.on_press.clone());
        let body = LinkBody {
            content: click.into(),
            underline: roles.primary,
            role: link.role,
            id: link.id.clone(),
            hit_on_label: link.hit_on_label,
        };
        let mut focus = TakesTheKeyboard::new(Element::new(body), link.focusable)
            .key(Named::Enter, link.on_press.clone())
            .key(Named::Space, link.on_press)
            .press_takes_focus(link.focus_on_press)
            .bare_child()
            .indicator(Indicator {
                outline: roles.secondary,
                layer: roles.primary,
                radius: state::FOCUS_RING_WIDTH,
            });
        if let Some(id) = link.id {
            focus = focus.id(id);
        }
        if let Some(menu) = link.on_menu_key {
            focus = focus.key(Named::ContextMenu, menu.clone()).chord(
                Named::F10,
                keyboard::Modifiers::SHIFT,
                menu,
            );
        }
        if let Some(on_page) = link.on_page {
            for (key, down) in [(Named::PageUp, false), (Named::PageDown, true)] {
                // Unmodified only, as the card's: Ctrl/Cmd+Shift+Page keys are the project chords.
                if let Some(message) = on_page(down) {
                    focus = focus.chord(key, keyboard::Modifiers::empty(), message);
                }
            }
        }
        if let Some(on_keyboard_focus) = link.on_keyboard_focus {
            focus = focus.on_shown_change(on_keyboard_focus);
        }
        if link.inline {
            Element::new(Spill {
                content: focus.into(),
            })
        } else {
            focus.into()
        }
    }
}

/// [`TextLink::inline`]: the link laid out [`GUTTER`] larger on every side than the box it reports,
/// and drawn, hit and focused at that larger box.
struct Spill<'a, M> {
    content: Element<'a, M>,
}

impl<'a, M> Spill<'a, M> {
    /// The link's own layout, the one child.
    fn spilled<'l>(&self, _tree: &Tree, layout: Layout<'l>) -> Option<(usize, Layout<'l>)> {
        layout.children().next().map(|child| (0, child))
    }

    fn child(&self, _i: usize) -> &Element<'a, M> {
        &self.content
    }

    fn child_mut(&mut self, _i: usize) -> &mut Element<'a, M> {
        &mut self.content
    }
}

impl<'a, M> Widget<M, iced::Theme, iced::Renderer> for Spill<'a, M> {
    forward_to_child!(content, M, iced::Theme, iced::Renderer; children, diff, size);
    forward_to_selected!(spilled, child, child_mut, M, iced::Theme, iced::Renderer;
        mouse_interaction, operate, overlay);

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let both = iced::Size::new(2.0 * GUTTER, 2.0 * GUTTER);
        let wider = layout::Limits::new(limits.min() + both, limits.max() + both);
        let link = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &wider);
        let size = iced::Size::new(
            (link.size().width - both.width).max(0.0),
            (link.size().height - both.height).max(0.0),
        );
        layout::Node::with_children(size, vec![link.move_to(iced::Point::new(-GUTTER, -GUTTER))])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        if let Some((_, link)) = self.spilled(tree, layout) {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                link,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style_: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some((_, link)) = self.spilled(tree, layout) {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style_,
                link,
                cursor,
                viewport,
            );
        }
    }
}

/// The drawn link: the button holding the label, padded on every side by a gutter wider than the
/// focus ring, and the underline under the label while the pointer is on it.
struct LinkBody<'a, M> {
    content: Element<'a, M>,
    underline: micold_core::tokens::Rgb,
    role: TypeRole,
    /// The link's id again, offered as a container: a link that is no Tab stop offers no
    /// focusable, and its bounds are still found by its id (feature 1122, R7).
    id: Option<iced::advanced::widget::Id>,
    /// [`TextLink::hit_on_label`].
    hit_on_label: bool,
}

impl<M> LinkBody<'_, M> {
    /// Where the link takes the pointer, a click and a rest, laid out in `layout`: its whole box,
    /// or the label's box inside the gutter with [`TextLink::hit_on_label`].
    fn hit(&self, layout: Layout<'_>) -> Rectangle {
        match layout.children().next() {
            Some(label) if self.hit_on_label => label.bounds(),
            _ => layout.bounds(),
        }
    }
}

/// The room on every side of the label, where the focus ring lands clear of the glyphs and of the
/// underline (visual pass M1 r3 F1). The material layer draws its rings inside the control:
/// `keyboard_focus::ring_bounds` insets the ring's quad by half its width, and iced strokes a
/// quad's border inside the quad, so the ring covers half a width to one and a half widths in
/// from the link's edge (1.5 px to 4.5 px). Twice the width puts the label 1.5 px clear of it, on
/// a whole pixel (review A M1 r5 F1).
pub(super) const GUTTER: f32 = 2.0 * state::FOCUS_RING_WIDTH;

/// The row of the 1 px underline under a label laid out in `text` at `size`: under the glyphs,
/// not at the foot of the line box. The label sits centred in its line, and its descenders end
/// about half a size below the centre. Not clamped to the label's box, which would lift it into
/// the glyphs under a tight line height; the gutter keeps it clear of the ring at every role
/// (`the_focus_ring_clears_the_label_on_every_side`).
fn underline_y(text: Rectangle, size: f32) -> f32 {
    (text.center_y() + size * 0.5).round()
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for LinkBody<'_, M> {
    forward_to_child!(content, M, iced::Theme, iced::Renderer;
        children, diff, size, layout);

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        operation.container(self.id.as_ref(), self.hit(layout));
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style_: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style_,
            layout,
            cursor,
            viewport,
        );
        if !cursor.is_over(self.hit(layout)) {
            return;
        }
        // The button's one child is the label, inside the padding.
        let Some(label) = layout.children().next() else {
            return;
        };
        let text = label.bounds();
        let y = underline_y(text, self.role.size());
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: text.x,
                    y,
                    width: text.width,
                    height: 1.0,
                },
                ..renderer::Quad::default()
            },
            Background::Color(style::color(self.underline)),
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        // A release off the link, by mouse or finger, ends its press without being captured
        // (U1): the button would capture it, so it is told the press was lost instead, which it
        // drops quietly.
        let bounds = self.hit(layout);
        // Off the hit box the button is told the pointer is away, so the gutter outside it takes
        // no press.
        let cursor = if cursor.is_over(bounds) {
            cursor
        } else {
            mouse::Cursor::Unavailable
        };
        let lost;
        let event = match *event {
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if !cursor.is_over(bounds) =>
            {
                lost = Event::Touch(touch::Event::FingerLost {
                    id: touch::Finger(0),
                    position: iced::Point::ORIGIN,
                });
                &lost
            }
            Event::Touch(touch::Event::FingerLifted { id, position })
                if !bounds.contains(position) =>
            {
                lost = Event::Touch(touch::Event::FingerLost { id, position });
                &lost
            }
            _ => event,
        };
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(self.hit(layout)) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::test_support::{key_with, named_key, roles, Focusables, Mounted};
    use iced::keyboard::{key::Named, Modifiers};
    use iced::widget::{column, container, text};
    use iced::Point;

    const AWAY: mouse::Cursor = mouse::Cursor::Unavailable;

    fn link() -> TextLink<'static, String> {
        TextLink::new("Issue #1122", "open".to_string(), roles())
    }

    /// The link first, then a plain block below it to press "off" the link.
    fn mounted(link: TextLink<'static, String>) -> Mounted<'static> {
        Mounted::new(column![
            Element::from(link),
            container(text("elsewhere")).padding(16)
        ])
    }

    fn on(m: &Mounted<'_>) -> Point {
        m.node.children()[0].bounds().center()
    }

    fn off(m: &Mounted<'_>) -> Point {
        m.node.children()[1].bounds().center()
    }

    fn release(m: &mut Mounted<'_>, at: Point) -> (Vec<String>, bool) {
        m.send_captured(
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        )
    }

    fn press(m: &mut Mounted<'_>, at: Point) -> (Vec<String>, bool) {
        m.send_captured(
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        )
    }

    /// `inline(true)`: the link reports its label's box, the gutter spilling round it, and still
    /// takes a click and the pointer across the whole gutter.
    #[test]
    fn an_inline_link_reports_its_labels_box_and_keeps_its_gutter() {
        let boxed = mounted(link());
        let mut inline = mounted(link().inline(true));
        let (outer, inner) = (
            boxed.node.children()[0].bounds(),
            inline.node.children()[0].bounds(),
        );
        assert_eq!(inner.width, outer.width - 2.0 * GUTTER);
        assert_eq!(inner.height, outer.height - 2.0 * GUTTER);
        assert_eq!(
            inline.node.children()[1].bounds().y,
            inner.height,
            "the next line starts at the label's foot"
        );
        let edge = Point::new(inner.x - GUTTER + 1.0, inner.y - GUTTER + 1.0);
        let pointer = inline.element.as_widget().mouse_interaction(
            &inline.tree,
            Layout::new(&inline.node),
            mouse::Cursor::Available(edge),
            &Rectangle::with_size(inline.room),
            &inline.renderer,
        );
        assert_eq!(pointer, mouse::Interaction::Pointer);
        press(&mut inline, edge);
        assert_eq!(release(&mut inline, edge).0, ["open"]);
    }

    /// `hit_on_label(true)` (review A 1123 M2 r1 F2): the gutter keeps its room but takes neither
    /// the pointer nor a click; the label still takes both.
    #[test]
    fn a_link_hit_on_its_label_leaves_its_gutter_to_its_neighbours() {
        let mut inline = mounted(link().inline(true).hit_on_label(true));
        let inner = inline.node.children()[0].bounds();
        let edge = Point::new(inner.x - GUTTER + 1.0, inner.y - GUTTER + 1.0);
        let pointer_at = |m: &Mounted<'_>, at: Point| {
            m.element.as_widget().mouse_interaction(
                &m.tree,
                Layout::new(&m.node),
                mouse::Cursor::Available(at),
                &Rectangle::with_size(m.room),
                &m.renderer,
            )
        };
        assert_eq!(pointer_at(&inline, edge), mouse::Interaction::None);
        assert!(
            !press(&mut inline, edge).1,
            "the press in the gutter was captured"
        );
        assert!(release(&mut inline, edge).0.is_empty());
        let label = Point::new(inner.x + 1.0, inner.y + 1.0);
        assert_eq!(pointer_at(&inline, label), mouse::Interaction::Pointer);
        press(&mut inline, label);
        assert_eq!(release(&mut inline, label).0, ["open"]);
    }

    #[test]
    fn a_press_and_release_on_it_publishes_once_and_both_are_captured() {
        let mut m = mounted(link());
        let at = on(&m);
        let (published, captured) = press(&mut m, at);
        assert!(
            published.is_empty(),
            "the press alone published {published:?}"
        );
        assert!(captured, "the press was not captured");
        assert_eq!(release(&mut m, at), (vec!["open".to_string()], true));
        assert!(
            release(&mut m, at).0.is_empty(),
            "a second release published"
        );
    }

    #[test]
    fn a_press_on_it_released_off_it_publishes_nothing() {
        let mut m = mounted(link());
        let (at, away) = (on(&m), off(&m));
        press(&mut m, at);
        assert_eq!(release(&mut m, away), (Vec::new(), false));
        assert!(
            release(&mut m, at).0.is_empty(),
            "the press outlived its release"
        );
    }

    #[test]
    fn a_press_off_it_released_on_it_publishes_nothing() {
        let mut m = mounted(link());
        let (at, away) = (on(&m), off(&m));
        press(&mut m, away);
        assert_eq!(release(&mut m, at), (Vec::new(), false));
    }

    #[test]
    fn enter_and_space_activate_it_only_while_it_has_the_keyboard() {
        let mut m = mounted(link());
        assert!(m.send(named_key(Named::Enter), AWAY).is_empty());
        assert!(m.send(named_key(Named::Space), AWAY).is_empty());
        m.focus_next();
        assert_eq!(m.send(named_key(Named::Enter), AWAY), ["open"]);
        assert_eq!(m.send(named_key(Named::Space), AWAY), ["open"]);
    }

    #[test]
    fn the_menu_key_and_shift_f10_publish_its_menu_message_while_focused() {
        let mut m = mounted(link().on_menu_key("menu".to_string()));
        m.focus_next();
        assert_eq!(m.send(named_key(Named::ContextMenu), AWAY), ["menu"]);
        assert_eq!(
            m.send(key_with(Named::F10, Modifiers::SHIFT), AWAY),
            ["menu"]
        );
    }

    #[test]
    fn page_up_and_down_publish_what_on_page_gives() {
        let mut m = mounted(link().on_page(|down| down.then(|| "page down".to_string())));
        m.focus_next();
        assert_eq!(m.send(named_key(Named::PageDown), AWAY), ["page down"]);
        assert!(m.send(named_key(Named::PageUp), AWAY).is_empty());
    }

    #[test]
    fn a_link_that_does_not_focus_on_press_is_left_without_the_keyboard() {
        let mut m = mounted(link().focus_on_press(false));
        let at = on(&m);
        press(&mut m, at);
        assert_eq!(release(&mut m, at).0, ["open"]);
        assert!(
            m.send(named_key(Named::Enter), AWAY).is_empty(),
            "the click gave the link the keyboard"
        );
    }

    #[test]
    fn a_link_that_is_not_focusable_is_skipped_by_tab_yet_opens_on_a_click() {
        let mut m = mounted(link().focusable(false));
        assert_eq!(m.drive(Focusables(0)), Some(0), "Tab can reach the link");
        m.focus_next();
        assert!(m.send(named_key(Named::Enter), AWAY).is_empty());
        assert!(m.send(named_key(Named::Space), AWAY).is_empty());
        let at = on(&m);
        press(&mut m, at);
        assert_eq!(release(&mut m, at).0, ["open"]);
    }

    #[test]
    fn the_pointer_is_a_pointing_hand_over_it_only() {
        let m = mounted(link());
        let interaction = |at: Point| {
            m.element.as_widget().mouse_interaction(
                &m.tree,
                Layout::new(&m.node),
                mouse::Cursor::Available(at),
                &Rectangle::with_size(m.room),
                &m.renderer,
            )
        };
        assert_eq!(interaction(on(&m)), mouse::Interaction::Pointer);
        assert_eq!(interaction(off(&m)), mouse::Interaction::None);
    }

    #[test]
    fn the_label_is_drawn_in_primary_at_its_role() {
        for (made, role) in [
            (link(), TypeRole::Body),
            (link().role(TypeRole::Caption), TypeRole::Caption),
        ] {
            let mut m = Mounted::new(made);
            let painted = m.painted_text();
            let label = painted
                .iter()
                .find(|p| p.content == "Issue #1122")
                .expect("the label is painted");
            assert_eq!(label.color, style::color(roles().primary));
            assert_eq!(label.size, role.size(), "{role:?}");
        }
    }

    /// Every pixel that differs between the two buffers is close to `colour`, and some do.
    fn only_changed_to(a: &[u8], b: &[u8], colour: iced::Color) -> bool {
        let want = colour.into_rgba8();
        let mut changed = 0;
        for (pa, pb) in a.chunks(4).zip(b.chunks(4)) {
            if pa != pb {
                changed += 1;
                if pb.iter().zip(want).take(3).any(|(x, y)| x.abs_diff(y) > 2) {
                    return false;
                }
            }
        }
        changed > 0
    }

    #[test]
    fn an_underline_in_the_link_colour_is_drawn_only_under_the_pointer() {
        let backdrop = style::color(roles().surface);
        let mut m = Mounted::new(link());
        let at = m.node.bounds().center();
        let rest = m.drawn(backdrop).0;
        let hovered = m.drawn_under(backdrop, mouse::Cursor::Available(at)).0;
        assert!(
            only_changed_to(&rest, &hovered, style::color(roles().primary)),
            "the pointer drew no primary underline, or drew something else"
        );
        let beside = Point::new(at.x, m.node.bounds().height + 50.0);
        assert_eq!(
            m.drawn_under(backdrop, mouse::Cursor::Available(beside)).0,
            rest,
            "an underline without the pointer on the link"
        );
    }

    #[test]
    fn the_focus_ring_is_drawn_only_for_keyboard_focus() {
        let backdrop = style::color(roles().surface);
        let mut m = mounted(link());
        let rest = m.drawn(backdrop).0;
        let at = on(&m);
        press(&mut m, at);
        release(&mut m, at);
        assert_eq!(m.drawn(backdrop).0, rest, "a click drew the ring");
        // A Tab after that click moves on past the link, so Tab onto a fresh one.
        let mut m = mounted(link());
        m.focus_next();
        let focused = m.drawn(backdrop).0;
        let ring = style::color(roles().secondary).into_rgba8();
        assert!(
            focused
                .chunks(4)
                .any(|p| p.iter().zip(ring).take(3).all(|(x, y)| x.abs_diff(y) <= 2)),
            "no secondary ring under keyboard focus"
        );
    }

    #[test]
    fn keyboard_focus_arriving_and_leaving_is_published() {
        let mut m = mounted(link().on_keyboard_focus(|f| format!("focus {f}")));
        m.focus_next();
        assert_eq!(m.redraw(), ["focus true"]);
        let away = off(&m);
        assert_eq!(press(&mut m, away).0, ["focus false"]);
    }

    /// Review A M1 F3: a finger lifted off the link publishes nothing and is not captured.
    #[test]
    fn a_finger_lifted_off_it_publishes_nothing_and_is_not_captured() {
        let mut m = mounted(link());
        let (at, away) = (on(&m), off(&m));
        let id = touch::Finger(1);
        m.send(
            Event::Touch(touch::Event::FingerPressed { id, position: at }),
            mouse::Cursor::Available(at),
        );
        assert_eq!(
            m.send_captured(
                Event::Touch(touch::Event::FingerLifted { id, position: away }),
                mouse::Cursor::Available(away),
            ),
            (Vec::new(), false)
        );
    }

    /// Review A M1 F2: the link's child draws no hover layer, so a Tab-focused link draws the same
    /// focus layer under the pointer as away from it; only the underline is added.
    #[test]
    fn the_focus_layer_is_the_same_under_the_pointer() {
        let backdrop = style::color(roles().surface);
        let mut m = Mounted::new(link());
        m.focus_next();
        let at = m.node.bounds().center();
        let away = m.drawn(backdrop).0;
        let hovered = m.drawn_under(backdrop, mouse::Cursor::Available(at)).0;
        // The ring sits in the gutter, so the underline still shows under keyboard focus.
        assert!(
            only_changed_to(&away, &hovered, style::color(roles().primary)),
            "the pointer drew no underline on the focused link, or changed its layer"
        );
    }

    /// Visual pass M1 r3 F1, review A M1 r5 F1 and F2: the focus ring lands in the gutter around
    /// the label, at least a pixel clear of the label's box and of the underline on every side,
    /// at every role, so it never paints over a glyph or hides the underline.
    #[test]
    fn the_focus_ring_clears_the_label_on_every_side() {
        use super::super::keyboard_focus::ring_bounds;
        let w = state::FOCUS_RING_WIDTH;
        for role in TypeRole::ALL {
            let m = Mounted::new(link().role(role));
            let link = m.node.bounds();
            // The label's node sits inside the button's, relative to it.
            let label = m.node.children()[0].bounds();
            let label = Rectangle {
                x: link.x + label.x,
                y: link.y + label.y,
                ..label
            };
            // iced strokes a quad's border inside the quad (tiny-skia offsets the stroke's path
            // by half the width; wgpu's SDF fills `dist + width`), so the ring's inner edge is
            // `ring_bounds` shrunk by the whole width.
            let ring = ring_bounds(link);
            let inner = Rectangle {
                x: ring.x + w,
                y: ring.y + w,
                width: ring.width - 2.0 * w,
                height: ring.height - 2.0 * w,
            };
            assert!(
                label.x - inner.x >= 1.0
                    && label.y - inner.y >= 1.0
                    && inner.x + inner.width - (label.x + label.width) >= 1.0
                    && inner.y + inner.height - (label.y + label.height) >= 1.0,
                "{role:?}: the ring's inner edge {inner:?} does not clear the label {label:?}"
            );
            let underline = underline_y(label, role.size());
            assert!(
                inner.y + inner.height - (underline + 1.0) >= 1.0,
                "{role:?}: the underline at {underline} reaches the ring's inner edge {inner:?}"
            );
            assert!(
                underline >= label.center_y() + role.size() * 0.4,
                "{role:?}: the underline at {underline} sits in the glyphs of {label:?}"
            );
        }
    }
}
