//! `TabStrip` — a row of [`Tab`](super::Tab)s sharing one indicator edge (feature 026 FR-013).
//!
//! The strip is where "which edge" lives, because an orientation is a property of a strip and not
//! of a tab: a strip whose tabs each carried their own edge could be built with the bar above one
//! tab and below its neighbour, which is not an orientation at all. Setting it here and pushing it
//! down is what lets FR-014's gallery pose the two side by side and have each one mean something.
//!
//! Everything else a strip does is deliberately absent. It does not scroll, it does not know which
//! of its members is marked, and it does not decide what a tab holds — those belong to the call
//! site, which has the session the strip is a view of. What the component owns is the arrangement:
//! one row, the design system's gap, and one edge for all of them.

use crate::ui::material::tab::{IndicatorEdge, Tab};
use iced::widget::row;
use iced::{Alignment, Element};
use micold_core::tokens::{spacing, Roles};

/// A row of tabs. Builder form (Principle VIII):
/// `TabStrip::new(tabs, roles).edge(IndicatorEdge::Bottom).into()`.
pub struct TabStrip<'a, M> {
    pub(crate) tabs: Vec<Tab<'a, M>>,
    roles: Roles,
    pub(crate) edge: IndicatorEdge,
}

impl<'a, M: Clone + 'a> TabStrip<'a, M> {
    /// A strip of `tabs`, themed by `roles`, indicator on the default [`IndicatorEdge::Top`].
    pub fn new(tabs: Vec<Tab<'a, M>>, roles: Roles) -> Self {
        Self {
            tabs,
            roles,
            edge: IndicatorEdge::Top,
        }
    }

    /// Draw every tab's indicator on `edge`.
    ///
    /// Applied to the members here rather than at conversion so the strip can be *asked* what its
    /// tabs carry without rendering them, which is the only way the agreement is testable without a
    /// renderer.
    pub fn edge(mut self, edge: IndicatorEdge) -> Self {
        self.edge = edge;
        self.tabs = self.tabs.into_iter().map(|t| t.edge(edge)).collect();
        self
    }
}

impl<'a, M: Clone + 'a> From<TabStrip<'a, M>> for Element<'a, M> {
    fn from(s: TabStrip<'a, M>) -> Self {
        let _ = s.roles;
        let mut strip = row![].spacing(spacing::SM).align_y(Alignment::Center);
        for tab in s.tabs {
            strip = strip.push(tab);
        }
        strip.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::text::{Text, TypeRole};
    use crate::ui::material::{IndicatorEdge, Tab};
    use iced::Element;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens::{self, Roles};

    fn roles() -> Roles {
        tokens::roles(ColorScheme::Dark)
    }

    fn tabs<'a>(r: Roles) -> Vec<Tab<'a, ()>> {
        vec![
            Tab::new(Text::new("1", TypeRole::Label, r), r).active(true),
            Tab::new(Text::new("2", TypeRole::Label, r), r),
        ]
    }

    /// The edge belongs to the **strip**, not to each tab, and a tab in a strip takes the strip's.
    ///
    /// FR-014 asks the gallery to pose both orientations side by side, and the only honest way to
    /// pose an orientation is to set it once and have every member follow — a strip whose tabs each
    /// carried their own edge could be posed with the bar above one tab and below its neighbour,
    /// which is not an orientation at all. Both values are asserted because `tests/inventory` will
    /// require both to be posed and `showcase_completeness.rs` C3 will hold that.
    #[test]
    fn a_strip_gives_every_tab_its_own_indicator_edge() {
        let r = roles();
        for edge in [IndicatorEdge::Top, IndicatorEdge::Bottom] {
            let strip: TabStrip<'_, ()> = TabStrip::new(tabs(r), r).edge(edge);
            assert_eq!(
                strip.edge, edge,
                "the strip did not keep the edge it was given"
            );
            assert!(
                strip.tabs.iter().all(|t| t.edge == edge),
                "{edge:?}: a tab in the strip is drawing its indicator on a different edge from \
                 the strip it belongs to"
            );
        }
    }

    /// The default is the application's own, for the same reason a bare [`Tab`]'s is.
    #[test]
    fn a_strip_defaults_to_the_top_edge() {
        let r = roles();
        let strip: TabStrip<'_, ()> = TabStrip::new(tabs(r), r);
        assert_eq!(strip.edge, IndicatorEdge::Top);
        assert!(strip.tabs.iter().all(|t| t.edge == IndicatorEdge::Top));
        let _: Element<'_, ()> = strip.into();
    }
}
