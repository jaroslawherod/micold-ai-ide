//! `SideSheet` — a panel stacked over the right of an area, covering it without moving it
//! (feature 1270, contract D2, D3, research R5): the comments drawer over the diff.
//!
//! A title, a × that closes it and the content under them, `review::drawer::width(area)` wide —
//! 360 px, never more than half the area. The sheet takes the whole area it is offered and lays its
//! panel out at the area's right edge, so it is drawn with `stack![area, sheet]` and the area's own
//! layout is computed as if the sheet were absent (FR-010: opening it does not change the diff's
//! width or move its lines). A press on the panel stays there; left of it the area gets it.
//!
//! Unlike `NavigationDrawer`, which pushes its sibling and leaves a rail, this covers. It is not an
//! overlay either: it is part of the view, under dialogs and menus.
//!
//! Builder form: `SideSheet::new(content, roles).title("Comments").on_close(Msg::Close).into()`.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::widget::{column, container, row, Space};
use iced::{Alignment, Background, Element, Event, Length, Point, Rectangle, Size, Vector};
use micold_core::review::drawer;
use micold_core::tokens::{spacing, Roles};

use super::icons::Icon;
use super::{style, IconButton, Text, TypeRole};

/// A panel over the right of the area it is given.
pub struct SideSheet<'a, M> {
    content: Element<'a, M>,
    roles: Roles,
    title: Option<String>,
    on_close: Option<M>,
}

impl<'a, M: Clone + 'a> SideSheet<'a, M> {
    /// A sheet holding `content`, themed by `roles`.
    pub fn new(content: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            content: content.into(),
            roles,
            title: None,
            on_close: None,
        }
    }

    /// The title in the sheet's head.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The message × sends; without it no × is drawn.
    pub fn on_close(mut self, message: M) -> Self {
        self.on_close = Some(message);
        self
    }
}

impl<'a, M: Clone + 'a> From<SideSheet<'a, M>> for Element<'a, M> {
    fn from(sheet: SideSheet<'a, M>) -> Self {
        let r = sheet.roles;
        let mut head = row![].spacing(spacing::SM).align_y(Alignment::Center);
        if let Some(title) = sheet.title {
            head = head.push(Text::new(title, TypeRole::Section, r));
        }
        head = head.push(Space::new().width(Length::Fill));
        if let Some(message) = sheet.on_close {
            head = head.push(IconButton::new(Icon::Close, r).compact().on_press(message));
        }
        let panel = container(
            column![
                container(head).padding([spacing::XS, spacing::MD]),
                container(sheet.content)
                    .width(Length::Fill)
                    .height(Length::Fill),
            ]
            .spacing(spacing::XS),
        )
        .padding(iced::Padding {
            left: 1.0,
            ..iced::Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(style::color(r.surface_container_low))),
            ..container::Style::default()
        });
        Element::new(Sheet {
            panel: panel.into(),
            roles: r,
        })
    }
}

/// The sheet's layout: the whole area, with the panel at its right edge.
struct Sheet<'a, M> {
    panel: Element<'a, M>,
    roles: Roles,
}

impl<M> Sheet<'_, M> {
    /// The panel's own layout, its only child.
    fn panel_layout(layout: Layout<'_>) -> Layout<'_> {
        layout
            .children()
            .next()
            .expect("the sheet lays out its panel")
    }
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for Sheet<'_, M> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.panel)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.panel));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let area = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let width = drawer::width(area.width);
        let fixed = layout::Limits::new(Size::ZERO, Size::new(width, area.height));
        let panel = self
            .panel
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &fixed)
            .move_to(Point::new(area.width - width, 0.0));
        layout::Node::with_children(area, vec![panel])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.panel.as_widget_mut().operate(
            &mut tree.children[0],
            Self::panel_layout(layout),
            renderer,
            operation,
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
        let panel = Self::panel_layout(layout);
        self.panel.as_widget_mut().update(
            &mut tree.children[0],
            event,
            panel,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        // The panel covers what is under it: a press on it never reaches the diff.
        if matches!(event, Event::Mouse(mouse::Event::ButtonPressed(_)))
            && cursor.is_over(panel.bounds())
        {
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let panel = Self::panel_layout(layout);
        let inner = self.panel.as_widget().mouse_interaction(
            &tree.children[0],
            panel,
            cursor,
            viewport,
            renderer,
        );
        // `Idle` rather than `None` over the panel, so a stack treats the area under it as covered.
        if inner == mouse::Interaction::None && cursor.is_over(panel.bounds()) {
            mouse::Interaction::Idle
        } else {
            inner
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
        use iced::advanced::Renderer as _;
        let panel = Self::panel_layout(layout);
        self.panel.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style_,
            panel,
            cursor,
            viewport,
        );
        // The edge against the diff: the outline divider, inside the panel's 1 px left inset.
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    width: 1.0,
                    ..panel.bounds()
                },
                ..Default::default()
            },
            Background::Color(style::color(self.roles.outline_variant)),
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        self.panel.as_widget_mut().overlay(
            &mut tree.children[0],
            Self::panel_layout(layout),
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    //! Contract D2, D3 (feature 1270 T027): the sheet is `min(360, half the area)` wide at the
    //! area's right edge, with its title and × laid out, and × emits its message.

    use super::*;
    use iced::advanced::mouse;
    use iced::{Event, Rectangle, Size};
    use micold_core::tokens::LIGHT;

    use crate::ui::material::icons::Icon;
    use crate::ui::material::test_support::{self, Mounted};

    fn sheet() -> SideSheet<'static, String> {
        SideSheet::new(Text::new("Rows", TypeRole::Body, LIGHT), LIGHT)
            .title("Comments")
            .on_close("close".to_owned())
    }

    /// The panel's bounds in an area `width` wide.
    fn panel(width: f32) -> Rectangle {
        test_support::bounds_at(sheet().into(), Size::new(width, 600.0), &[0])
    }

    #[test]
    fn the_sheet_is_360_wide_but_at_most_half_the_area_at_its_right_edge() {
        let wide = panel(1_000.0);
        assert_eq!(
            wide,
            Rectangle::new((640.0, 0.0).into(), (360.0, 600.0).into())
        );
        let narrow = panel(500.0);
        assert_eq!(
            narrow,
            Rectangle::new((250.0, 0.0).into(), (250.0, 600.0).into())
        );
    }

    #[test]
    fn the_title_close_and_content_are_laid_out_inside_the_panel() {
        let area = Size::new(1_000.0, 600.0);
        let mut mounted = Mounted::new_in(sheet(), area);
        let painted = mounted.painted_text();
        let find = |content: &str| {
            painted
                .iter()
                .find(|p| p.content == content)
                .unwrap_or_else(|| panic!("{content:?} is not painted: {painted:?}"))
                .bounds
        };
        let panel = panel(area.width);
        let title = find("Comments");
        let close = find(&Icon::Close.glyph().to_string());
        let content = find("Rows");
        for part in [title, close, content] {
            assert!(
                panel.contains(part.position()) && part.x + part.width <= panel.x + panel.width,
                "{part:?} inside {panel:?}"
            );
        }
        assert!(close.x > title.x + title.width, "× right of the title");
        assert!(
            content.y >= title.y + title.height,
            "content under the head"
        );
    }

    #[test]
    fn close_emits_its_message() {
        let mut mounted = Mounted::new_in(sheet(), Size::new(1_000.0, 600.0));
        let at = mounted
            .painted_text()
            .into_iter()
            .find(|p| p.content == Icon::Close.glyph().to_string())
            .expect("the ×")
            .bounds
            .center();
        let cursor = mouse::Cursor::Available(at);
        mounted.send(
            Event::Mouse(mouse::Event::CursorMoved { position: at }),
            cursor,
        );
        mounted.send(
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            cursor,
        );
        let published = mounted.send(
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            cursor,
        );
        assert_eq!(published, ["close"]);
    }

    /// A press on the sheet does not reach what it covers: the diff under it.
    #[test]
    fn a_press_on_the_panel_is_captured() {
        let mut mounted = Mounted::new_in(sheet(), Size::new(1_000.0, 600.0));
        let inside = iced::Point::new(800.0, 400.0);
        let (_, captured) = mounted.send_captured(
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            mouse::Cursor::Available(inside),
        );
        assert!(captured, "the sheet keeps the press");
        let outside = iced::Point::new(100.0, 400.0);
        let (_, captured) = mounted.send_captured(
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            mouse::Cursor::Available(outside),
        );
        assert!(!captured, "left of the sheet the diff gets the press");
    }
}
