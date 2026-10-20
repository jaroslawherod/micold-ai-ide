//! Which Overview card is expanded and where it is drawn (feature 1121, research R1/R6,
//! contracts K1–K3).
//!
//! Pure: no I/O and no logging (FR-016, FR-018). The client only feeds events in and draws the card
//! this names at the place computed here, so the rules are tested without the GUI.

use std::time::Duration;

use crate::session::SessionLocation;

/// How long the pointer rests on a card before it expands (K2, R5): under the 0.5 s of SC-006.
pub const EXPAND_REST: Duration = Duration::from_millis(400);

/// Why a card is expanded (data-model *Expansion*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// The pointer rested on it.
    Pointer,
    /// It received keyboard focus.
    Focus,
}

/// What can change which card is expanded (data-model *Events and transitions*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The pointer rested on a card for [`EXPAND_REST`].
    PointerRested(SessionLocation),
    /// The pointer left a card it was over.
    PointerLeft(SessionLocation),
    /// A card received keyboard (shown) focus.
    FocusGained(SessionLocation),
    /// A card lost keyboard focus.
    FocusLost(SessionLocation),
    /// Escape, with a card expanded (D1).
    Escape,
    /// The window lost focus.
    WindowBlurred,
    /// A card menu opened: nothing changes form until it closes (FR-012).
    MenuOpened,
    /// The card menu closed.
    MenuClosed,
    /// The cards now listed: a card not among them is gone.
    Listed(Vec<SessionLocation>),
    /// The expanded card's inner scroll can move up / down.
    BodyScrolled {
        /// It can scroll up.
        up: bool,
        /// It can scroll down.
        down: bool,
    },
}

/// Which Overview card is expanded and the facts that decide the next one (data-model
/// *Expansion*). `Default` expands nothing (FR-013).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Expansion {
    expanded: Option<(SessionLocation, Cause)>,
    focused: Option<SessionLocation>,
    focus_escaped: bool,
    pointer_on: Option<SessionLocation>,
    pointer_escaped: bool,
    menu_open: bool,
    body_edges: (bool, bool),
}

impl Expansion {
    /// The expanded card, if any (FR-006: at most one).
    pub fn expanded(&self) -> Option<&SessionLocation> {
        self.expanded.as_ref().map(|(location, _)| location)
    }

    /// Why the expanded card is expanded.
    pub fn cause(&self) -> Option<Cause> {
        self.expanded.as_ref().map(|&(_, cause)| cause)
    }

    /// Whether the expanded card's inner scroll can move (up, down); `(false, false)` with none.
    pub fn body_edges(&self) -> (bool, bool) {
        self.body_edges
    }

    /// Take in `event` (K1, data-model *Events and transitions*).
    pub fn apply(&mut self, event: Event) {
        match event {
            Event::PointerRested(location) => {
                // Escape collapsed this card with the pointer still on it: it stays collapsed
                // until the pointer leaves it.
                let escaped_here =
                    self.pointer_escaped && self.pointer_on.as_ref() == Some(&location);
                if self.menu_open || escaped_here {
                    return;
                }
                self.pointer_on = Some(location.clone());
                self.pointer_escaped = false;
                self.expand(location, Cause::Pointer);
            }
            Event::PointerLeft(location) => {
                if self.pointer_on.as_ref() == Some(&location) {
                    self.pointer_on = None;
                    self.pointer_escaped = false;
                }
                if self.expanded == Some((location, Cause::Pointer)) && !self.menu_open {
                    self.collapse();
                    // FR-005: the hover ended on no other card; the focused card expands again,
                    // unless Escape collapsed it since it got focus.
                    if let Some(focused) = self.focused.clone().filter(|_| !self.focus_escaped) {
                        self.expand(focused, Cause::Focus);
                    }
                }
            }
            Event::FocusGained(location) => {
                self.focused = Some(location.clone());
                self.focus_escaped = false;
                if !self.menu_open {
                    self.expand(location, Cause::Focus);
                }
            }
            Event::FocusLost(location) => {
                if self.focused.as_ref() == Some(&location) {
                    self.focused = None;
                    self.focus_escaped = false;
                }
                if self.expanded == Some((location, Cause::Focus)) && !self.menu_open {
                    self.collapse();
                }
            }
            Event::Escape => {
                if let Some((location, _)) = self.expanded.take() {
                    self.body_edges = (false, false);
                    self.focus_escaped |= self.focused.as_ref() == Some(&location);
                    self.pointer_escaped |= self.pointer_on.as_ref() == Some(&location);
                }
            }
            Event::WindowBlurred => self.collapse(),
            Event::MenuOpened => self.menu_open = true,
            Event::MenuClosed => self.menu_open = false,
            Event::Listed(listed) => {
                let gone = |slot: &Option<SessionLocation>| {
                    slot.as_ref()
                        .is_some_and(|location| !listed.contains(location))
                };
                if gone(&self.expanded.as_ref().map(|(location, _)| location.clone())) {
                    self.collapse();
                }
                if gone(&self.focused) {
                    self.focused = None;
                    self.focus_escaped = false;
                }
                if gone(&self.pointer_on) {
                    self.pointer_on = None;
                    self.pointer_escaped = false;
                }
            }
            Event::BodyScrolled { up, down } => {
                if self.expanded.is_some() {
                    self.body_edges = (up, down);
                }
            }
        }
    }

    /// Expand `location` for `cause`; another expanded card collapses (FR-006).
    fn expand(&mut self, location: SessionLocation, cause: Cause) {
        if self.expanded.as_ref().map(|(expanded, _)| expanded) != Some(&location) {
            self.body_edges = (false, false);
        }
        self.expanded = Some((location, cause));
    }

    /// Collapse the expanded card, if any.
    fn collapse(&mut self) {
        self.expanded = None;
        self.body_edges = (false, false);
    }
}

/// Where an expanded card is drawn, in the grid's coordinates (data-model *Placement*).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Its top edge.
    pub top: f32,
    /// Its height: its natural height, capped at the view's.
    pub height: f32,
    /// Its content is taller than the view and scrolls inside the card (FR-008).
    pub scrolls: bool,
}

/// Where a card whose grid slot is `slot` (`(top, height)`) and whose expanded content needs
/// `natural` is drawn inside a view `view` (`(top, height)`) (K3, FR-008): as tall as its content,
/// up to the view's height; at its slot's top, unless that would end below the view, then moved up
/// just enough to end at the view's bottom, but never above the view's top nor below its slot's top.
pub fn place(slot: (f32, f32), natural: f32, view: (f32, f32)) -> Placement {
    let (slot_top, _) = slot;
    let (view_top, view_height) = view;
    let height = natural.min(view_height);
    let view_bottom = view_top + view_height;
    let top = if slot_top + height <= view_bottom {
        slot_top
    } else {
        (view_bottom - height).max(view_top).min(slot_top)
    };
    Placement {
        top,
        height,
        scrolls: natural > view_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The view in every test: 600 px tall, scrolled 1000 px down.
    const VIEW: (f32, f32) = (1000.0, 600.0);
    const VIEW_BOTTOM: f32 = VIEW.0 + VIEW.1;
    const SLOT_HEIGHT: f32 = 200.0;

    /// K3: a card that fits below its slot's top stays at its slot, as tall as its content.
    #[test]
    fn a_card_that_fits_stays_at_its_slot() {
        let p = place((1100.0, SLOT_HEIGHT), 320.0, VIEW);
        assert_eq!(
            p,
            Placement {
                top: 1100.0,
                height: 320.0,
                scrolls: false
            },
            "a card that ends inside the view keeps its slot's top"
        );
    }

    /// K3 (US2 sc.5): a card that would end below the view moves up just enough to end at its
    /// bottom.
    #[test]
    fn a_card_that_would_end_below_the_view_moves_up_to_its_bottom() {
        let p = place((1350.0, SLOT_HEIGHT), 400.0, VIEW);
        assert_eq!(
            p.top,
            VIEW_BOTTOM - 400.0,
            "moved up just enough to be seen whole"
        );
        assert_eq!(p.height, 400.0);
        assert!(!p.scrolls);
    }

    /// K3 (US2 sc.6): a card taller than the view is as tall as the view, starts at the view's
    /// top, and scrolls inside.
    #[test]
    fn a_card_taller_than_the_view_is_capped_and_scrolls() {
        let p = place((1200.0, SLOT_HEIGHT), 900.0, VIEW);
        assert_eq!(
            p,
            Placement {
                top: VIEW.0,
                height: VIEW.1,
                scrolls: true
            },
            "capped at the view's height, moved to its top, scrolling inside"
        );
    }

    /// K3: the shift never moves a card below its slot's top, even when the slot's top lies above
    /// the view (the Overview scrolled past it).
    #[test]
    fn a_card_is_never_moved_below_its_slots_top() {
        let p = place((900.0, SLOT_HEIGHT), 900.0, VIEW);
        assert_eq!(p.top, 900.0, "a slot above the view keeps its top");
        assert_eq!(p.height, VIEW.1);
    }

    /// K3: whenever the expanded content is at least the slot's height, the drawn card covers its
    /// whole slot — for every slot that ends inside the view (one that ends below it cannot be
    /// both covered and seen whole; the shift keeps the card in view).
    #[test]
    fn the_placed_card_always_covers_its_slot() {
        for slot_top in (700..=(VIEW_BOTTOM - SLOT_HEIGHT) as i32).step_by(25) {
            for natural in (200..1000).step_by(50) {
                let slot = (slot_top as f32, SLOT_HEIGHT);
                let p = place(slot, natural as f32, VIEW);
                assert!(
                    p.top <= slot.0 && p.top + p.height >= slot.0 + slot.1,
                    "slot {slot:?}, natural {natural}: {p:?} must cover the slot"
                );
                assert_eq!(p.height, (natural as f32).min(VIEW.1));
            }
        }
    }

    fn loc(name: &str) -> SessionLocation {
        SessionLocation::Worktree(name.to_owned())
    }

    /// An expansion after `events`, in order.
    fn after(events: impl IntoIterator<Item = Event>) -> Expansion {
        let mut expansion = Expansion::default();
        for event in events {
            expansion.apply(event);
        }
        expansion
    }

    fn rest(name: &str) -> Event {
        Event::PointerRested(loc(name))
    }

    fn leave(name: &str) -> Event {
        Event::PointerLeft(loc(name))
    }

    fn focus(name: &str) -> Event {
        Event::FocusGained(loc(name))
    }

    /// The expanded card's name, for short assertions.
    fn shown(expansion: &Expansion) -> Option<String> {
        expansion.expanded().map(|location| match location {
            SessionLocation::Worktree(name) => name.clone(),
            SessionLocation::Default => "Default".to_owned(),
        })
    }

    /// K1, FR-013: a fresh expansion expands nothing.
    #[test]
    fn a_fresh_expansion_expands_nothing() {
        let fresh = Expansion::default();
        assert_eq!(shown(&fresh), None);
        assert_eq!(fresh.cause(), None);
        assert_eq!(fresh.body_edges(), (false, false));
    }

    /// K1, Story 2 sc.1/sc.3: a rest expands the card by the pointer; leaving collapses it.
    #[test]
    fn a_rest_expands_a_card_and_leaving_collapses_it() {
        let rested = after([rest("a")]);
        assert_eq!(shown(&rested), Some("a".into()));
        assert_eq!(rested.cause(), Some(Cause::Pointer));
        assert_eq!(shown(&after([rest("a"), leave("a")])), None);
    }

    /// K1, FR-006, Story 2 sc.4: a rest on another card replaces the expanded one.
    #[test]
    fn a_rest_on_another_card_expands_only_that_card() {
        assert_eq!(shown(&after([rest("a"), rest("b")])), Some("b".into()));
        // A's leave arriving after B's rest leaves B expanded.
        assert_eq!(
            shown(&after([rest("a"), rest("b"), leave("a")])),
            Some("b".into())
        );
    }

    /// K1, Story 3 sc.1: focus expands a card by focus; focus on another card replaces it.
    #[test]
    fn focus_expands_the_focused_card() {
        let focused = after([focus("a")]);
        assert_eq!(shown(&focused), Some("a".into()));
        assert_eq!(focused.cause(), Some(Cause::Focus));
        assert_eq!(shown(&after([focus("a"), focus("b")])), Some("b".into()));
        assert_eq!(
            shown(&after([focus("a"), Event::FocusLost(loc("a"))])),
            None
        );
    }

    /// K1, FR-005: a hover ending without a rest elsewhere re-expands the focused card, unless
    /// Escape collapsed it since it got focus.
    #[test]
    fn a_hover_ending_re_expands_the_focused_card_unless_escaped() {
        let back = after([focus("a"), rest("b"), leave("b")]);
        assert_eq!(shown(&back), Some("a".into()));
        assert_eq!(back.cause(), Some(Cause::Focus));
        assert_eq!(
            shown(&after([focus("a"), Event::Escape, rest("b"), leave("b")])),
            None
        );
    }

    /// K1, Edge Case *Window regains focus*: a blur collapses, and nothing re-expands after it.
    #[test]
    fn a_window_blur_collapses_and_nothing_re_expands() {
        assert_eq!(
            shown(&after([focus("a"), rest("b"), Event::WindowBlurred])),
            None
        );
        assert_eq!(
            shown(&after([
                focus("a"),
                rest("b"),
                Event::WindowBlurred,
                leave("b")
            ])),
            None
        );
        assert_eq!(shown(&after([focus("a"), Event::WindowBlurred])), None);
    }

    /// K1, Story 3 sc.5, D1: Escape collapses a focus-expanded card and keeps its focus; focus
    /// moving away and back expands it again.
    #[test]
    fn escape_collapses_the_focused_card_until_focus_moves_back() {
        let escaped = after([focus("a"), Event::Escape]);
        assert_eq!(shown(&escaped), None);
        assert_eq!(escaped.focused, Some(loc("a")));
        assert_eq!(
            shown(&after([focus("a"), Event::Escape, focus("b"), focus("a")])),
            Some("a".into())
        );
    }

    /// K1: Escape on a hover-expanded card holds while the pointer stays on it; leaving and
    /// resting again expands it.
    #[test]
    fn escape_on_a_hovered_card_holds_until_the_pointer_leaves() {
        let held = [rest("a"), Event::Escape];
        assert_eq!(shown(&after(held.clone())), None);
        let further = [
            Event::MenuOpened,
            Event::MenuClosed,
            Event::Listed(vec![loc("a")]),
            rest("a"),
        ];
        assert_eq!(shown(&after(held.clone().into_iter().chain(further))), None);
        assert_eq!(
            shown(&after(held.into_iter().chain([leave("a"), rest("a")]))),
            Some("a".into())
        );
    }

    /// K1, FR-005: Escape on the focused card while the pointer expanded it still counts as escaped.
    #[test]
    fn escape_on_the_focused_card_expanded_by_the_pointer_counts() {
        let events = [focus("a"), rest("a"), Event::Escape, rest("b"), leave("b")];
        assert_eq!(shown(&after(events)), None);
    }

    /// K1, FR-012: while a card menu is open nothing changes form; once closed, events act again.
    #[test]
    fn an_open_menu_freezes_the_form() {
        assert_eq!(
            shown(&after([
                rest("a"),
                Event::MenuOpened,
                rest("b"),
                focus("b")
            ])),
            Some("a".into())
        );
        assert_eq!(
            shown(&after([Event::MenuOpened, rest("b"), focus("b")])),
            None
        );
        let reopened = [Event::MenuOpened, rest("b"), Event::MenuClosed, rest("b")];
        assert_eq!(shown(&after(reopened)), Some("b".into()));
        assert_eq!(
            shown(&after([Event::MenuOpened, Event::MenuClosed, focus("b")])),
            Some("b".into())
        );
    }

    /// K1, Edge Case *card removed*: a listing without the expanded card collapses it; a focus
    /// hand-back then expands the card it lands on.
    #[test]
    fn a_removed_card_collapses_and_the_hand_back_expands() {
        let gone = after([focus("a"), Event::Listed(vec![loc("b")])]);
        assert_eq!(shown(&gone), None);
        assert_eq!(gone.focused, None);
        let handed_back = [focus("a"), Event::Listed(vec![loc("b")]), focus("b")];
        assert_eq!(shown(&after(handed_back)), Some("b".into()));
        let kept = [rest("a"), Event::Listed(vec![loc("a"), loc("b")])];
        assert_eq!(shown(&after(kept)), Some("a".into()));
    }

    /// K1: every collapse clears the inner scroll's edges.
    #[test]
    fn each_collapse_clears_the_body_edges() {
        let edges = Event::BodyScrolled {
            up: true,
            down: true,
        };
        let collapses: [Vec<Event>; 5] = [
            vec![leave("a")],
            vec![Event::Escape],
            vec![Event::WindowBlurred],
            vec![Event::Listed(vec![])],
            vec![rest("b")],
        ];
        for collapse in collapses {
            let label = format!("{collapse:?}");
            let expansion = after([rest("a"), edges.clone()].into_iter().chain(collapse));
            assert_eq!(expansion.body_edges(), (false, false), "{label}");
        }
        assert_eq!(after([rest("a"), edges]).body_edges(), (true, true));
    }

    /// K2, SC-006: a card expands within half a second of the pointer resting on it.
    #[test]
    fn the_rest_is_under_half_a_second() {
        assert!(EXPAND_REST <= Duration::from_millis(500));
        assert!(!EXPAND_REST.is_zero());
    }
}
