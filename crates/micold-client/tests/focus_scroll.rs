//! Whether the scroll-into-view operation reaches anything on a real widget tree — FR-030's
//! second clause, "with the focused element visible".
//!
//! `focus.rs`'s own unit tests cover the arithmetic. What they cannot cover is whether the two
//! passes meet: whether the focused control's rectangle and the panel's are in the same
//! coordinate space, and whether the chain is driven far enough for the second pass to run at
//! all. All three fail silently — the operation runs, touches nothing, and the page sits exactly
//! where it did before any of this existed.
//!
//! So this builds a real scrollable taller than its panel, walks the keyboard to the last field
//! in it, and asks where the panel ended up. It lives out here rather than beside the code
//! because building the tree means naming raw iced widgets, which a module under `src/ui/` may
//! not do (SC-001, `material_boundary`).

mod support;

use iced::advanced::widget::operation::scrollable::Scrollable;
use iced::advanced::widget::operation::{Focusable, Outcome};
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{layout, Layout};
use iced::widget::{column, scrollable, text_input};
use iced::{Element, Length, Rectangle, Size, Vector};
use micold_core::tokens::spacing;
use support::layout as lay;

/// Small enough that twelve fields cannot fit, so the last one is unquestionably below the fold.
const WINDOW: Size = Size::new(400.0, 300.0);
const PANEL: f32 = 200.0;
const FIELDS: usize = 12;

/// What a panel is showing, read back out of the tree.
#[derive(Default)]
struct Showing {
    translation_y: Option<f32>,
    content_top: f32,
    height: f32,
}

impl<T> Operation<T> for Showing {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.translation_y = Some(translation.y);
        self.content_top = content_bounds.y;
        self.height = bounds.height;
    }
}

/// Where the keyboard is, read back out of the tree.
#[derive(Default)]
struct Holding {
    focused: Option<Rectangle>,
}

impl<T> Operation<T> for Holding {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        if state.is_focused() {
            self.focused = Some(bounds);
        }
    }
}

struct Panel<'a> {
    element: Element<'a, ()>,
    tree: Tree,
    node: layout::Node,
    renderer: iced::Renderer,
}

impl Panel<'_> {
    fn new() -> Self {
        let fields = (0..FIELDS).map(|_| text_input("", "").on_input(|_| ()).into());
        let mut element: Element<'_, ()> = scrollable(column(fields).spacing(spacing::SM))
            .height(Length::Fixed(PANEL))
            .into();
        let renderer = lay::renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, WINDOW),
        );
        Self {
            element,
            tree,
            node,
            renderer,
        }
    }

    /// One pass, the way a single `Widget::operate` call runs.
    fn pass(&mut self, operation: &mut dyn Operation<()>) {
        self.element.as_widget_mut().operate(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            operation,
        );
    }

    /// Run an operation the way the runtime does — to the end of its chain. Every operation here
    /// is two passes, so stopping at one would prove nothing.
    fn run(&mut self, operation: impl Operation<()> + 'static) {
        let mut current: Box<dyn Operation<()>> = Box::new(operation);
        loop {
            self.pass(current.as_mut());
            match current.finish() {
                Outcome::Chain(next) => current = next,
                Outcome::None | Outcome::Some(_) => break,
            }
        }
    }

    fn focus_next(&mut self) {
        self.run(iced::advanced::widget::operation::focusable::focus_next::<()>());
    }

    fn showing(&mut self) -> Showing {
        let mut probe = Showing::default();
        self.pass(&mut probe);
        probe
    }

    fn focused(&mut self) -> Option<Rectangle> {
        let mut probe = Holding::default();
        self.pass(&mut probe);
        probe.focused
    }
}

/// The defect, end to end: Tab to the last field of a page that scrolls and the page must have
/// followed. Before this operation existed the panel stayed at the top and the ring was painted
/// somewhere nobody could see.
#[test]
fn tabbing_to_the_last_field_brings_the_panel_to_it() {
    let mut panel = Panel::new();
    assert_eq!(panel.showing().translation_y, Some(0.0));

    for _ in 0..FIELDS {
        panel.focus_next();
    }
    let focused = panel.focused().expect("the traversal focused nothing");
    assert_eq!(
        panel.showing().translation_y,
        Some(0.0),
        "the traversal moved the panel on its own, so this test proves nothing"
    );

    panel.run(micold_client::ui::focus_into_view());

    let showing = panel.showing();
    let scrolled = showing.translation_y.unwrap();
    assert!(scrolled > 0.0, "the panel did not move");

    // A shaped text field's height is not a round number, so twelve of them stacked leave the
    // last one's bottom edge a fraction of a pixel outside the column that measured them — and
    // the panel cannot scroll past its own content. Half a pixel of slack is that accumulation,
    // not a real gap; a genuinely off-screen field misses by the hundreds.
    const SLACK: f32 = 0.5;
    let top = showing.content_top + scrolled;
    assert!(
        focused.y >= top - SLACK && focused.y + focused.height <= top + showing.height + SLACK,
        "the last field is at {:?}; the panel shows {top}..{}",
        focused,
        top + showing.height
    );
}

/// And it does not move a panel that was already showing the field. The first Tab lands on the
/// first field, which is at the top of a panel already at the top.
#[test]
fn the_first_field_leaves_the_panel_where_it_is() {
    let mut panel = Panel::new();
    panel.focus_next();
    panel.run(micold_client::ui::focus_into_view());
    assert_eq!(panel.showing().translation_y, Some(0.0));
}
