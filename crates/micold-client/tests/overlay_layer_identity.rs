//! A surface keeps its place in the stack whatever the surfaces beside it are doing (BUG-008).
//!
//! `overlay_stacking.rs` pins the *order* surfaces appear in. This pins something the order cannot
//! see: **how many layers each one occupies**, and therefore what index every surface above it
//! lands on.
//!
//! A `Surface` used to contribute one layer or two depending on whether it had a backdrop — and it
//! has a backdrop only while it is dismissible, which is to say only while it is open. So opening
//! the overflow menu inserted a layer *beneath* the project switcher's panel and pushed that panel
//! from child 2 to child 3 of the stack.
//!
//! `iced::widget::stack` keeps per-child widget state **by index**. A panel that changes index
//! therefore inherits the state of whatever used to live there — and since feature 018's menu panel
//! owns its own fade, what it inherited was a transition. Closing the ⋮ menu shifted the switcher's
//! panel onto the menu panel's old index, mid-exit, so the switcher's panel appeared at the menu's
//! opacity and faded out on its own: a panel nobody opened, closing.
//!
//! The invariant is therefore about *layer count*, not about drawing: a surface occupies the same
//! number of layers whether or not it can be dismissed. Asserted through the laid-out tree, because
//! that is where an index is observable at all.

mod support;

use iced::Element;
use micold_client::ui::cdk::overlay::{Anchor, Overlay, Surface};
use micold_core::overlay::Layer;
use support::layout::{self as lay, LayoutRecord};

/// The app's message type is irrelevant here; a unit message keeps these tests about structure.
type Msg = ();

/// The base the overlay floats over: window-filling, as the real shell is. It matters that this
/// fills — `stack` takes its size from its **first** child, so a shrink-sized base would squeeze
/// every panel above it to the base's own size and the panels would be indistinguishable.
fn base<'a>() -> Element<'a, Msg> {
    iced::widget::Space::new()
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

/// A panel of a stated size, so it can be told apart from a backdrop (which fills the window) and
/// from its sibling (which is a different size).
fn panel<'a>(width: f32, height: f32) -> Element<'a, Msg> {
    iced::widget::Space::new()
        .width(iced::Length::Fixed(width))
        .height(iced::Length::Fixed(height))
        .into()
}

/// A popover carrying `panel`, dismissible or not. Dismissible is what "open" means to the overlay
/// host: it is the only reason a backdrop exists.
fn popover<'a>(width: f32, height: f32, dismissible: bool) -> Surface<'a, Msg> {
    let surface = Surface::new(Layer::Popover, panel(width, height), Anchor::Center);
    if dismissible {
        surface.on_dismiss(())
    } else {
        surface
    }
}

/// Every layer of the stack: a root child, in order.
fn layers(records: &[LayoutRecord]) -> Vec<&LayoutRecord> {
    records
        .iter()
        .filter(|r| r.layer == lay::Layer::Base && r.path.len() == 1)
        .collect()
}

/// The index of the layer holding a panel of exactly `width` × `height`.
fn index_of(records: &[LayoutRecord], width: f32, height: f32) -> usize {
    let target = records
        .iter()
        .find(|r| {
            r.layer == lay::Layer::Base
                && (r.width - width).abs() < 0.5
                && (r.height - height).abs() < 0.5
        })
        .unwrap_or_else(|| panic!("no node measuring {width} × {height} in the resolved tree"));
    target.path[0]
}

/// The headline invariant: one surface opening must not move another.
#[test]
fn a_surface_keeps_its_layer_index_when_its_neighbour_opens() {
    let renderer = lay::renderer();
    // The second surface is the one under test — the project switcher's position in `ui::view`.
    // 137 × 89 is arbitrary and unmistakable: nothing else in the tree is that size.
    let with_neighbour_closed = lay::resolve(
        Overlay::new(base())
            .push(popover(200.0, 100.0, false))
            .push(popover(137.0, 89.0, false))
            .into(),
        &renderer,
    );
    let with_neighbour_open = lay::resolve(
        Overlay::new(base())
            .push(popover(200.0, 100.0, true))
            .push(popover(137.0, 89.0, false))
            .into(),
        &renderer,
    );

    assert_eq!(
        index_of(&with_neighbour_closed, 137.0, 89.0),
        index_of(&with_neighbour_open, 137.0, 89.0),
        "the second surface's panel changed layer index because the *first* surface opened. \
         `stack` keeps per-child state by index, so a panel that moves inherits whatever state \
         lived at its new index — for a panel that owns a fade, that is a transition it never \
         started. This is BUG-008: closing the ⋮ menu played an exit on the project switcher's \
         panel, which nobody had opened.",
    );
}

/// The same property stated as the rule that produces it, so a failure says *why* rather than only
/// *that*: a surface is a fixed number of layers.
#[test]
fn a_surface_occupies_the_same_number_of_layers_open_or_closed() {
    let renderer = lay::renderer();
    let closed = lay::resolve(
        Overlay::new(base())
            .push(popover(200.0, 100.0, false))
            .into(),
        &renderer,
    );
    let open = lay::resolve(
        Overlay::new(base())
            .push(popover(200.0, 100.0, true))
            .into(),
        &renderer,
    );

    assert_eq!(
        layers(&closed).len(),
        layers(&open).len(),
        "a dismissible surface occupies more layers than an undismissible one, so every surface \
         above it moves when it opens. The backdrop must be present either way — inert when there \
         is nothing to catch — rather than appearing and disappearing beneath its neighbours.",
    );
}
