//! `forward_to_child!` — the `Widget` methods of a wrapper that has exactly one child and
//! changes nothing about it (issue 703, duplication row W1).
//!
//! A wrapper in the `cdk` or `material` layer typically differs from its child in `update`, its
//! `tag` and `state`, and sometimes one of `draw` or `diff`. Everything else it hands to its child
//! unchanged, and wrote that out by hand, about seventy lines each. This macro writes them once.
//!
//! The wrapper names the field that holds the child, its message, theme and renderer types, and
//! the methods it wants forwarded; it writes the rest itself:
//!
//! ```ignore
//! impl<'a, M: 'a> Widget<M, iced::Theme, iced::Renderer> for Wrap<'a, M> {
//!     forward_to_child!(content, M, iced::Theme, iced::Renderer;
//!         children, diff, size, layout, draw, mouse_interaction, operate, overlay);
//!
//!     fn update(&mut self, /* … */) { /* … */ }
//! }
//! ```
//!
//! The child's tree is `tree.children[0]`, the one a `children`/`diff` pair declares. A wrapper
//! with a second child, or one that returns a different layout, writes its own method.

/// Emit the named forwarding methods of a single-child `Widget` impl. See the module docs.
macro_rules! forward_to_child {
    ($field:ident, $m:ty, $t:ty, $r:ty; $($method:ident),+ $(,)?) => {
        $( $crate::ui::cdk::passthrough::forward_to_child!(@one $method, $field, $m, $t, $r); )+
    };
    (@one children, $field:ident, $m:ty, $t:ty, $r:ty) => {
        fn children(&self) -> Vec<iced::advanced::widget::Tree> {
            vec![iced::advanced::widget::Tree::new(&self.$field)]
        }
    };
    (@one diff, $field:ident, $m:ty, $t:ty, $r:ty) => {
        fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
            tree.diff_children(std::slice::from_ref(&self.$field));
        }
    };
    (@one size, $field:ident, $m:ty, $t:ty, $r:ty) => {
        fn size(&self) -> iced::Size<iced::Length> {
            self.$field.as_widget().size()
        }
    };
    (@one layout, $field:ident, $m:ty, $t:ty, $r:ty) => {
        fn layout(
            &mut self,
            tree: &mut iced::advanced::widget::Tree,
            renderer: &$r,
            limits: &iced::advanced::layout::Limits,
        ) -> iced::advanced::layout::Node {
            self.$field
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits)
        }
    };
    (@one draw, $field:ident, $m:ty, $t:ty, $r:ty) => {
        fn draw(
            &self,
            tree: &iced::advanced::widget::Tree,
            renderer: &mut $r,
            theme: &$t,
            style: &iced::advanced::renderer::Style,
            layout: iced::advanced::Layout<'_>,
            cursor: iced::advanced::mouse::Cursor,
            viewport: &iced::Rectangle,
        ) {
            self.$field.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
        }
    };
    (@one mouse_interaction, $field:ident, $m:ty, $t:ty, $r:ty) => {
        fn mouse_interaction(
            &self,
            tree: &iced::advanced::widget::Tree,
            layout: iced::advanced::Layout<'_>,
            cursor: iced::advanced::mouse::Cursor,
            viewport: &iced::Rectangle,
            renderer: &$r,
        ) -> iced::advanced::mouse::Interaction {
            self.$field.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            )
        }
    };
    (@one operate, $field:ident, $m:ty, $t:ty, $r:ty) => {
        fn operate(
            &mut self,
            tree: &mut iced::advanced::widget::Tree,
            layout: iced::advanced::Layout<'_>,
            renderer: &$r,
            operation: &mut dyn iced::advanced::widget::Operation,
        ) {
            self.$field
                .as_widget_mut()
                .operate(&mut tree.children[0], layout, renderer, operation);
        }
    };
    (@one overlay, $field:ident, $m:ty, $t:ty, $r:ty) => {
        fn overlay<'b>(
            &'b mut self,
            tree: &'b mut iced::advanced::widget::Tree,
            layout: iced::advanced::Layout<'b>,
            renderer: &$r,
            viewport: &iced::Rectangle,
            translation: iced::Vector,
        ) -> Option<iced::advanced::overlay::Element<'b, $m, $t, $r>> {
            self.$field.as_widget_mut().overlay(
                &mut tree.children[0],
                layout,
                renderer,
                viewport,
                translation,
            )
        }
    };
}

/// Emit `mouse_interaction`, `operate` and `overlay` for a wrapper that hands them to one of
/// several children, chosen per call (issue 718, duplication row W8).
///
/// `$pick(&self, tree, layout)` returns the chosen child's index and its layout, or `None` when
/// nothing is drawn; `$child(&self, i)` and `$child_mut(&mut self, i)` return that child's element.
/// The child's tree is `tree.children[i]`. A wrapper that adjusts the cursor writes its own
/// `mouse_interaction` and lists only the others.
macro_rules! forward_to_selected {
    ($pick:ident, $child:ident, $child_mut:ident, $m:ty, $t:ty, $r:ty; $($method:ident),+ $(,)?) => {
        $( $crate::ui::cdk::passthrough::forward_to_selected!(
            @one $method, $pick, $child, $child_mut, $m, $t, $r); )+
    };
    (@one mouse_interaction, $pick:ident, $child:ident, $child_mut:ident, $m:ty, $t:ty, $r:ty) => {
        fn mouse_interaction(
            &self,
            tree: &iced::advanced::widget::Tree,
            layout: iced::advanced::Layout<'_>,
            cursor: iced::advanced::mouse::Cursor,
            viewport: &iced::Rectangle,
            renderer: &$r,
        ) -> iced::advanced::mouse::Interaction {
            self.$pick(tree, layout)
                .map(|(i, child_layout)| {
                    self.$child(i).as_widget().mouse_interaction(
                        &tree.children[i],
                        child_layout,
                        cursor,
                        viewport,
                        renderer,
                    )
                })
                .unwrap_or_default()
        }
    };
    (@one operate, $pick:ident, $child:ident, $child_mut:ident, $m:ty, $t:ty, $r:ty) => {
        fn operate(
            &mut self,
            tree: &mut iced::advanced::widget::Tree,
            layout: iced::advanced::Layout<'_>,
            renderer: &$r,
            operation: &mut dyn iced::advanced::widget::Operation,
        ) {
            if let Some((i, child_layout)) = self.$pick(tree, layout) {
                self.$child_mut(i).as_widget_mut().operate(
                    &mut tree.children[i],
                    child_layout,
                    renderer,
                    operation,
                );
            }
        }
    };
    (@one overlay, $pick:ident, $child:ident, $child_mut:ident, $m:ty, $t:ty, $r:ty) => {
        fn overlay<'b>(
            &'b mut self,
            tree: &'b mut iced::advanced::widget::Tree,
            layout: iced::advanced::Layout<'b>,
            renderer: &$r,
            viewport: &iced::Rectangle,
            translation: iced::Vector,
        ) -> Option<iced::advanced::overlay::Element<'b, $m, $t, $r>> {
            let (i, child_layout) = self.$pick(tree, layout)?;
            self.$child_mut(i).as_widget_mut().overlay(
                &mut tree.children[i],
                child_layout,
                renderer,
                viewport,
                translation,
            )
        }
    };
}

pub(crate) use forward_to_child;
pub(crate) use forward_to_selected;
