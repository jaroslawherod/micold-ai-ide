//! A column whose children keep their widget state by key, wherever they move (research R12).
//!
//! The transcript's entries are drawn inside iced `lazy`, which keeps an entry's element and its
//! shaped text in the entry's widget-tree slot. A plain column matches slots by index, and so does
//! iced's `keyed_column` whenever the length is unchanged: at the history bound one event in and
//! one out shifts every entry by a slot, and every `lazy` rebuilds and reshapes its text — over
//! 100 ms on a full transcript (SC-004). Here the slots follow the keys, so an entry leaving the
//! front or arriving at the end rebuilds nothing else.

use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell, Widget};
use iced::widget::Column;
use iced::{Element, Event, Length, Padding, Rectangle, Renderer, Size, Theme, Vector};
use std::collections::HashMap;
use std::hash::Hash;

/// A vertical list of keyed children. Builder form:
/// `KeyedColumn::new(rows).spacing(…).padding(…).width(Length::Fill).into()`.
pub struct KeyedColumn<'a, K, M> {
    keys: Vec<K>,
    column: Column<'a, M>,
}

/// The keys the tree's child slots belong to, in order.
struct Keys<K>(Vec<K>);

impl<'a, K: Copy + Eq + Hash + 'static, M: 'a> KeyedColumn<'a, K, M> {
    /// `children` in order, each under its key.
    pub fn new(children: impl IntoIterator<Item = (K, Element<'a, M>)>) -> Self {
        let (keys, children): (Vec<K>, Vec<Element<'a, M>>) = children.into_iter().unzip();
        Self {
            keys,
            column: Column::with_children(children),
        }
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.column = self.column.spacing(spacing);
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.column = self.column.padding(padding);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.column = self.column.width(width);
        self
    }
}

impl<K: Copy + Eq + Hash + 'static, M> Widget<M, Theme, Renderer> for KeyedColumn<'_, K, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Keys<K>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Keys(self.keys.clone()))
    }

    fn children(&self) -> Vec<Tree> {
        self.column.children()
    }

    /// Move each kept child's slot to where its key now is, give a new key an empty slot (which the
    /// column's own diff then builds), then diff each child against its own slot.
    fn diff(&self, tree: &mut Tree) {
        let Tree {
            state, children, ..
        } = tree;
        let Keys(held) = state.downcast_mut::<Keys<K>>();
        if *held != self.keys {
            let mut slots: HashMap<K, Tree> =
                held.iter().copied().zip(children.drain(..)).collect();
            children.extend(
                self.keys
                    .iter()
                    .map(|key| slots.remove(key).unwrap_or_else(Tree::empty)),
            );
            held.clone_from(&self.keys);
        }
        self.column.diff(tree);
    }

    fn size(&self) -> Size<Length> {
        Widget::<M, Theme, Renderer>::size(&self.column)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.column.layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.column.operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        self.column.update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.column
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.column
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        self.column
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, K: Copy + Eq + Hash + 'static, M: 'a> From<KeyedColumn<'a, K, M>> for Element<'a, M> {
    fn from(column: KeyedColumn<'a, K, M>) -> Self {
        Element::new(column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::{lazy, text};
    use std::cell::Cell;

    thread_local! {
        static BUILT: Cell<usize> = const { Cell::new(0) };
    }

    fn rows(keys: &[u32]) -> KeyedColumn<'static, u32, ()> {
        KeyedColumn::new(keys.iter().map(|&k| {
            let row: Element<'static, ()> = lazy(k, |k| {
                BUILT.with(|b| b.set(b.get() + 1));
                text(format!("row {k}"))
            })
            .into();
            (k, row)
        }))
    }

    fn built_by_diff(from: &[u32], to: &[u32]) -> usize {
        let before: Element<'static, ()> = rows(from).into();
        let mut tree = Tree::new(before.as_widget());
        BUILT.with(|b| b.set(0));
        let after: Element<'static, ()> = rows(to).into();
        tree.diff(after.as_widget());
        BUILT.with(|b| b.get())
    }

    #[test]
    fn one_in_at_the_end_and_one_out_at_the_front_builds_only_the_new_row() {
        assert_eq!(built_by_diff(&[1, 2, 3, 4], &[2, 3, 4, 5]), 1);
    }

    #[test]
    fn an_unchanged_list_builds_nothing_and_a_removal_builds_nothing() {
        assert_eq!(built_by_diff(&[1, 2, 3], &[1, 2, 3]), 0);
        assert_eq!(built_by_diff(&[1, 2, 3], &[2, 3]), 0);
    }
}
