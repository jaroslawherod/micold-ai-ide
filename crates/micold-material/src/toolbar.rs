//! `Toolbar` — the window's one top bar (Constitution Principle VIII; feature 1127, contract
//! B1–B6).
//!
//! A `surface` bar [`anatomy::app_bar::HEIGHT`] tall with a hairline beneath it at rest. From the
//! leading end: the app mark, the leading child (the project switcher, later the breadcrumb), the
//! fixed leading items (the Overview button), then free space, then the trailing actions. Any
//! future top bar should reuse it rather than fork a bespoke bar.
//!
//! # It lays itself out
//!
//! A `row!` with a `Length::Fill` spacer cannot say who gives way when the window is narrow: the
//! spacer takes the line before any child shrinks, and a long project name pushes the actions off
//! the trailing edge. So the bar is a give-way [`Lane`]: the mark, the fixed items and the actions
//! keep their natural width, and the leading child is the one that shortens
//! ([`GiveWay::Shorten`]), down to what it cannot give up. No child is laid out outside the bar and
//! none overlaps another (FR-010).
//!
//! Builder form: `Toolbar::new(roles).mark(m).leading(l).leading_fixed(f).action(a).into()`.

use crate::give_way::{Floor, GiveWay, Lane, Slot};
use crate::style;
use iced::widget::{column, container, Space};
use iced::{Background, Element, Length};
use micold_core::tokens::{anatomy, Roles};

/// The top bar, built child by child (Principle VIII builder-API rule), then `.into()`.
pub struct Toolbar<'a, M> {
    roles: Roles,
    mark: Option<Element<'a, M>>,
    leading: Option<Element<'a, M>>,
    leading_fixed: Vec<Element<'a, M>>,
    actions: Vec<Element<'a, M>>,
    elevated: bool,
}

impl<'a, M: 'a> Toolbar<'a, M> {
    /// An empty bar themed by `roles`.
    pub fn new(roles: Roles) -> Self {
        Self {
            roles,
            mark: None,
            leading: None,
            leading_fixed: Vec::new(),
            actions: Vec::new(),
            elevated: false,
        }
    }

    /// The bar's first child, at its leading end: the app mark. It never gives way.
    pub fn mark(mut self, mark: impl Into<Element<'a, M>>) -> Self {
        self.mark = Some(mark.into());
        self
    }

    /// The child after the mark, and the one that shortens when the bar is too narrow.
    pub fn leading(mut self, leading: impl Into<Element<'a, M>>) -> Self {
        self.leading = Some(leading.into());
        self
    }

    /// An item after the leading child, at its natural width, after those already added.
    pub fn leading_fixed(mut self, item: impl Into<Element<'a, M>>) -> Self {
        self.leading_fixed.push(item.into());
        self
    }

    /// Whether content is scrolled beneath the bar, which raises it to elevation 2 (FR-025a).
    ///
    /// Supplied rather than observed: the bar has no way to see the sidebar's scroll position, and
    /// the sidebar is the only scroll region under it (contract §7.1). `State::app_bar_elevated`
    /// derives the flag from that one offset.
    pub fn elevated(mut self, elevated: bool) -> Self {
        self.elevated = elevated;
        self
    }

    /// Append a trailing action element.
    pub fn action(mut self, action: impl Into<Element<'a, M>>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Append several trailing action elements.
    pub fn actions(mut self, actions: Vec<Element<'a, M>>) -> Self {
        self.actions.extend(actions);
        self
    }
}

/// A slot that keeps its width, in the leading or the trailing group.
fn kept(trailing: bool) -> Slot {
    Slot {
        give_way: GiveWay::Keep,
        trailing,
        floor: Floor::Natural,
    }
}

impl<'a, M: 'a> From<Toolbar<'a, M>> for Element<'a, M> {
    fn from(t: Toolbar<'a, M>) -> Self {
        let mut lane = Lane::new(
            anatomy::app_bar::HEIGHT,
            anatomy::app_bar::PADDING,
            anatomy::app_bar::GAP,
        );
        if let Some(mark) = t.mark {
            lane = lane.push(mark, kept(false));
        }
        if let Some(leading) = t.leading {
            let shortens = Slot {
                give_way: GiveWay::Shorten(0),
                trailing: false,
                floor: Floor::Squeezed,
            };
            lane = lane.push(leading, shortens);
        }
        for item in t.leading_fixed {
            lane = lane.push(item, kept(false));
        }
        for action in t.actions {
            lane = lane.push(action, kept(true));
        }

        let elevated = t.elevated;
        let bar = container(Element::from(lane))
            .width(Length::Fill)
            .height(Length::Fixed(anatomy::app_bar::HEIGHT))
            .style(style::app_bar_surface(t.roles, elevated));

        // The separator belongs to the bar *at rest* only. Once the bar is raised its own shadow
        // and tonal shift do that job (§7.1), and keeping a hairline under a shadow reads as two
        // edges — the thing FR-015 replaces lines with depth to avoid.
        if elevated {
            return bar.into();
        }
        // `anatomy::app_bar::BOTTOM_EDGE` is the bar plus this — the number every surface anchored
        // below the bar reads (FR-029a). Drawn from the constant so the two cannot drift apart.
        let separator = container(
            Space::new()
                .width(Length::Fill)
                .height(Length::Fixed(anatomy::app_bar::DIVIDER)),
        )
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(style::separator(t.roles))),
            ..Default::default()
        });

        column![bar, separator].into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{laid_out, leaves, overlap, roles, Mounted};
    use super::*;
    use crate::{Ellipsized, TypeRole};
    use iced::advanced::layout;
    use iced::{Rectangle, Size};

    const WIDE: Size = Size::new(1280.0, 800.0);
    const HEIGHT: f32 = anatomy::app_bar::HEIGHT;

    /// A fixed-size box standing in for a child.
    fn block(w: f32, h: f32) -> Element<'static, String> {
        Space::new().width(w).height(h).into()
    }

    /// A bar of the title bar's shape: a 24 mark, a long leading label, a 120 fixed item, two 40
    /// actions.
    fn bar(label: &str) -> Toolbar<'static, String> {
        Toolbar::new(roles())
            .mark(block(anatomy::app_bar::MARK, anatomy::app_bar::MARK))
            .leading(
                Ellipsized::at_role(label.to_string(), TypeRole::Action, roles().on_surface)
                    .shrink(),
            )
            .leading_fixed(block(120.0, 40.0))
            .action(block(40.0, 40.0))
            .action(block(40.0, 40.0))
    }

    /// The lane's children's bounds, in window space: shell column → bar container → lane.
    fn children(node: &layout::Node) -> Vec<Rectangle> {
        let lane = &node.children()[0].children()[0];
        let offset = node.children()[0].bounds().position() - iced::Point::ORIGIN;
        lane.children()
            .iter()
            .map(|c| c.bounds() + offset + (lane.bounds().position() - iced::Point::ORIGIN))
            .collect()
    }

    /// B1: `HEIGHT` + `DIVIDER` tall at rest, `HEIGHT` elevated, full width.
    #[test]
    fn the_bar_is_its_height_plus_the_divider_at_rest_and_its_height_elevated() {
        let rest = laid_out(bar("micold-ai-ide").into(), WIDE).bounds();
        assert_eq!(rest.height, anatomy::app_bar::BOTTOM_EDGE);
        assert_eq!(rest.width, WIDE.width);
        let raised = laid_out(bar("micold-ai-ide").elevated(true).into(), WIDE).bounds();
        assert_eq!(raised.height, HEIGHT);
    }

    /// B1, B2: `PADDING` at each end, `GAP` between children, centred vertically, in the order
    /// mark, leading, leading-fixed, then the actions at the trailing end.
    #[test]
    fn children_sit_in_bar_order_padding_from_the_ends_and_gap_apart() {
        let c = children(&laid_out(bar("micold-ai-ide").into(), WIDE));
        assert_eq!(c.len(), 5);
        assert_eq!(c[0].x, anatomy::app_bar::PADDING, "the mark leads");
        for pair in c[..3].windows(2) {
            assert_eq!(pair[1].x, pair[0].x + pair[0].width + anatomy::app_bar::GAP);
        }
        assert_eq!(c[4].x + c[4].width, WIDE.width - anatomy::app_bar::PADDING);
        assert_eq!(c[3].x + c[3].width + anatomy::app_bar::GAP, c[4].x);
        assert!(
            c[2].x + c[2].width < c[3].x,
            "free space before the actions"
        );
        for b in &c {
            assert!(
                (b.center_y() - HEIGHT / 2.0).abs() < 0.5,
                "{b:?} is not centred"
            );
        }
    }

    /// B4: at a narrow width the leading child shortens while the mark, the fixed item and the
    /// actions keep their width; nothing leaves the bar or overlaps.
    #[test]
    fn at_a_narrow_width_only_the_leading_child_shortens() {
        let long = "a-project-name-sixty-characters-long-for-the-narrow-bar-test";
        let wide = children(&laid_out(bar(long).into(), WIDE));
        let narrow_room = Size::new(400.0, 800.0);
        let narrow = children(&laid_out(bar(long).into(), narrow_room));
        assert!(
            narrow[1].width < wide[1].width,
            "the leading child did not shorten"
        );
        for i in [0, 2, 3, 4] {
            assert_eq!(narrow[i].width, wide[i].width, "child {i} gave way");
        }
        for (i, a) in narrow.iter().enumerate() {
            assert!(
                a.x >= 0.0
                    && a.x + a.width <= narrow_room.width
                    && a.y >= 0.0
                    && a.y + a.height <= HEIGHT,
                "child {i} at {a:?} leaves the bar"
            );
            for b in &narrow[i + 1..] {
                assert!(!overlap(a, b), "{a:?} overlaps {b:?}");
            }
        }
    }

    /// B6: what floats over the bar (a tooltip under the pointer) moves none of its children.
    #[test]
    fn what_floats_over_the_bar_moves_no_child() {
        let with_tip = || {
            Toolbar::new(roles())
                .mark(crate::Tooltip::new(block(24.0, 24.0), "Micold", roles()))
                .action(block(40.0, 40.0))
        };
        let mut m = Mounted::new_in(with_tip(), WIDE);
        let before = leaves(&m.node);
        let at = iced::Point::new(anatomy::app_bar::PADDING + 12.0, HEIGHT / 2.0);
        assert!(!m.floated_text(at).is_empty(), "the tooltip floats");
        m.relayout();
        assert_eq!(leaves(&m.node), before);
    }
}
