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

use crate::ui::cdk::passthrough::forward_to_child;
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{mouse, renderer, Clipboard, Layout, Renderer as _, Shell};
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
}

impl<'a, M: Clone + 'a> From<TextLink<'a, M>> for Element<'a, M> {
    fn from(link: TextLink<'a, M>) -> Self {
        use keyboard::key::Named;
        let roles = link.roles;
        // The click is the rendering stack's button's: a press and a release both on it, each
        // captured, and the redraw when the pointer comes and goes, which draws the underline.
        let click =
            iced::widget::button(Text::new(link.label, link.role, roles).tint(roles.primary))
                .padding(iced::Padding::ZERO.left(SIDE).right(SIDE))
                .style(|_: &iced::Theme, _| iced::widget::button::Style::default())
                .on_press(link.on_press.clone());
        let body = LinkBody {
            content: click.into(),
            underline: roles.primary,
            role: link.role,
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
                if let Some(message) = on_page(down) {
                    focus = focus.key(key, message);
                }
            }
        }
        if let Some(on_keyboard_focus) = link.on_keyboard_focus {
            focus = focus.on_shown_change(on_keyboard_focus);
        }
        focus.into()
    }
}

/// The drawn link: the button holding the label, padded on each side by the focus ring's width
/// so the ring clears the text, and the underline under the label while the pointer is on it.
struct LinkBody<'a, M> {
    content: Element<'a, M>,
    underline: micold_core::tokens::Rgb,
    role: TypeRole,
}

/// The room left of and right of the label.
const SIDE: f32 = state::FOCUS_RING_WIDTH;

impl<M> Widget<M, iced::Theme, iced::Renderer> for LinkBody<'_, M> {
    forward_to_child!(content, M, iced::Theme, iced::Renderer;
        children, diff, size, layout, operate);

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
        if !cursor.is_over(layout.bounds()) {
            return;
        }
        // The button's one child is the label, inside the padding.
        let Some(label) = layout.children().next() else {
            return;
        };
        let text = label.bounds();
        // Under the glyphs, not at the foot of the line box: the label sits centred in its line,
        // and its descenders end about half a size below the centre.
        let y = (text.center_y() + self.role.size() * 0.5).round();
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
        let bounds = layout.bounds();
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
        if cursor.is_over(layout.bounds()) {
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
        // The ring, drawn inside the link, may cover the underline's row, so nothing need change.
        assert!(
            hovered == away || only_changed_to(&away, &hovered, style::color(roles().primary)),
            "the pointer changed the focused link's layer, not only its underline"
        );
    }
}
