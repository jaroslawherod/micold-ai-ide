//! `Toolbar` — the window's one top bar (Constitution Principle VIII; feature 1127, contract
//! B1–B6).
//!
//! A `surface` bar [`anatomy::app_bar::HEIGHT`] tall with a hairline beneath it at rest. From the
//! leading end: the app mark, the leading child (the breadcrumb), the fixed leading items (none in
//! the app today), then the centre child (the jump-to field) in the free space, then the trailing
//! actions. Any future top bar should reuse it rather than fork a bespoke bar.
//!
//! # It lays itself out
//!
//! A `row!` with a `Length::Fill` spacer cannot say who gives way when the window is narrow: the
//! spacer takes the line before any child shrinks, and a long project name pushes the actions off
//! the trailing edge, and a centre child cannot both sit centred and yield first. So the bar is a
//! give-way [`Lane`]: the mark, the fixed items and the actions keep their natural width; the
//! centre child yields first ([`GiveWay::Shorten`] at rank 0) and the leading child next (rank 1),
//! each down to what it cannot give up. No child is laid out outside the bar and none overlaps
//! another (FR-010).
//!
//! For the title bar that is data-model *Give-way order*'s five steps: the field gives up its spare
//! width, then its label, then its hint (a [`FieldButton`](crate::FieldButton) narrowed past a
//! [`Fit`](crate::FieldFit) draws the next one), then the breadcrumb shortens its place, then its
//! project (the breadcrumb's own lane). Each step happens only once the one before is spent.
//!
//! The centre child sits with equal free space either side between the end of the leading group
//! and the start of the trailing group — not on the window's centre line, which a long breadcrumb
//! would make it cross (contract B3).
//!
//! Builder form:
//! `Toolbar::new(roles).mark(m).leading(l).leading_fixed(f).centre(c).action(a).into()`.

use crate::give_way::{Floor, GiveWay, Group, Lane, Slot};
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
    centre: Option<Element<'a, M>>,
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
            centre: None,
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

    /// The child centred in the free space between the leading and the trailing groups: the
    /// jump-to field. It takes the free space up to its own maximum, and is the first to give way.
    pub fn centre(mut self, centre: impl Into<Element<'a, M>>) -> Self {
        self.centre = Some(centre.into());
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

/// A slot that keeps its width, in `group`.
fn kept(group: Group) -> Slot {
    Slot {
        give_way: GiveWay::Keep,
        group,
        floor: Floor::Natural,
    }
}

/// A slot that shortens at `rank` down to what its child cannot give up, in `group`.
fn shortens(rank: u8, group: Group) -> Slot {
    Slot {
        give_way: GiveWay::Shorten(rank),
        group,
        floor: Floor::Squeezed,
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
            lane = lane.push(mark, kept(Group::Leading));
        }
        if let Some(leading) = t.leading {
            lane = lane.push(leading, shortens(1, Group::Leading));
        }
        for item in t.leading_fixed {
            lane = lane.push(item, kept(Group::Leading));
        }
        if let Some(centre) = t.centre {
            // No narrower than the bar's control target: the field's last fit is its glyph in an
            // `ICON_TARGET` square, still pressable (FR-010).
            let centre_slot = Slot {
                floor: Floor::Width(anatomy::app_bar::ICON_TARGET),
                ..shortens(0, Group::Centre)
            };
            lane = lane.push(centre, centre_slot);
        }
        for action in t.actions {
            lane = lane.push(action, kept(Group::Trailing));
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

    const LONG_PROJECT: &str = "a-project-with-a-name-sixty-characters-long-for-the-title-ba";
    const LONG_PLACE: &str = "feat/1127-a-branch-name-sixty-characters-long-for-the-bar-x";
    const LABEL: &str = "Jump to location, session or project";
    const HINT: &str = "Ctrl+Shift+P";
    /// Wider than the field needs for its words, so the first step — giving up spare width — is a
    /// step of its own whatever the font measures.
    const FIELD_MAX: f32 = 420.0;

    /// The title bar's whole shape: the mark, a `project / place` breadcrumb, a 120 fixed item, the
    /// jump-to field in the centre, two 40 actions.
    fn title_bar(project: &str, place: &str) -> Toolbar<'static, String> {
        // The bundled fonts first: the breadcrumb and the field measure their words as they are
        // built.
        let _ = super::super::test_support::renderer();
        Toolbar::new(roles())
            .mark(block(anatomy::app_bar::MARK, anatomy::app_bar::MARK))
            .leading(
                crate::Breadcrumb::new(project, roles())
                    .on_press("switch".to_string())
                    .place(place, None),
            )
            .leading_fixed(block(120.0, 40.0))
            .centre(
                crate::FieldButton::new(crate::icons::Icon::Search, LABEL, roles())
                    .hint(HINT)
                    .max_width(FIELD_MAX)
                    .on_press_maybe(Some("jump".to_string())),
            )
            .action(block(40.0, 40.0))
            .action(block(40.0, 40.0))
    }

    /// B3 (FR-008): the centre child has as much free space before it as after it, between the end
    /// of the leading group and the start of the trailing group — which, with a breadcrumb longer
    /// than the actions, is not the window's centre line.
    #[test]
    fn the_centre_child_is_centred_in_the_free_space_not_the_window() {
        for place in ["fix/1117", "feat/1127-a-longer-branch-name-for-the-bar"] {
            let c = children(&laid_out(title_bar("micold-ai-ide", place).into(), WIDE));
            assert_eq!(c.len(), 6, "mark, breadcrumb, fixed, centre, two actions");
            let (lead_end, field, trail_start) = (c[2].x + c[2].width, c[3], c[4].x);
            let before = field.x - lead_end;
            let after = trail_start - (field.x + field.width);
            assert!(
                (before - after).abs() <= 0.5,
                "{place}: {before} before the field, {after} after"
            );
            assert!(before >= anatomy::app_bar::GAP, "{place}: {before}");
            assert!(
                (field.center_x() - WIDE.width / 2.0).abs() > 1.0,
                "{place}: centred on the window, not in the free space"
            );
            assert_eq!(field.width, FIELD_MAX, "{place}");
        }
    }

    /// How far a bar `width` wide has given way (data-model *Give-way order*): 0 nothing; 1 the
    /// field is narrower than its maximum; 2 it dropped its label; 3 its hint; 4 the place is
    /// shortened; 5 the project too. Checks the steps' order at that width on the way.
    fn stage(width: f32) -> u8 {
        let room = Size::new(width, 800.0);
        let mut m = Mounted::new_in(title_bar(LONG_PROJECT, LONG_PLACE), room);
        let field = children(&m.node)[3];
        let painted: Vec<String> = m.painted_text().into_iter().map(|p| p.content).collect();
        let has = |w: &str| painted.iter().any(|p| p == w);
        let cut = |full: &str| {
            painted
                .iter()
                .find(|p| p.ends_with('…') && full.starts_with(p.trim_end_matches('…')))
                .cloned()
        };
        let (label, hint) = (has(LABEL), has(HINT));
        let (place, project) = (cut(LONG_PLACE), cut(LONG_PROJECT));
        let at = format!("at {width}: {painted:?}");
        assert!(has(&crate::icons::Icon::Search.glyph().to_string()), "{at}");
        assert!(!label || hint, "the hint went before the label {at}");
        if hint {
            assert!(place.is_none() && project.is_none(), "{at}");
        }
        if place.is_some() {
            assert_eq!(field.width, anatomy::app_bar::ICON_TARGET, "{at}");
        }
        if project.is_some() {
            assert_eq!(
                place.as_deref().map(|p| p.chars().count()),
                Some(2),
                "the place is at its floor first {at}"
            );
        }
        if project.is_some() {
            5
        } else if place.is_some() {
            4
        } else if !hint {
            3
        } else if !label {
            2
        } else if field.width < FIELD_MAX - 0.5 {
            1
        } else {
            0
        }
    }

    /// B4 (FR-010, FR-007): narrowing the bar, the field gives up its spare width, then its label,
    /// then its hint; then the place shortens, then the project — each step only once the one
    /// before is spent, and every step is taken on the way down.
    #[test]
    fn the_bar_gives_way_in_the_five_step_order() {
        let mut seen = Vec::new();
        let mut width = 2000.0;
        while width >= 500.0 {
            let now = stage(width);
            if let Some(&last) = seen.last() {
                assert!(now >= last, "at {width} stage {now} after {last}");
            }
            if seen.last() != Some(&now) {
                seen.push(now);
            }
            width -= 2.0;
        }
        assert_eq!(seen, vec![0, 1, 2, 3, 4, 5], "every step, in order");
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
