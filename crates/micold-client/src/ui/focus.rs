//! What holds the keyboard, and where it is on screen — BUG-003, and FR-030.
//!
//! Two halves of one question. The first is which control has focus and how it says so; the second
//! is whether the user can see the control that has it.
//!
//! # Joining a text field to [`State::focused_field`](crate::app::State::focused_field) — BUG-003
//!
//! # Why this is one call and not two
//!
//! A focused field is two halves that must both be present: it reports focus
//! (`on_focus_change`) and it is *told* focus back (`active`). Either alone is a defect that
//! nothing catches. Reporting without being told is the whole of BUG-003 — the state was
//! observable and no view read it, so every field in the application drew permanently at rest for
//! two features running. Being told without reporting is the same field with a longer path to the
//! same nothing.
//!
//! Neither half looks wrong on its own, which is exactly why they were separable for so long. So
//! the two are one call here, and a field is either wired or visibly not (Principle V): there is no
//! spelling of half of it.
//!
//! # Why the library does not do this itself
//!
//! `FormField` takes `active` as a parameter on purpose — the state that thickens a filled field's
//! indicator is focus for a text input and *open* for a picker (§7.7), so the wrapper is told which
//! rather than assuming. That reasoning is sound and unchanged. What was missing was a way for a
//! caller to find out what to say, and where to keep it; this is the application's answer, and it
//! lives in the application because the choice of where the fact is held is the application's.

use crate::app::Message;
use crate::features::window::FieldId;
use crate::features::window::Msg as WindowMsg;
use crate::ui::material::{Checkbox, Select, TextField};

use iced::advanced::widget::operation::scrollable::{AbsoluteOffset, Scrollable};
use iced::advanced::widget::operation::{Focusable, Outcome};
use iced::advanced::widget::{operate, Id, Operation};
use iced::{Rectangle, Task, Vector};
use micold_core::tokens::spacing;

/// Wire an input's focus to the application's [`FieldId`].
pub trait TrackFocus {
    /// Report this input's focus as `id`, and draw it from whatever the application currently
    /// holds in `focused`.
    fn track_focus(self, id: FieldId, focused: Option<FieldId>) -> Self;
}

impl<'a> TrackFocus for TextField<'a, Message> {
    fn track_focus(self, id: FieldId, focused: Option<FieldId>) -> Self {
        self.active(focused == Some(id))
            .on_focus_change(move |focused| {
                Message::Window(WindowMsg::FieldFocusChanged(id, focused))
            })
    }
}

/// The checkbox joins on the same terms, which is the point of it being a trait: one `FieldId`
/// space and one message for every input in the application, so "which control has the keyboard"
/// has a single answer rather than one per kind of control.
impl<'a> TrackFocus for Checkbox<'a, Message> {
    fn track_focus(self, id: FieldId, focused: Option<FieldId>) -> Self {
        self.focused(focused == Some(id))
            .on_focus_change(move |focused| {
                Message::Window(WindowMsg::FieldFocusChanged(id, focused))
            })
    }
}

/// A select joins the one blur route (feature 937, contract C4, research R3): leaving it reports
/// `FieldFocusChanged(id, false)` like a text field's blur does, so the owning draft can mark it.
///
/// `focused` is unused: a select holds its own focus and draws it itself, so it is not told.
impl<'a, T> TrackFocus for Select<'a, T, Message>
where
    T: Clone + ToString + PartialEq + 'a,
{
    fn track_focus(self, id: FieldId, _focused: Option<FieldId>) -> Self {
        self.on_blur(Message::Window(WindowMsg::FieldFocusChanged(id, false)))
    }
}

/// How much clear space is left between a control the keyboard has just reached and the edge of
/// the panel it was scrolled into.
///
/// Without it a control can be *technically* visible — its last row of pixels flush against the
/// panel's edge — and read as cut off, which is the same failure FR-030 is about with an extra
/// step.
const MARGIN: f32 = spacing::MD;

/// Bring whatever currently holds the keyboard into view, if it is inside something that scrolls
/// (FR-030, second clause: "with the focused element visible").
///
/// # Why iced does not do this
///
/// Its focus operations answer "who has the keyboard" and nothing else — `focus_next` never sees a
/// scrollable and a scrollable never hears about focus. That is a fair division while every
/// focusable is a text field in a short form. It stops being fair the moment a surface is a rail
/// beside a scrolling page: Tab then walks into controls below the fold, the ring is painted
/// faithfully, and the user sees nothing move. The keyboard is somewhere they cannot see.
///
/// # Why it is two passes
///
/// A scrollable is told about itself *before* its children are traversed, so a single pass reaches
/// the panel while the focused control is still ahead of it. The first pass therefore only reads —
/// it records the focused control's rectangle — and chains a second that only writes. iced's own
/// `focus_next` is built the same way and for the same reason.
pub fn scroll_focused_into_view<M: Send + 'static>() -> Task<M> {
    operate(into_view())
}

/// The container id every dialog surface carries, so Tab can be confined to one (feature 1092,
/// FR-009, research R5). iced's `focus_next` walks the whole view, page behind the scrim included.
pub fn dialog_scope() -> Id {
    Id::new("dialog")
}

/// Tab (`forward`) or Shift+Tab within the open dialog only: the traversal is scoped to the
/// [`dialog_scope`] container, so the keyboard never reaches the page behind it.
pub fn dialog_focus_move<T: Send + 'static>(forward: bool) -> impl Operation<T> {
    use iced::advanced::widget::operation::{focusable, scope};
    let inner: Box<dyn Operation<T>> = if forward {
        Box::new(focusable::focus_next::<T>())
    } else {
        Box::new(focusable::focus_previous::<T>())
    };
    scope(dialog_scope(), BoxedOperation(inner))
}

/// [`dialog_focus_move`] as the runtime runs it.
pub fn dialog_focus_move_task<M: Send + 'static>(forward: bool) -> Task<M> {
    operate(dialog_focus_move(forward))
}

/// Tab (`forward`) or Shift+Tab over the whole view, as iced's own traversal runs it. Used by the
/// showcase, which moves the keyboard as the application's own `Message::FocusMoved` arm does.
pub fn focus_move_task<M: Send + 'static>(forward: bool) -> Task<M> {
    if forward {
        iced::widget::operation::focus_next()
    } else {
        iced::widget::operation::focus_previous()
    }
}

/// A boxed operation, which `scope` can take as the `impl Operation` it wants.
struct BoxedOperation<T>(Box<dyn Operation<T>>);

impl<T: 'static> Operation<T> for BoxedOperation<T> {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        self.0.traverse(operate);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        self.0.container(id, bounds);
    }

    fn focusable(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        self.0.focusable(id, bounds, state);
    }

    fn finish(&self) -> Outcome<T> {
        self.0.finish()
    }
}

/// The operation itself, for a caller that needs to drive it rather than hand it to the runtime.
///
/// [`scroll_focused_into_view`] is the shape the application uses, and a [`Task`] is opaque — it
/// cannot be run against a widget tree, so nothing built on it can answer whether the two passes
/// reach anything. This is the same operation before it is wrapped.
pub fn into_view<T>() -> impl Operation<T> {
    FindFocused { focused: None }
}

/// Where the focused control is in the window, or `None` when nothing holds the keyboard — a
/// read-only operation, for a caller that must anchor something to a control (decision D11).
///
/// Every layout in the pass is in untranslated content coordinates (see [`ShowFocused`]), so the
/// bounds a control reports are where it would be before its scrollables moved the content. The
/// translation of each scrollable that holds it comes off, which leaves where it is on screen. The
/// result is `Some(None)` rather than no result when nothing is focused, so a caller can tell the
/// pass ran.
pub fn focused_bounds() -> impl Operation<Option<Rectangle>> {
    FocusedBounds(ScreenBounds::default())
}

/// [`focused_bounds`] as the runtime runs it.
pub fn focused_bounds_task() -> Task<Option<Rectangle>> {
    operate(focused_bounds())
}

/// [`focused_bounds`]: the pass, finishing with what it found.
struct FocusedBounds(ScreenBounds);

impl Operation<Option<Rectangle>> for FocusedBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<Rectangle>>)) {
        self.0.traverse(operate);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.0.entered(bounds, translation);
    }

    fn focusable(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        self.0.offered(id, bounds, state.is_focused());
    }

    fn finish(&self) -> Outcome<Option<Rectangle>> {
        Outcome::Some(self.0.found)
    }
}

/// Where a widget is on screen: the focused control ([`focused_bounds`]), or each widget named by
/// [`Self::of_any`], which a caller drives over a subtree itself and reads with
/// [`Self::all_visible`] (a card's links in the Overview grid, feature 1122 R7, feature 1123 R6). Each scrollable holding it takes its
/// translation off the bounds it reports, and clips the part [`Self::visible`] reports to its
/// view.
#[derive(Default)]
pub(crate) struct ScreenBounds {
    /// The widgets looked for by their ids; the focused control when empty.
    targets: Vec<Id>,
    /// The translation of the scrollable just met and its view on screen, until the traversal
    /// into its content claims them.
    entering: Option<(Vector, Rectangle)>,
    /// For each traversal the pass is inside, the translation its scrollable applies (zero for a
    /// traversal that is not a scrollable's content) and the view on screen its content shows
    /// through, the innermost scrollable's clipped by those around it (`None` outside every
    /// scrollable).
    inside: Vec<(Vector, Option<Rectangle>)>,
    /// The first match on screen.
    found: Option<Rectangle>,
    /// The part of every match inside every scrollable view holding it, in traversal order; a
    /// match wholly out of view has none.
    all_visible: Vec<Rectangle>,
}

impl ScreenBounds {
    /// The pass that finds every widget named by one of `ids` (a container or a focusable
    /// control), in one traversal.
    pub(crate) fn of_any(ids: Vec<Id>) -> Self {
        Self {
            targets: ids,
            ..Self::default()
        }
    }

    /// The part on screen of each widget found, after the pass: its bounds clipped to the view of
    /// each scrollable holding it, so a link scrolled out of a card's body is not under a pointer
    /// on the card's title (feature 1122, review A M2 F2). One entry per match at least partly in
    /// view.
    pub(crate) fn all_visible(&self) -> &[Rectangle] {
        &self.all_visible
    }

    /// The sum of the translations of the scrollables the pass is inside.
    fn shift(&self) -> Vector {
        self.inside
            .iter()
            .fold(Vector::ZERO, |sum, (t, _)| sum + *t)
    }

    /// The view the pass is inside on screen, if any scrollable holds it.
    fn view(&self) -> Option<Rectangle> {
        self.inside.last().and_then(|(_, view)| *view)
    }

    /// A scrollable met at `bounds`, translating its content by `translation`.
    fn entered(&mut self, bounds: Rectangle, translation: Vector) {
        let own = bounds - self.shift();
        // Wholly outside the view around it: an empty view, which nothing intersects.
        let view = match self.view() {
            Some(outer) => outer
                .intersection(&own)
                .unwrap_or(Rectangle::new(own.position(), iced::Size::ZERO)),
            None => own,
        };
        self.entering = Some((translation, view));
    }

    fn take(&mut self, bounds: Rectangle) {
        let on_screen = bounds - self.shift();
        let visible = match self.view() {
            Some(view) => view.intersection(&on_screen),
            None => Some(on_screen),
        };
        if self.found.is_none() {
            self.found = Some(on_screen);
        }
        self.all_visible.extend(visible);
    }

    /// A focusable control offered to the pass: taken when it is the one looked for.
    fn offered(&mut self, id: Option<&Id>, bounds: Rectangle, focused: bool) {
        let wanted = if self.targets.is_empty() {
            focused
        } else {
            self.named(id)
        };
        if wanted {
            self.take(bounds);
        }
    }

    fn named(&self, id: Option<&Id>) -> bool {
        id.is_some_and(|id| self.targets.contains(id))
    }
}

impl<T> Operation<T> for ScreenBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        let level = match self.entering.take() {
            Some((translation, view)) => (translation, Some(view)),
            None => (Vector::ZERO, self.view()),
        };
        self.inside.push(level);
        operate(self);
        self.inside.pop();
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.entered(bounds, translation);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if self.named(id) {
            self.take(bounds);
        }
    }

    fn focusable(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        self.offered(id, bounds, state.is_focused());
    }

    fn finish(&self) -> Outcome<T> {
        Outcome::None
    }
}

/// Pass one: where is the keyboard?
struct FindFocused {
    focused: Option<Rectangle>,
}

impl<T> Operation<T> for FindFocused {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        if state.is_focused() {
            self.focused = Some(bounds);
        }
    }

    fn finish(&self) -> Outcome<T> {
        match self.focused {
            Some(focused) => Outcome::Chain(Box::new(ShowFocused { focused })),
            // Nothing holds the keyboard — the traversal ran off the end of a surface with no
            // inputs. Scrolling anything here would move the page for no reason.
            None => Outcome::None,
        }
    }
}

/// Pass two: move whatever is hiding it.
struct ShowFocused {
    focused: Rectangle,
}

impl<T> Operation<T> for ShowFocused {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        // A surface can hold several panels — the settings view's page scrolls and the rail beside
        // it does not — so a scrollable acts only on a control that is actually inside it. Every
        // layout in the pass shares one untranslated coordinate space (a scrollable hands its
        // children the *unshifted* content layout), so this comparison and the arithmetic below
        // are in the same units.
        if !content_bounds.intersects(&self.focused) {
            return;
        }

        scroll_panel_to(
            self.focused,
            MARGIN,
            bounds,
            content_bounds,
            translation,
            state,
        );
    }
}

/// Scroll one panel just far enough for `target` to be inside it with `margin` to spare; a target
/// already in view moves nothing. The shared tail of both passes that show a control or a row.
pub(super) fn scroll_panel_to(
    target: Rectangle,
    margin: f32,
    bounds: Rectangle,
    content_bounds: Rectangle,
    translation: Vector,
    state: &mut dyn Scrollable,
) {
    let delta = delta_into_view(
        target,
        bounds.height,
        content_bounds.y,
        translation.y,
        margin,
    );
    if delta != 0.0 {
        state.scroll_by(AbsoluteOffset { x: 0.0, y: delta }, bounds, content_bounds);
    }
}

/// How far a panel showing `viewport_height` of content from `translation_y` must scroll for
/// `focused` to be inside it with `margin` to spare. Positive scrolls the content up.
///
/// The margin is the caller's: a focused control takes [`MARGIN`], so its ring is not flush against
/// the panel's edge, and a picker's highlighted row takes none (`picker_scroll`), because a row
/// that is wholly visible must not move its list (feature 038, FR-007).
///
/// Split out because this is the part that can be wrong in a way no rendered check would name: it
/// is arithmetic on four numbers, and the failure it guards is "the control is on screen but flush
/// against the edge", which a screenshot shows and a layout gate does not.
pub(super) fn delta_into_view(
    focused: Rectangle,
    viewport_height: f32,
    content_top: f32,
    translation_y: f32,
    margin: f32,
) -> f32 {
    let top = content_top + translation_y;
    let bottom = top + viewport_height;
    let wanted_top = focused.y - margin;
    let wanted_bottom = focused.y + focused.height + margin;

    if wanted_top < top || wanted_bottom - wanted_top > viewport_height {
        // Above the fold — or too tall to fit at all, in which case aligning its top is the
        // useful half: that is where its label is and where typing goes. Checked here rather
        // than after, because the "is its bottom off-screen?" question is also true of a control
        // whose bottom can never be on-screen, and answering that one first scrolls past the part
        // the user needs.
        wanted_top - top
    } else if wanted_bottom > bottom {
        wanted_bottom - bottom
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::test_support::{roles, Mounted};
    use crate::ui::material::Button;
    use iced::advanced::widget::operation::scrollable::scroll_by;
    use iced::widget::{column, Scrollable, Space};
    use iced::Length;

    /// A panel 400dp tall showing content from `translation_y`, and a 56dp control at `y`.
    fn delta(control_y: f32, translation_y: f32) -> f32 {
        delta_into_view(
            Rectangle {
                x: 0.0,
                y: control_y,
                width: 300.0,
                height: 56.0,
            },
            400.0,
            0.0,
            translation_y,
            MARGIN,
        )
    }

    /// A control already comfortably inside the panel does not move it. Tab through a short form
    /// and the page must sit still — a view that lurches on every keystroke is worse than one that
    /// never scrolls.
    #[test]
    fn a_control_already_in_view_moves_nothing() {
        assert_eq!(delta(100.0, 0.0), 0.0);
    }

    /// The defect this exists for: the traversal reaches a control below the fold and the page
    /// follows it far enough that the whole control, plus its margin, is inside.
    #[test]
    fn a_control_below_the_fold_is_brought_up() {
        // Bottom edge at 856; the panel shows 0..400.
        let d = delta(800.0, 0.0);
        assert!(d > 0.0, "the panel did not scroll down to reach it");
        assert_eq!(800.0 + 56.0 + MARGIN - (0.0 + d + 400.0), 0.0);
    }

    /// And back the other way. Shift+Tab off the top of a scrolled page is the same defect
    /// mirrored, and it is the one a "scroll down to it" implementation forgets.
    #[test]
    fn a_control_above_the_fold_is_brought_down() {
        let d = delta(50.0, 500.0);
        assert!(d < 0.0, "the panel did not scroll up to reach it");
        assert_eq!(500.0 + d, 50.0 - MARGIN);
    }

    /// A control flush against the panel's edge is not "visible enough". Without the margin the
    /// arithmetic is satisfied by a row whose last pixels touch the boundary, which reads as cut
    /// off.
    #[test]
    fn a_control_flush_with_the_edge_is_still_moved() {
        // Bottom edge exactly at 400, the panel's own bottom.
        let d = delta(344.0, 0.0);
        assert_eq!(d, MARGIN);
    }

    /// A control taller than the panel cannot be shown whole. Showing its top is the useful half —
    /// that is where its label is, and where typing goes.
    #[test]
    fn a_control_taller_than_the_panel_is_aligned_to_its_top() {
        let tall = Rectangle {
            x: 0.0,
            y: 600.0,
            width: 300.0,
            height: 900.0,
        };
        let d = delta_into_view(tall, 400.0, 0.0, 0.0, MARGIN);
        assert_eq!(d, 600.0 - MARGIN);
    }

    /// The panel's own origin is not zero — it sits below an app bar and beside a rail — so the
    /// content top has to be subtracted rather than assumed away. Getting this wrong scrolls by
    /// the height of everything above the panel.
    #[test]
    fn the_panels_offset_on_screen_is_not_part_of_the_distance() {
        let control = Rectangle {
            x: 0.0,
            y: 1000.0,
            width: 300.0,
            height: 56.0,
        };
        assert_eq!(
            delta_into_view(control, 400.0, 120.0, 0.0, MARGIN),
            delta_into_view(
                Rectangle {
                    y: 880.0,
                    ..control
                },
                400.0,
                0.0,
                0.0,
                MARGIN
            )
        );
    }

    /// A scrollable showing a button 600dp down its content, scrolled by `scrolled`.
    fn scrolled_button(scrolled: f32) -> Mounted<'static> {
        let id = Id::new("list");
        let content = column![
            Space::new().height(Length::Fixed(600.0)),
            Button::filled("Start", roles()).on_press("start".to_string())
        ];
        let mut mounted = Mounted::new(
            Scrollable::new(content)
                .id(id.clone())
                .height(Length::Fixed(300.0)),
        );
        mounted.drive::<()>(scroll_by(
            id,
            AbsoluteOffset {
                x: 0.0,
                y: scrolled,
            },
        ));
        mounted
    }

    /// Decision D11: the focused control's bounds are where it is on screen, so the scrollable's
    /// translation comes off its layout position.
    #[test]
    fn the_focused_controls_bounds_are_reported_after_the_scrollables_translation() {
        let mut at_rest = scrolled_button(0.0);
        at_rest.focus_next();
        let rest = at_rest.drive(focused_bounds()).flatten().expect("focused");
        assert!(
            rest.y >= 600.0,
            "the button sits below the spacer: {rest:?}"
        );

        let mut scrolled = scrolled_button(250.0);
        scrolled.focus_next();
        let shown = scrolled.drive(focused_bounds()).flatten().expect("focused");
        assert_eq!(shown.y, rest.y - 250.0);
        assert_eq!(
            (shown.x, shown.width, shown.height),
            (rest.x, rest.width, rest.height)
        );
    }

    /// With nothing focused there is nothing to report.
    #[test]
    fn nothing_is_reported_when_nothing_is_focused() {
        assert_eq!(scrolled_button(0.0).drive(focused_bounds()), Some(None));
    }
}
