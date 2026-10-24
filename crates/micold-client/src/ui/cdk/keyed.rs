//! A row or column whose children keep their widget state by key, wherever they move (feature 1126
//! R10; promoted from the transcript's `KeyedColumn`, feature 939 research R12).
//!
//! The session strip's tabs come and go before one another; a positional row would hand a pressed
//! tab its neighbour's tree slot and drop the press (feature 023 R1). Keyed slots follow the member.
//!
//! The transcript's entries are drawn inside iced `lazy`, which keeps an entry's element and its
//! shaped text in the entry's widget-tree slot. A plain column matches slots by index, and so does
//! iced's `keyed_column` whenever the length is unchanged: at the history bound one event in and
//! one out shifts every entry by a slot, and every `lazy` rebuilds and reshapes its text — over
//! 100 ms on a full transcript (SC-004). Here the slots follow the keys, so an entry leaving the
//! front or arriving at the end rebuilds nothing else.

use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell, Widget};
use iced::widget::{Column, Row};
use iced::{Element, Event, Length, Padding, Rectangle, Renderer, Size, Theme, Vector};
use std::collections::HashMap;
use std::hash::Hash;

/// A list of keyed children along one axis. Builder form:
/// `Keyed::column(rows).spacing(…).padding(…).width(Length::Fill).into()`, or `Keyed::row(..)`.
pub struct Keyed<'a, K, M> {
    keys: Vec<K>,
    line: Line<'a, M>,
}

/// The laid-out list: iced's own row or column, which does every part of the work but the slots.
enum Line<'a, M> {
    Row(Row<'a, M>),
    Column(Column<'a, M>),
}

/// The keys the tree's child slots belong to, in order.
struct Keys<K>(Vec<K>);

impl<'a, K: Copy + Eq + Hash + 'static, M: 'a> Keyed<'a, K, M> {
    /// `children` left to right, each under its key.
    pub fn row(children: impl IntoIterator<Item = (K, Element<'a, M>)>) -> Self {
        let (keys, children) = Self::split(children);
        Self {
            keys,
            line: Line::Row(Row::with_children(children)),
        }
    }

    /// `children` top to bottom, each under its key.
    pub fn column(children: impl IntoIterator<Item = (K, Element<'a, M>)>) -> Self {
        let (keys, children) = Self::split(children);
        Self {
            keys,
            line: Line::Column(Column::with_children(children)),
        }
    }

    fn split(
        children: impl IntoIterator<Item = (K, Element<'a, M>)>,
    ) -> (Vec<K>, Vec<Element<'a, M>>) {
        children.into_iter().unzip()
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.line = match self.line {
            Line::Row(row) => Line::Row(row.spacing(spacing)),
            Line::Column(column) => Line::Column(column.spacing(spacing)),
        };
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.line = match self.line {
            Line::Row(row) => Line::Row(row.padding(padding)),
            Line::Column(column) => Line::Column(column.padding(padding)),
        };
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.line = match self.line {
            Line::Row(row) => Line::Row(row.width(width)),
            Line::Column(column) => Line::Column(column.width(width)),
        };
        self
    }
}

impl<'a, K, M> Keyed<'a, K, M> {
    fn inner(&self) -> &dyn Widget<M, Theme, Renderer> {
        match &self.line {
            Line::Row(row) => row,
            Line::Column(column) => column,
        }
    }

    fn inner_mut(&mut self) -> &mut dyn Widget<M, Theme, Renderer> {
        match &mut self.line {
            Line::Row(row) => row,
            Line::Column(column) => column,
        }
    }
}

impl<K: Copy + Eq + Hash + 'static, M> Widget<M, Theme, Renderer> for Keyed<'_, K, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Keys<K>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Keys(self.keys.clone()))
    }

    fn children(&self) -> Vec<Tree> {
        self.inner().children()
    }

    /// Move each kept child's slot to where its key now is, give a new key an empty slot (which the
    /// line's own diff then builds), then diff each child against its own slot.
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
        self.inner().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.inner().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.inner_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.inner_mut().operate(tree, layout, renderer, operation);
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
        self.inner_mut().update(
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
        self.inner()
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
        self.inner()
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
        match &mut self.line {
            Line::Row(row) => row.overlay(tree, layout, renderer, viewport, translation),
            Line::Column(column) => column.overlay(tree, layout, renderer, viewport, translation),
        }
    }
}

impl<'a, K: Copy + Eq + Hash + 'static, M: 'a> From<Keyed<'a, K, M>> for Element<'a, M> {
    fn from(keyed: Keyed<'a, K, M>) -> Self {
        Element::new(keyed)
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

    type Axis = fn(Vec<(u32, Element<'static, ()>)>) -> Keyed<'static, u32, ()>;

    /// Both axes, so each test holds for the strip's row and the transcript's column.
    const AXES: [Axis; 2] = [Keyed::row, Keyed::column];

    fn rows(axis: Axis, keys: &[u32]) -> Keyed<'static, u32, ()> {
        axis(
            keys.iter()
                .map(|&k| {
                    let row: Element<'static, ()> = lazy(k, |k| {
                        BUILT.with(|b| b.set(b.get() + 1));
                        text(format!("row {k}"))
                    })
                    .into();
                    (k, row)
                })
                .collect(),
        )
    }

    fn built_by_diff(axis: Axis, from: &[u32], to: &[u32]) -> usize {
        let before: Element<'static, ()> = rows(axis, from).into();
        let mut tree = Tree::new(before.as_widget());
        BUILT.with(|b| b.set(0));
        let after: Element<'static, ()> = rows(axis, to).into();
        tree.diff(after.as_widget());
        BUILT.with(|b| b.get())
    }

    #[test]
    fn one_in_at_the_end_and_one_out_at_the_front_builds_only_the_new_row() {
        for axis in AXES {
            assert_eq!(built_by_diff(axis, &[1, 2, 3, 4], &[2, 3, 4, 5]), 1);
        }
    }

    #[test]
    fn an_unchanged_list_builds_nothing_and_a_removal_builds_nothing() {
        for axis in AXES {
            assert_eq!(built_by_diff(axis, &[1, 2, 3], &[1, 2, 3]), 0);
            assert_eq!(built_by_diff(axis, &[1, 2, 3], &[2, 3]), 0);
        }
    }
}
