//! Which Overview card is expanded and where it is drawn (feature 1121, research R1/R6,
//! contracts K1–K4).
//!
//! Pure: no I/O and no logging (FR-016, FR-018). The client only feeds events in and draws the card
//! this names at the place computed here, so the rules are tested without the GUI.

use std::time::Duration;

use crate::session::SessionLocation;

/// How long the pointer rests on a card before it expands (K2, R5): under the 0.5 s of SC-006.
pub const EXPAND_REST: Duration = Duration::from_millis(400);

/// One link of a card (feature 1123, research R6, data-model §5): its issue link, or the number of
/// one of its pull request rows. Stable per number, so a reading that reorders the rows keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardLink {
    /// The issue link on the branch line (feature 1122).
    Issue,
    /// The number of a pull request row.
    PullRequest(u64),
}

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
    /// The pointer rested on a card's issue link for [`EXPAND_REST`] (feature 1122, FR-007): it
    /// counts as a rest on that card, but expands nothing.
    LinkRested(SessionLocation),
    /// The pointer left a card it was over.
    PointerLeft(SessionLocation),
    /// A card received keyboard (shown) focus.
    FocusGained(SessionLocation),
    /// A card lost keyboard focus.
    FocusLost(SessionLocation),
    /// One of a card's links received keyboard (shown) focus (feature 1122, FR-008; feature 1123,
    /// FR-009): focus on a link counts as focus on its card.
    LinkFocusGained(SessionLocation, CardLink),
    /// One of a card's links lost keyboard focus.
    LinkFocusLost(SessionLocation, CardLink),
    /// The Overview opened and put the keyboard on this card (research R13): the focus it reports
    /// next expands nothing (FR-013), and the card stays collapsed until focus moves.
    OpenFocused(SessionLocation),
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

/// Which of the focused card's keyboard stops reported the last gain (feature 1123, research R6,
/// data-model §5): the two reports of a move inside one card, in either order, are one move, and
/// the part that gained last holds the keyboard.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum FocusPart {
    /// The card itself.
    #[default]
    Card,
    /// One of its links.
    Link(CardLink),
}

/// A focus loss of `part` of `location` and the expansion just before it: if the very next event
/// is the gain of another part of that card, the two are one move inside the card and the loss is
/// undone ("just held", contract K5).
#[derive(Debug, Clone, PartialEq, Eq)]
struct JustLost {
    location: SessionLocation,
    part: FocusPart,
    before: Expansion,
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
    /// The rest on `pointer_on` was on its issue link (feature 1122, FR-007): nothing expanded
    /// for it, so leaving that card ends a hover all the same.
    pointer_on_link: bool,
    menu_open: bool,
    /// The pointer rested on `pointer_on` while a menu was open: it expands once the menu closes.
    rest_held: bool,
    /// Keyboard focus moved while a menu was open: it settles once the menu closes.
    focus_held: bool,
    /// The card the Overview's opening put the keyboard on, until its focus report arrives.
    open_focus: Option<SessionLocation>,
    body_edges: (bool, bool),
    /// Which part of `focused` holds the keyboard (feature 1122, FR-008).
    focus_part: FocusPart,
    /// The link whose focus the open card menu took (FR-009): its loss waits for the next focus
    /// report ("held across the menu").
    link_held: Option<(SessionLocation, CardLink)>,
    /// The previous event, when it was a focus loss of the focused card or its link.
    just_lost: Option<Box<JustLost>>,
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

    /// The card that holds the keyboard, itself or one of its links (feature 1123: the root tells
    /// Shift+Tab from another card onto this card's link from a move inside the card).
    pub fn focused(&self) -> Option<&SessionLocation> {
        self.focused.as_ref()
    }

    /// The card and which of its links holds the keyboard (feature 1122, FR-008; feature 1123):
    /// not while an open menu holds it (FR-009).
    pub fn focused_link(&self) -> Option<(&SessionLocation, CardLink)> {
        let FocusPart::Link(link) = self.focus_part else {
            return None;
        };
        self.focused
            .as_ref()
            .filter(|_| self.link_held.is_none())
            .map(|location| (location, link))
    }

    /// Take in `event` (K1, data-model *Events and transitions*).
    pub fn apply(&mut self, event: Event) {
        // "Just held" lasts exactly one event (K5).
        let just_lost = self.just_lost.take();
        match event {
            Event::PointerRested(location) => {
                // Escape collapsed this card with the pointer still on it: it stays collapsed
                // until the pointer leaves it.
                let escaped_here =
                    self.pointer_escaped && self.pointer_on.as_ref() == Some(&location);
                if escaped_here {
                    return;
                }
                self.pointer_on = Some(location.clone());
                self.pointer_escaped = false;
                self.pointer_on_link = false;
                // FR-012: no other card expands under an open menu; this one does once it closes.
                self.rest_held = self.menu_open;
                if !self.menu_open {
                    self.expand(location, Cause::Pointer);
                }
            }
            Event::LinkRested(location) => {
                let same_card = self.pointer_on.as_ref() == Some(&location);
                if self.menu_open {
                    // The pointer moved from its card onto the card's link under the menu: the
                    // rest the menu held for the card is gone, so nothing expands on close
                    // (review A F3, M3). Another card's link changes nothing (FR-012).
                    if same_card {
                        self.rest_held = false;
                        self.pointer_on_link = true;
                    }
                    return;
                }
                // Escape collapsed this card with the pointer on it: moving onto its own link is
                // no leave, so it stays collapsed (1121; review A F1, M3).
                if !same_card {
                    self.pointer_escaped = false;
                }
                self.pointer_on = Some(location.clone());
                self.pointer_on_link = true;
                // A rest on a link counts as a rest on its card (FR-007): another card expanded
                // would cover the link, so it collapses; the link's own card keeps its form.
                if self
                    .expanded()
                    .is_some_and(|expanded| *expanded != location)
                {
                    self.collapse();
                }
            }
            Event::PointerLeft(location) => {
                let mut left_link = false;
                if self.pointer_on.as_ref() == Some(&location) {
                    self.pointer_on = None;
                    self.pointer_escaped = false;
                    self.rest_held = false;
                    left_link = std::mem::take(&mut self.pointer_on_link);
                }
                let hover_ended = self.expanded == Some((location, Cause::Pointer))
                    || (left_link && self.expanded.is_none());
                if hover_ended && !self.menu_open {
                    self.end_hover();
                }
            }
            Event::FocusGained(location) => self.gained(location, FocusPart::Card, just_lost),
            Event::FocusLost(location) => self.lost(location, FocusPart::Card),
            Event::LinkFocusGained(location, link) => {
                self.gained(location, FocusPart::Link(link), just_lost);
            }
            Event::LinkFocusLost(location, link) => self.lost(location, FocusPart::Link(link)),
            Event::OpenFocused(location) => self.open_focus = Some(location),
            Event::Escape => {
                if let Some((location, _)) = self.expanded.take() {
                    self.body_edges = (false, false);
                    self.focus_escaped |= self.focused.as_ref() == Some(&location);
                    self.pointer_escaped |= self.pointer_on.as_ref() == Some(&location);
                    // A rest the menu held back does not undo this collapse on close.
                    self.rest_held = false;
                }
            }
            Event::WindowBlurred => {
                self.rest_held = false;
                // The loss the link's menu deferred happens now: no link holds the keyboard under
                // the menu once the window has lost it (review A F3, M4).
                if let Some((held, _)) = self.link_held.take() {
                    self.lost_plain(held);
                }
                self.focus_held = false;
                self.collapse();
            }
            Event::MenuOpened => self.menu_open = true,
            Event::MenuClosed => {
                self.menu_open = false;
                // What the open menu held back (review A F2, F3, M3): a hover that ended under it
                // ends now (FR-005), and a rest it held back expands its card now.
                let hovered = self
                    .expanded
                    .as_ref()
                    .and_then(|(location, cause)| (*cause == Cause::Pointer).then_some(location));
                if hovered.is_some_and(|location| self.pointer_on.as_ref() != Some(location)) {
                    self.end_hover();
                }
                if std::mem::take(&mut self.rest_held) {
                    if let Some(location) = self.pointer_on.clone() {
                        self.expand(location, Cause::Pointer);
                    }
                }
                // A focus move it held back settles now, as the pointer's does (M3 follow-up).
                if std::mem::take(&mut self.focus_held) {
                    self.settle_focus();
                }
            }
            Event::Listed(listed) => {
                // The root lists after every update (`settle_card_menu`): a listing that changes
                // nothing is no event between a card's loss and its link's gain, so it keeps the
                // pairing (K5, review A F1, M4).
                let unchanged = self.clone();
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
                if gone(
                    &self
                        .link_held
                        .as_ref()
                        .map(|(location, _)| location.clone()),
                ) {
                    self.link_held = None;
                }
                if gone(&self.pointer_on) {
                    self.pointer_on = None;
                    self.pointer_escaped = false;
                    self.pointer_on_link = false;
                }
                // The opening's focus will never report for a card no longer listed (r2 F1, M4).
                if gone(&self.open_focus) {
                    self.open_focus = None;
                }
                if *self == unchanged {
                    self.just_lost = just_lost;
                }
            }
            Event::BodyScrolled { up, down } => {
                if self.expanded.is_some() {
                    self.body_edges = (up, down);
                }
            }
        }
    }

    /// `part` of `location` received the keyboard (K5): a move inside the card that holds it, or
    /// just held it, changes no form; anything else is a new focus (1121).
    fn gained(
        &mut self,
        location: SessionLocation,
        part: FocusPart,
        just_lost: Option<Box<JustLost>>,
    ) {
        if let Some((held, _)) = self.link_held.take() {
            if held == location {
                // The return from the link's menu, to the link or to its card (FR-009).
                self.focused = Some(location);
                self.focus_part = part;
                return;
            }
            // Any other report: the loss the menu deferred happens now, then this report.
            self.lost_plain(held);
        }
        if let Some(lost) = just_lost {
            // A loss of any other part of this card, just before: one move inside it (R6).
            if lost.location == location && lost.part != part {
                *self = lost.before;
                self.focus_part = part;
                return;
            }
        }
        if self.focused.as_ref() == Some(&location) {
            if self.focus_part == part {
                // It already holds the keyboard: a repeated report (the root's hand-off from a
                // link that went to its card, contract U6) changes nothing.
                return;
            }
            // Another part of the card holds the keyboard: a move inside the card, gain first.
            self.focus_part = part;
            return;
        }
        self.focus_part = part;
        // The opening's own focus (R13): focused, and collapsed as Escape leaves a card,
        // so nothing expands until focus moves (FR-013, review A F1, M4).
        let opening = self.open_focus.take().as_ref() == Some(&location);
        self.focused = Some(location.clone());
        self.focus_escaped = opening;
        if opening {
            return;
        }
        self.focus_held |= self.menu_open;
        if !self.menu_open {
            self.expand(location, Cause::Focus);
        }
    }

    /// `part` of `location` lost the keyboard (K5).
    fn lost(&mut self, location: SessionLocation, part: FocusPart) {
        if self.focused.as_ref() == Some(&location) {
            if part != self.focus_part {
                // A later part already reported its gain: the old half of a move inside the card
                // (or a stale report) changes nothing (R6).
                return;
            }
            if let FocusPart::Link(link) = part {
                if self.menu_open {
                    // The link's Menu key opened the menu, which took the keyboard (FR-009).
                    self.link_held = Some((location, link));
                    return;
                }
            }
            self.just_lost = Some(Box::new(JustLost {
                location: location.clone(),
                part,
                before: self.clone(),
            }));
        }
        self.lost_plain(location);
    }

    /// A focus loss as 1121 defines it.
    fn lost_plain(&mut self, location: SessionLocation) {
        self.open_focus = None;
        if self.focused.as_ref() == Some(&location) {
            self.focused = None;
            self.focus_escaped = false;
        }
        self.focus_held |= self.menu_open;
        if self.expanded == Some((location.clone(), Cause::Focus)) && !self.menu_open {
            // A press on the card (its inner scrollbar) hides the focus with the pointer
            // still resting on it: hovered, so it stays, by the pointer (review A F3, M4).
            let hovered = !self.pointer_escaped && self.pointer_on.as_ref() == Some(&location);
            if hovered {
                self.expanded = Some((location, Cause::Pointer));
            } else {
                self.collapse();
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

    /// Collapse the card the pointer expanded: the hover ended on no other card, so the focused
    /// card expands again, unless Escape collapsed it since it got focus (FR-005).
    fn end_hover(&mut self) {
        self.collapse();
        if let Some(focused) = self.focused.clone().filter(|_| !self.focus_escaped) {
            self.expand(focused, Cause::Focus);
        }
    }

    /// After focus moved under a menu: a card focus left collapses, and with no card expanded the
    /// focused one expands, unless Escape collapsed it since it got focus.
    fn settle_focus(&mut self) {
        let stale = self.expanded.as_ref().is_some_and(|(location, cause)| {
            *cause == Cause::Focus && self.focused.as_ref() != Some(location)
        });
        if stale {
            self.collapse();
        }
        if self.expanded.is_none() {
            if let Some(focused) = self.focused.clone().filter(|_| !self.focus_escaped) {
                self.expand(focused, Cause::Focus);
            }
        }
    }

    /// Collapse the expanded card, if any.
    fn collapse(&mut self) {
        self.expanded = None;
        self.body_edges = (false, false);
    }
}

/// What Page Up / Page Down scroll while a card has the keyboard (K4, FR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTarget {
    /// The expanded card's inner body.
    Card,
}

/// Whether Page Down (`down`) or Page Up goes to the expanded card (K4): only to a card expanded
/// by focus whose body can still move that way; else `None`, and the key behaves as today.
pub fn page_target(expansion: &Expansion, down: bool) -> Option<PageTarget> {
    let (up_room, down_room) = expansion.body_edges();
    let room = if down { down_room } else { up_room };
    (expansion.cause() == Some(Cause::Focus) && room).then_some(PageTarget::Card)
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

    /// FR-013, R13 (review A F1, M4): the focus the Overview's opening puts on its first card
    /// expands nothing, nor does a hover ending elsewhere; focus moving on expands as usual, and
    /// moving back onto that card expands it.
    #[test]
    fn the_opening_focus_expands_nothing() {
        let opened = || [Event::OpenFocused(loc("a")), focus("a")];
        assert_eq!(shown(&after(opened())), None);
        let hovered = opened().into_iter().chain([rest("b"), leave("b")]);
        assert_eq!(shown(&after(hovered)), None);
        let lost = || {
            opened()
                .into_iter()
                .chain([Event::FocusLost(loc("a")), focus("b")])
        };
        assert_eq!(shown(&after(lost())), Some("b".into()));
        let back = lost().chain([Event::FocusLost(loc("b")), focus("a")]);
        assert_eq!(shown(&after(back)), Some("a".into()));
    }

    /// Review A/B r2 F1 (M4): a listing without the opening's card drops its pending opening
    /// focus, so that card's later genuine focus (re-listed) expands it instead of being taken
    /// for the opening's.
    #[test]
    fn a_removed_opening_card_drops_its_opening_focus() {
        let relisted = [
            Event::OpenFocused(loc("a")),
            Event::Listed(vec![loc("b")]),
            Event::Listed(vec![loc("a"), loc("b")]),
            focus("a"),
        ];
        assert_eq!(shown(&after(relisted)), Some("a".into()));
        let kept = [
            Event::OpenFocused(loc("a")),
            Event::Listed(vec![loc("a"), loc("b")]),
            focus("a"),
        ];
        assert_eq!(shown(&after(kept)), None);
    }

    /// FR-005 (review A F3, M4): a press on the focus-expanded card it rests on (its inner
    /// scrollbar) hides the focus but leaves the pointer on it: the card stays expanded, now by
    /// the pointer, and collapses when the pointer leaves. Off the card, the focus loss collapses.
    #[test]
    fn a_focus_loss_under_the_resting_pointer_keeps_the_card_by_hover() {
        let pressed = after([rest("a"), focus("a"), Event::FocusLost(loc("a"))]);
        assert_eq!(shown(&pressed), Some("a".into()));
        assert_eq!(pressed.cause(), Some(Cause::Pointer));
        let left = after([
            rest("a"),
            focus("a"),
            Event::FocusLost(loc("a")),
            leave("a"),
        ]);
        assert_eq!(shown(&left), None);
        let elsewhere = after([rest("b"), focus("a"), Event::FocusLost(loc("a"))]);
        assert_eq!(shown(&elsewhere), None);
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
            None,
            "a blur left the hovered card expanded"
        );
        assert_eq!(
            shown(&after([
                focus("a"),
                rest("b"),
                Event::WindowBlurred,
                leave("b")
            ])),
            None,
            "the focused card re-expanded after the blur"
        );
        assert_eq!(
            shown(&after([focus("a"), Event::WindowBlurred])),
            None,
            "a blur left the focused card expanded"
        );
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
            Some("a".into()),
            "a rest or focus under the open menu moved the expansion"
        );
        assert_eq!(
            shown(&after([Event::MenuOpened, rest("b"), focus("b")])),
            None,
            "a rest or focus under the open menu expanded a card"
        );
        let reopened = [Event::MenuOpened, rest("b"), Event::MenuClosed, rest("b")];
        assert_eq!(
            shown(&after(reopened)),
            Some("b".into()),
            "a rest after the menu closed did not expand"
        );
        assert_eq!(
            shown(&after([Event::MenuOpened, Event::MenuClosed, focus("b")])),
            Some("b".into()),
            "a focus after the menu closed did not expand"
        );
    }

    /// K1, FR-005, FR-012 (review A F2, M3): a pointer leaving the hover-expanded card while a
    /// menu is open collapses it once the menu closes, and the focused card expands again.
    #[test]
    fn a_leave_during_an_open_menu_collapses_on_close() {
        let left = [rest("a"), Event::MenuOpened, leave("a")];
        assert_eq!(
            shown(&after(left.clone())),
            Some("a".into()),
            "collapsed under the menu"
        );
        let closed = left.clone().into_iter().chain([Event::MenuClosed]);
        assert_eq!(shown(&after(closed)), None);
        let focused = [focus("f")]
            .into_iter()
            .chain(left)
            .chain([Event::MenuClosed]);
        let back = after(focused);
        assert_eq!(shown(&back), Some("f".into()));
        assert_eq!(back.cause(), Some(Cause::Focus));
    }

    /// K1, US4 sc.3 (review A F3, M3): a rest held back by an open menu expands its card once
    /// the menu closes, unless the pointer left it first or Escape collapsed it there.
    #[test]
    fn a_rest_during_an_open_menu_expands_on_close() {
        let rested = [rest("a"), Event::MenuOpened, leave("a"), rest("b")];
        assert_eq!(
            shown(&after(rested.clone())),
            Some("a".into()),
            "b expanded under the menu"
        );
        let closed = after(rested.into_iter().chain([Event::MenuClosed]));
        assert_eq!(shown(&closed), Some("b".into()));
        assert_eq!(closed.cause(), Some(Cause::Pointer));
        let gone = [Event::MenuOpened, rest("b"), leave("b"), Event::MenuClosed];
        assert_eq!(shown(&after(gone)), None);
        let escaped = [
            rest("b"),
            Event::Escape,
            Event::MenuOpened,
            rest("b"),
            Event::MenuClosed,
        ];
        assert_eq!(shown(&after(escaped)), None);
    }

    /// K1, FR-012 (review A F1, M3 round 2): a rest held back by an open menu does not expand on
    /// close once Escape or a window blur collapsed the card under the menu.
    #[test]
    fn a_rest_held_by_the_menu_is_dropped_by_escape_or_blur() {
        for dismiss in [Event::Escape, Event::WindowBlurred] {
            let events = [
                rest("a"),
                Event::MenuOpened,
                leave("a"),
                rest("a"),
                dismiss.clone(),
                Event::MenuClosed,
            ];
            assert_eq!(shown(&after(events)), None, "after {dismiss:?}");
        }
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

    fn edges(up: bool, down: bool) -> Event {
        Event::BodyScrolled { up, down }
    }

    /// K4, FR-008: Page Down / Page Up go to the card only while it is expanded by focus and its
    /// body can move that way.
    #[test]
    fn page_keys_target_a_focus_expanded_card_that_can_move_that_way() {
        let focused = after([focus("a"), edges(false, true)]);
        assert_eq!(page_target(&focused, true), Some(PageTarget::Card));
        assert_eq!(page_target(&focused, false), None, "at its top");
        let at_end = after([focus("a"), edges(true, false)]);
        assert_eq!(page_target(&at_end, true), None, "at its end");
        assert_eq!(page_target(&at_end, false), Some(PageTarget::Card));
    }

    /// K4: no card expanded, a card expanded by the pointer, or a body that does not overflow:
    /// the keys are not the card's.
    #[test]
    fn page_keys_are_not_the_cards_without_a_focus_expanded_overflowing_body() {
        let cases = [
            ("nothing expanded", after([])),
            ("pointer cause", after([rest("a"), edges(true, true)])),
            ("no overflow", after([focus("a")])),
        ];
        for (label, expansion) in cases {
            for down in [false, true] {
                assert_eq!(page_target(&expansion, down), None, "{label}, down {down}");
            }
        }
    }

    /// K4 (carried from M2): an expand starts the body's edges at `(false, false)`, so a page
    /// claim never follows the edges of a card expanded before.
    #[test]
    fn an_expand_resets_the_body_edges() {
        let refocused = after([focus("a"), edges(true, true), focus("b")]);
        assert_eq!(refocused.body_edges(), (false, false));
        let again = after([
            focus("a"),
            edges(true, true),
            Event::Escape,
            Event::FocusLost(loc("a")),
            focus("a"),
        ]);
        assert_eq!(again.body_edges(), (false, false));
    }

    /// FR-005, FR-012 (M3 review A F2/F3 for focus): focus that moved under an open menu settles
    /// when it closes: the card focus left collapses, the card it reached expands.
    #[test]
    fn focus_moved_under_an_open_menu_settles_on_close() {
        let moved = [
            focus("a"),
            Event::MenuOpened,
            Event::FocusLost(loc("a")),
            focus("b"),
        ];
        let held = after(moved.clone());
        assert_eq!(shown(&held), Some("a".into()), "frozen while open");
        let closed = after(moved.into_iter().chain([Event::MenuClosed]));
        assert_eq!(shown(&closed), Some("b".into()));
        assert_eq!(closed.cause(), Some(Cause::Focus));
        let left = after([
            focus("a"),
            Event::MenuOpened,
            Event::FocusLost(loc("a")),
            Event::MenuClosed,
        ]);
        assert_eq!(shown(&left), None, "focus left the grid under the menu");
    }

    /// FR-005: a window blur under the menu drops what focus did there: nothing expands on close.
    #[test]
    fn focus_held_by_the_menu_is_dropped_by_a_blur() {
        let blurred = after([
            Event::MenuOpened,
            focus("b"),
            Event::WindowBlurred,
            Event::MenuClosed,
        ]);
        assert_eq!(shown(&blurred), None);
    }

    fn link_rest(name: &str) -> Event {
        Event::LinkRested(loc(name))
    }

    /// Feature 1122 K5, FR-007, Story 4 sc.2: a rest on a collapsed card's link expands nothing;
    /// a rest on the rest of that card then expands it as before.
    #[test]
    fn a_rest_on_a_collapsed_cards_link_expands_nothing() {
        assert_eq!(shown(&after([link_rest("a")])), None);
        let then_card = after([link_rest("a"), rest("a")]);
        assert_eq!(shown(&then_card), Some("a".into()));
        assert_eq!(then_card.cause(), Some(Cause::Pointer));
    }

    /// K5, FR-007, Story 4 sc.3: an expanded card stays expanded while the pointer is on its link.
    #[test]
    fn an_expanded_card_stays_expanded_under_a_rest_on_its_link() {
        let kept = after([rest("a"), link_rest("a")]);
        assert_eq!(shown(&kept), Some("a".into()));
        assert_eq!(kept.cause(), Some(Cause::Pointer));
        let focused = after([focus("a"), link_rest("a")]);
        assert_eq!(shown(&focused), Some("a".into()));
    }

    /// K5, FR-007 (1121 FR-005's re-expansion): a rest on another card's link counts as a rest on
    /// that card, so the focused card does not cover it; leaving that card brings it back.
    #[test]
    fn a_rest_on_another_cards_link_keeps_the_focused_card_collapsed() {
        let on_link = || [focus("a"), rest("b"), leave("b"), link_rest("c")];
        assert_eq!(shown(&after(on_link())), None);
        let left = after(on_link().into_iter().chain([leave("c")]));
        assert_eq!(shown(&left), Some("a".into()));
        assert_eq!(left.cause(), Some(Cause::Focus));
    }

    /// K5, FR-012 (1121): a rest on a link under an open menu changes nothing.
    #[test]
    fn a_rest_on_a_link_under_an_open_menu_changes_nothing() {
        let before = after([rest("a"), Event::MenuOpened]);
        let mut rested = before.clone();
        rested.apply(link_rest("b"));
        assert_eq!(rested, before);
        assert_eq!(
            shown(&after([
                rest("a"),
                Event::MenuOpened,
                link_rest("b"),
                Event::MenuClosed
            ])),
            Some("a".into())
        );
    }

    /// K5 with 1121's Escape rule (review A F1, M3): an Escape-collapsed card stays collapsed while
    /// the pointer moves onto its link and back, until the pointer leaves it.
    #[test]
    fn an_escaped_card_stays_collapsed_across_a_rest_on_its_link() {
        let back = || [rest("a"), Event::Escape, link_rest("a"), rest("a")];
        assert_eq!(shown(&after(back())), None);
        let left = after(back().into_iter().chain([leave("a"), rest("a")]));
        assert_eq!(shown(&left), Some("a".into()));
    }

    /// K5, FR-012 (review A F3, M3): a rest the menu held for a card is dropped when the pointer
    /// then rests on that card's link, so closing the menu expands nothing.
    #[test]
    fn a_held_rest_is_dropped_by_a_rest_on_that_cards_link() {
        let closed = after([
            Event::MenuOpened,
            rest("a"),
            link_rest("a"),
            Event::MenuClosed,
        ]);
        assert_eq!(shown(&closed), None);
    }

    fn link_focus(name: &str) -> Event {
        Event::LinkFocusGained(loc(name), CardLink::Issue)
    }

    fn link_lost(name: &str) -> Event {
        Event::LinkFocusLost(loc(name), CardLink::Issue)
    }

    fn lost(name: &str) -> Event {
        Event::FocusLost(loc(name))
    }

    /// Focus on A's card, then forward Tab onto A's link (card lost, then link gained).
    fn tabbed_to_link(name: &str) -> Vec<Event> {
        vec![focus(name), lost(name), link_focus(name)]
    }

    /// K5, FR-008, Story 3 sc.1: forward Tab from a card to its own link is a move inside the
    /// card: it stays expanded and focused.
    #[test]
    fn tab_from_a_card_to_its_link_keeps_it_expanded() {
        let tabbed = after(tabbed_to_link("a"));
        assert_eq!(shown(&tabbed), Some("a".into()));
        assert_eq!(tabbed.cause(), Some(Cause::Focus));
        assert_eq!(tabbed.focused, Some(loc("a")));
        assert_eq!(tabbed.focused_link(), Some((&loc("a"), CardLink::Issue)));
    }

    /// K5, FR-008: after Escape, the same Tab does not re-expand the card.
    #[test]
    fn tab_to_the_link_after_escape_stays_collapsed() {
        let escaped = after([focus("a"), Event::Escape, lost("a"), link_focus("a")]);
        assert_eq!(shown(&escaped), None);
        assert!(escaped.focus_escaped);
        assert_eq!(escaped.focused, Some(loc("a")));
    }

    /// K5, FR-008, Story 3 sc.1: Shift+Tab from the link back to its card (card gained, then link
    /// lost) keeps an escaped card collapsed and focused.
    #[test]
    fn shift_tab_from_the_link_to_its_card_keeps_the_form() {
        let mut events = tabbed_to_link("a");
        events.extend([Event::Escape, focus("a"), link_lost("a")]);
        let back = after(events);
        assert_eq!(shown(&back), None);
        assert_eq!(back.focused, Some(loc("a")));
        assert!(back.focus_escaped);
        assert_eq!(back.focused_link(), None);
    }

    /// K5, FR-008: Tab from A's link to the next card moves the expansion to B.
    #[test]
    fn tab_from_a_link_to_the_next_card_expands_that_card() {
        let mut events = tabbed_to_link("a");
        events.extend([link_lost("a"), focus("b")]);
        let next = after(events);
        assert_eq!(shown(&next), Some("b".into()));
        assert_eq!(next.focused, Some(loc("b")));
        assert_eq!(next.focused_link(), None);
    }

    /// K5 "just held": any other event between the loss and the link's gain ends the pairing:
    /// the gain is a new focus (A expands, the escape is cleared).
    #[test]
    fn an_event_between_loss_and_link_gain_makes_a_new_focus() {
        let apart = after([
            focus("a"),
            Event::Escape,
            lost("a"),
            leave("z"),
            link_focus("a"),
        ]);
        assert_eq!(shown(&apart), Some("a".into()));
        assert!(!apart.focus_escaped);
    }

    /// The link of A holds the keyboard, its Menu key opens the menu, which takes the keyboard.
    fn link_menu_opened(escaped: bool) -> Vec<Event> {
        let mut events = tabbed_to_link("a");
        if escaped {
            events.push(Event::Escape);
        }
        events.extend([Event::MenuOpened, link_lost("a")]);
        events
    }

    /// K5, FR-009, Story 3 sc.5: closing the link's menu returns to the link without
    /// re-expanding a card collapsed by Escape.
    #[test]
    fn the_link_menu_returns_to_an_escaped_card_collapsed() {
        let mut events = link_menu_opened(true);
        events.extend([Event::MenuClosed, link_focus("a")]);
        let back = after(events);
        assert_eq!(shown(&back), None);
        assert!(back.focus_escaped);
        assert_eq!(back.focused_link(), Some((&loc("a"), CardLink::Issue)));
    }

    /// K5, FR-009: without Escape the card stays expanded across the link's menu.
    #[test]
    fn the_link_menu_keeps_an_expanded_card_expanded() {
        let mut events = link_menu_opened(false);
        let open = after(events.clone());
        assert_eq!(shown(&open), Some("a".into()));
        assert_eq!(open.link_held, Some((loc("a"), CardLink::Issue)));
        events.extend([Event::MenuClosed, link_focus("a")]);
        let back = after(events);
        assert_eq!(shown(&back), Some("a".into()));
        assert_eq!(back.link_held, None);
    }

    /// K5, FR-009: a menu choice that moves the keyboard to B applies the deferred loss first.
    #[test]
    fn a_link_menu_choice_moving_focus_collapses_the_card() {
        let mut events = link_menu_opened(false);
        events.extend([Event::MenuClosed, focus("b")]);
        let moved = after(events);
        assert_eq!(shown(&moved), Some("b".into()));
        assert_eq!(moved.focused, Some(loc("b")));
        assert_eq!(moved.link_held, None);
    }

    /// K5, R9: with the link gone the return lands on the card (`FocusGained(A)`), still a move
    /// inside the card.
    #[test]
    fn the_link_menu_return_falls_back_to_the_card() {
        for escaped in [true, false] {
            let mut events = link_menu_opened(escaped);
            events.extend([Event::MenuClosed, focus("a")]);
            let back = after(events);
            let expected = (!escaped).then(|| "a".to_owned());
            assert_eq!(shown(&back), expected, "escaped: {escaped}");
            assert_eq!(back.focus_escaped, escaped);
            assert_eq!(back.focused, Some(loc("a")));
            assert_eq!(back.focused_link(), None);
        }
    }

    /// K5: a focus report while the link's menu is still open applies the deferred loss under
    /// the menu, so the move settles when it closes (1121).
    #[test]
    fn a_focus_report_under_the_link_menu_settles_on_close() {
        let mut events = link_menu_opened(false);
        events.push(focus("b"));
        let open = after(events.clone());
        assert_eq!(shown(&open), Some("a".into()), "frozen while open");
        assert_eq!(open.link_held, None);
        events.push(Event::MenuClosed);
        assert_eq!(shown(&after(events)), Some("b".into()));
    }

    /// K5: the card's own menu is unchanged from 1121: its focus loss under the menu is not held.
    #[test]
    fn the_cards_own_menu_is_unchanged() {
        let events = [focus("a"), Event::MenuOpened, lost("a")];
        let open = after(events.clone());
        assert_eq!(open.link_held, None);
        assert_eq!(open.focused, None);
        let closed = after(events.into_iter().chain([Event::MenuClosed]));
        assert_eq!(shown(&closed), None);
    }

    /// K5, Edge Case *Card goes away*: a listing without A while A's link holds the keyboard
    /// clears the focus, held or not.
    #[test]
    fn a_listing_without_the_links_card_clears_its_focus() {
        let mut events = tabbed_to_link("a");
        events.push(Event::Listed(vec![loc("b")]));
        let gone = after(events);
        assert_eq!(gone.focused, None);
        assert_eq!(gone.focused_link(), None);
        let mut held = link_menu_opened(false);
        held.push(Event::Listed(vec![loc("b")]));
        let held = after(held);
        assert_eq!(held.focused, None);
        assert_eq!(held.link_held, None);
    }

    /// K5, contract U6: the root hands the keyboard from a link that went to its card as a move
    /// inside the card (card gained, link lost); the card's own report that follows changes
    /// nothing, so an escaped card stays collapsed.
    #[test]
    fn a_hand_off_from_a_gone_link_to_its_card_keeps_the_form() {
        let mut events = tabbed_to_link("a");
        events.extend([Event::Escape, focus("a"), link_lost("a"), focus("a")]);
        let handed = after(events);
        assert_eq!(shown(&handed), None);
        assert!(handed.focus_escaped);
        assert_eq!(handed.focused, Some(loc("a")));
        assert_eq!(handed.focused_link(), None);
    }

    /// K5, FR-008 (review A F1, M4): the root's listing after every update, unchanged, sits
    /// between the card's loss and its link's gain and keeps the pairing: Tab onto an escaped
    /// card's link leaves it collapsed and the card the pointer expanded expanded.
    #[test]
    fn an_unchanged_listing_keeps_the_pairing_of_a_tab_onto_the_link() {
        let tabbed = after([
            focus("a"),
            Event::Escape,
            rest("b"),
            lost("a"),
            Event::Listed(vec![loc("a"), loc("b")]),
            link_focus("a"),
        ]);
        assert_eq!(shown(&tabbed), Some("b".into()));
        assert_eq!(tabbed.cause(), Some(Cause::Pointer));
        assert!(tabbed.focus_escaped);
        assert_eq!(tabbed.focused_link(), Some((&loc("a"), CardLink::Issue)));
    }

    /// K5: a listing that changes something (another card gone) still ends the pairing.
    #[test]
    fn a_changed_listing_ends_the_pairing() {
        let apart = after([
            focus("a"),
            Event::Escape,
            lost("a"),
            Event::Listed(vec![loc("a")]),
            link_focus("a"),
        ]);
        assert_eq!(shown(&apart), None, "nothing gone: paired");
        let apart = after([
            focus("a"),
            Event::Escape,
            rest("b"),
            lost("a"),
            Event::Listed(vec![loc("a")]),
            link_focus("a"),
        ]);
        assert_eq!(shown(&apart), Some("a".into()), "b gone: a new focus");
        assert!(!apart.focus_escaped);
    }

    /// FR-009 (review A F3, M4): a window blur with the link's menu holding its focus applies the
    /// deferred loss, so no link is left holding the keyboard under the menu.
    #[test]
    fn a_blur_under_the_link_menu_applies_the_held_loss() {
        let mut events = link_menu_opened(false);
        events.push(Event::WindowBlurred);
        let blurred = after(events);
        assert_eq!(blurred.link_held, None);
        assert_eq!(blurred.focused, None);
        assert_eq!(blurred.focused_link(), None);
        assert_eq!(shown(&blurred), None);
    }

    /// A part of card `name`'s keyboard stops: `None` is the card, `Some(link)` one of its links.
    type Part = Option<CardLink>;

    const CARD: Part = None;
    const ISSUE: Part = Some(CardLink::Issue);

    fn pr(number: u64) -> Part {
        Some(CardLink::PullRequest(number))
    }

    fn gain(name: &str, part: Part) -> Event {
        match part {
            None => focus(name),
            Some(link) => Event::LinkFocusGained(loc(name), link),
        }
    }

    fn loss(name: &str, part: Part) -> Event {
        match part {
            None => lost(name),
            Some(link) => Event::LinkFocusLost(loc(name), link),
        }
    }

    /// The two reports of a keyboard move on card `name` from `from` to `to`, in either order
    /// (feature 1123, research R6): iced reports a move as one loss and one gain.
    fn moved(name: &str, from: Part, to: Part, gain_first: bool) -> Vec<Event> {
        if gain_first {
            vec![gain(name, to), loss(name, from)]
        } else {
            vec![loss(name, from), gain(name, to)]
        }
    }

    /// Card A focused (expanded by focus), its issue link tabbed to, then Escape if `escaped`.
    fn on_issue_link(escaped: bool, gain_first: bool) -> Vec<Event> {
        let mut events = vec![focus("a")];
        events.extend(moved("a", CARD, ISSUE, gain_first));
        if escaped {
            events.push(Event::Escape);
        }
        events
    }

    /// K5a (feature 1123, FR-010): a move from the issue link to a number on a focus-expanded
    /// card, in either report order, keeps it expanded and names the number.
    #[test]
    fn a_move_from_link_to_link_keeps_the_card_expanded() {
        for gain_first in [false, true] {
            let mut events = on_issue_link(false, gain_first);
            events.extend(moved("a", ISSUE, pr(12), gain_first));
            let moved = after(events);
            assert_eq!(shown(&moved), Some("a".into()), "gain first: {gain_first}");
            assert_eq!(
                moved.cause(),
                Some(Cause::Focus),
                "gain first: {gain_first}"
            );
            assert_eq!(
                moved.focused_link(),
                Some((&loc("a"), CardLink::PullRequest(12))),
                "gain first: {gain_first}"
            );
        }
    }

    /// K5b (FR-010, 1122 FR-008): after Escape the same move leaves the card collapsed.
    #[test]
    fn a_move_from_link_to_link_after_escape_stays_collapsed() {
        for gain_first in [false, true] {
            let mut events = on_issue_link(true, gain_first);
            events.extend(moved("a", ISSUE, pr(12), gain_first));
            let moved = after(events);
            assert_eq!(shown(&moved), None, "gain first: {gain_first}");
            assert!(moved.focus_escaped, "gain first: {gain_first}");
            assert_eq!(
                moved.focused_link(),
                Some((&loc("a"), CardLink::PullRequest(12))),
                "gain first: {gain_first}"
            );
        }
    }

    /// K5c: card → number and back, expanded or Escape-collapsed, in either report order: no form
    /// change; the number holds the keyboard, then nothing but the card.
    #[test]
    fn a_move_between_the_card_and_a_number_keeps_the_form() {
        for escaped in [false, true] {
            for gain_first in [false, true] {
                let case = format!("escaped: {escaped}, gain first: {gain_first}");
                let expected = (!escaped).then(|| "a".to_owned());
                let mut events = vec![focus("a")];
                if escaped {
                    events.push(Event::Escape);
                }
                events.extend(moved("a", CARD, pr(7), gain_first));
                let there = after(events.clone());
                assert_eq!(shown(&there), expected, "{case}");
                assert_eq!(
                    there.focused_link(),
                    Some((&loc("a"), CardLink::PullRequest(7))),
                    "{case}"
                );
                events.extend(moved("a", pr(7), CARD, gain_first));
                let back = after(events);
                assert_eq!(shown(&back), expected, "{case}");
                assert_eq!(back.focus_escaped, escaped, "{case}");
                assert_eq!(back.focused, Some(loc("a")), "{case}");
                assert_eq!(back.focused_link(), None, "{case}");
            }
        }
    }

    /// K5d: after a gain-first move from #1 to #2, #2 holds the keyboard; when it then loses it to
    /// another card's gain, in either order, the card collapses and that card expands.
    #[test]
    fn a_number_that_gained_last_loses_to_another_card() {
        for gain_first in [false, true] {
            let mut events = vec![focus("a")];
            events.extend(moved("a", CARD, pr(1), false));
            events.extend(moved("a", pr(1), pr(2), true));
            let on_two = after(events.clone());
            assert_eq!(shown(&on_two), Some("a".into()));
            assert_eq!(
                on_two.focused_link(),
                Some((&loc("a"), CardLink::PullRequest(2)))
            );
            if gain_first {
                events.extend([focus("b"), loss("a", pr(2))]);
            } else {
                events.extend([loss("a", pr(2)), focus("b")]);
            }
            let left = after(events);
            assert_eq!(shown(&left), Some("b".into()), "gain first: {gain_first}");
            assert_eq!(left.focused, Some(loc("b")), "gain first: {gain_first}");
            assert_eq!(left.focused_link(), None, "gain first: {gain_first}");
        }
    }

    /// K5e: a stale loss of #1, arriving after #2 gained and the card took the keyboard back (in
    /// either order), changes nothing.
    #[test]
    fn a_stale_loss_of_an_older_number_changes_nothing() {
        for escaped in [false, true] {
            for gain_first in [false, true] {
                let case = format!("escaped: {escaped}, gain first: {gain_first}");
                let mut events = vec![focus("a")];
                events.extend(moved("a", CARD, pr(1), false));
                if escaped {
                    events.push(Event::Escape);
                }
                events.push(gain("a", pr(2)));
                events.extend(moved("a", pr(2), CARD, gain_first));
                let before = after(events.clone());
                events.push(loss("a", pr(1)));
                let stale = after(events);
                assert_eq!(stale, before, "{case}");
                assert_eq!(shown(&stale), (!escaped).then(|| "a".to_owned()), "{case}");
                assert_eq!(stale.focused, Some(loc("a")), "{case}");
                assert_eq!(stale.focused_link(), None, "{case}");
            }
        }
    }

    /// K5f (FR-009 on a number): the number's Menu key opens the menu, which takes the keyboard:
    /// its loss is held, no link holds the keyboard under the menu, and the return gain restores
    /// the number; a blur applies the held loss, a listing without the card drops it.
    #[test]
    fn a_numbers_menu_holds_its_focus_until_the_return() {
        for gain_first in [false, true] {
            let case = format!("gain first: {gain_first}");
            let mut events = on_issue_link(false, gain_first);
            events.extend(moved("a", ISSUE, pr(5), gain_first));
            events.extend([Event::MenuOpened, loss("a", pr(5))]);
            let open = after(events.clone());
            assert_eq!(
                open.link_held,
                Some((loc("a"), CardLink::PullRequest(5))),
                "{case}"
            );
            assert_eq!(open.focused_link(), None, "{case}");
            assert_eq!(shown(&open), Some("a".into()), "{case}");

            let mut back = events.clone();
            back.extend([Event::MenuClosed, gain("a", pr(5))]);
            let back = after(back);
            assert_eq!(back.link_held, None, "{case}");
            assert_eq!(shown(&back), Some("a".into()), "{case}");
            assert_eq!(
                back.focused_link(),
                Some((&loc("a"), CardLink::PullRequest(5))),
                "{case}"
            );

            let mut blurred = events.clone();
            blurred.push(Event::WindowBlurred);
            let blurred = after(blurred);
            assert_eq!(blurred.link_held, None, "{case}");
            assert_eq!(blurred.focused, None, "{case}");
            assert_eq!(shown(&blurred), None, "{case}");

            let mut gone = events;
            gone.push(Event::Listed(vec![loc("b")]));
            let gone = after(gone);
            assert_eq!(gone.link_held, None, "{case}");
            assert_eq!(gone.focused, None, "{case}");
            assert_eq!(gone.focused_link(), None, "{case}");
        }
    }

    /// K2, SC-006: a card expands within half a second of the pointer resting on it.
    #[test]
    fn the_rest_is_under_half_a_second() {
        assert!(EXPAND_REST <= Duration::from_millis(500));
        assert!(!EXPAND_REST.is_zero());
    }
}
