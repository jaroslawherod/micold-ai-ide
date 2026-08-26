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
use crate::ui::material::{Checkbox, TextField};

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
            .on_focus_change(move |focused| Message::FieldFocusChanged(id, focused))
    }
}

/// The checkbox joins on the same terms, which is the point of it being a trait: one `FieldId`
/// space and one message for every input in the application, so "which control has the keyboard"
/// has a single answer rather than one per kind of control.
impl<'a> TrackFocus for Checkbox<'a, Message> {
    fn track_focus(self, id: FieldId, focused: Option<FieldId>) -> Self {
        self.focused(focused == Some(id))
            .on_focus_change(move |focused| Message::FieldFocusChanged(id, focused))
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

/// The operation itself, for a caller that needs to drive it rather than hand it to the runtime.
///
/// [`scroll_focused_into_view`] is the shape the application uses, and a [`Task`] is opaque — it
/// cannot be run against a widget tree, so nothing built on it can answer whether the two passes
/// reach anything. This is the same operation before it is wrapped.
pub fn into_view<T>() -> impl Operation<T> {
    FindFocused { focused: None }
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

        let delta = delta_into_view(self.focused, bounds.height, content_bounds.y, translation.y);
        if delta != 0.0 {
            state.scroll_by(AbsoluteOffset { x: 0.0, y: delta }, bounds, content_bounds);
        }
    }
}

/// How far a panel showing `viewport_height` of content from `translation_y` must scroll for
/// `focused` to be inside it with [`MARGIN`] to spare. Positive scrolls the content up.
///
/// Split out because this is the part that can be wrong in a way no rendered check would name: it
/// is arithmetic on four numbers, and the failure it guards is "the control is on screen but flush
/// against the edge", which a screenshot shows and a layout gate does not.
fn delta_into_view(
    focused: Rectangle,
    viewport_height: f32,
    content_top: f32,
    translation_y: f32,
) -> f32 {
    let top = content_top + translation_y;
    let bottom = top + viewport_height;
    let wanted_top = focused.y - MARGIN;
    let wanted_bottom = focused.y + focused.height + MARGIN;

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
        let d = delta_into_view(tall, 400.0, 0.0, 0.0);
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
            delta_into_view(control, 400.0, 120.0, 0.0),
            delta_into_view(
                Rectangle {
                    y: 880.0,
                    ..control
                },
                400.0,
                0.0,
                0.0
            )
        );
    }
}
