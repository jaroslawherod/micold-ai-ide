//! `material` animation wrappers (Constitution Principle VIII) mimicking Angular Material
//! motion.
//!
//! iced exposes no element opacity (still true as of 0.14 — `widget::opaque` gates events, it
//! does not blend), so [`Fade`] approximates a fade by compositing a
//! scrim of the surrounding surface color over its child via `fill_quad` (alpha = `1 -
//! progress`). [`Expand`] performs a real vertical reveal using the renderer's
//! transformation/clip: it animates its own laid-out height and reveals the child top-down,
//! clipped to the visible area. Both are passthrough widgets — layout, events, and the
//! overlay are delegated to the child.
//!
//! # Self-animating (feature 017, FR-011/FR-014)
//!
//! Each wrapper owns the scalar it animates, as widget-tree state. A caller says *where it wants
//! the element to be* — shown or hidden, and over how long — never how far along the transition
//! is. The progress it used to have to compute, thread down through the view, and clean up
//! afterwards is now [`Progress`], held where the renderer already keeps per-instance state.
//!
//! Two consequences fall out rather than being enforced: two instances cannot interfere, because
//! neither can see the other; and a removed wrapper drops its track with it, so nothing animates
//! on behalf of an element that is gone.

use std::time::Duration;

use crate::ui::cdk::motion::Progress;
use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::{Color, Element, Event, Length, Rectangle, Size, Transformation, Vector};

/// At or below this a transition counts as finished, and the wrapper is fully hidden.
///
/// Matched to the threshold the central animator's consumers used (`<= 0.001`) so "when does a
/// closing element stop existing" lands on the same frame it always did.
const HIDDEN: f32 = 0.001;

/// Report the cursor as [`mouse::Cursor::Unavailable`] whenever it sits outside `bounds`.
/// Used by widgets (like [`Expand`]) whose child keeps its full, untranslated layout even
/// while visually clipped/collapsed — without this, a child's own hit-test would still see
/// the real cursor position and could respond to clicks landing outside the widget's actual
/// (animated) visible area.
fn clip_cursor_to(cursor: mouse::Cursor, bounds: Rectangle) -> mouse::Cursor {
    if cursor.is_over(bounds) {
        cursor
    } else {
        mouse::Cursor::Unavailable
    }
}

/// Whether an event still reaches the child of a fully hidden wrapper.
///
/// A hidden wrapper is inert: its child is invisible, so letting a press through to a button
/// nobody can see would be a phantom hit. Window events are the exception, and a load-bearing one
/// — the frame tick arrives that way, and a nested wrapper that stopped receiving it would freeze
/// at zero and never animate back in.
fn reaches_hidden_child(event: &Event) -> bool {
    matches!(event, Event::Window(_))
}

/// One marker per wrapper, used only as its widget-tree tag.
///
/// The tag is how the renderer decides whether the state at a tree position still belongs to the
/// widget now standing there. Every wrapper keeps the same [`Track`], so a single shared tag would
/// let the renderer hand one wrapper's progress to a different wrapper that happened to take its
/// place — a dialog inheriting a closed menu's `1.0` and skipping its entrance. Distinct tags make
/// that structurally impossible; the state they guard is identical either way.
mod tags {
    pub struct Fade;
    pub struct Expand;
    pub struct Scale;
    pub struct Scrim;
}

/// The per-instance animation state a wrapper keeps in the widget tree.
struct Track {
    progress: Progress,
    /// The identity of what is being animated — see [`Motion::restart_on`].
    key: u64,
    /// Whether `on_hidden` has already been emitted for the current stay at zero, so a finished
    /// transition announces itself once rather than on every idle frame.
    announced: bool,
}

/// Where a wrapper is heading and how long it should take to get there.
///
/// This is the whole of what a caller supplies. Note what is absent: any notion of how far along
/// the transition currently is. That is the wrapper's own business, and lives in [`Track`].
struct Motion<M> {
    shown: bool,
    over: Duration,
    /// The outbound duration, when it differs from the inbound one. Material exits are quicker
    /// than entrances: leaving is acknowledgement, arriving is presentation.
    out: Option<Duration>,
    key: u64,
    enter: bool,
    on_hidden: Option<M>,
}

impl<M: Clone> Motion<M> {
    /// Heading toward `shown`, taking `over` to travel the full range.
    fn new(shown: bool, over: Duration) -> Self {
        Self {
            shown,
            over,
            out: None,
            key: 0,
            enter: false,
            on_hidden: None,
        }
    }

    /// How long the current direction of travel should take.
    fn duration(&self) -> Duration {
        match self.out {
            Some(out) if !self.shown => out,
            _ => self.over,
        }
    }

    /// Where the track is going.
    fn target(&self) -> f32 {
        if self.shown {
            1.0
        } else {
            0.0
        }
    }

    /// Where a freshly mounted track starts.
    ///
    /// Settled at its target by default, so an element built already-open does not animate into
    /// existence. [`Motion::enter`] opts into the opposite for an element whose *appearing* is
    /// the transition — a dialog is mounted precisely because it is opening.
    fn initial(&self) -> f32 {
        if self.enter {
            0.0
        } else {
            self.target()
        }
    }

    fn state(&self) -> tree::State {
        tree::State::new(Track {
            progress: Progress::new(self.initial()),
            key: self.key,
            announced: false,
        })
    }

    /// Restart the transition when the caller's identity changed under it.
    ///
    /// A wrapper whose key differs from the one its state was built with is animating something
    /// *else* now: the main view showing a different session is not the old view still arriving,
    /// so it begins its entrance rather than finishing a transition that belonged to the content
    /// it replaced. Always from zero — a restart is the entrance played again, which is what
    /// distinguishes it from [`Motion::initial`]'s question of how a track first appears.
    fn resync(&self, tree: &mut Tree) {
        let track = tree.state.downcast_mut::<Track>();
        if track.key != self.key {
            track.key = self.key;
            track.progress.restart_at(0.0);
            track.announced = false;
        }
    }

    /// The value to render with, without advancing anything — for `layout`/`draw`.
    fn value(tree: &Tree) -> f32 {
        tree.state.downcast_ref::<Track>().progress.value()
    }

    /// Advance the track on a frame tick and report what to render with.
    ///
    /// Also announces arrival at hidden, once, for a caller that must know when a closing element
    /// has finished closing (the only thing about a transition an application can still need).
    fn advance(&self, tree: &mut Tree, event: &Event, shell: &mut Shell<'_, M>) -> f32 {
        let target = self.target();
        let track = tree.state.downcast_mut::<Track>();
        let value = track
            .progress
            .on_frame(event, target, self.duration(), shell);

        if value > HIDDEN {
            track.announced = false;
        } else if target == 0.0 && !track.announced {
            track.announced = true;
            if let Some(message) = &self.on_hidden {
                shell.publish(message.clone());
            }
        }
        value
    }
}

/// The tree-state hooks a self-animating wrapper needs: where its track is kept, and the check
/// that restarts it when the caller's identity changed.
macro_rules! self_animating {
    ($tag:ty) => {
        fn tag(&self) -> tree::Tag {
            tree::Tag::of::<$tag>()
        }

        fn state(&self) -> tree::State {
            self.motion.state()
        }

        fn diff(&self, tree: &mut Tree) {
            self.motion.resync(tree);
            tree.diff_children(std::slice::from_ref(&self.content));
        }
    };
}

/// The builder steps every animated wrapper shares, so they read alike (Principle VIII).
macro_rules! motion_builder {
    () => {
        /// Start hidden and transition in, rather than appearing already settled.
        ///
        /// For an element whose mounting *is* its opening — a dialog, a menu panel.
        pub fn animate_in(mut self) -> Self {
            self.motion.enter = true;
            self
        }

        /// Identify what is being animated, so a change of identity restarts the transition
        /// instead of continuing the outgoing element's.
        pub fn restart_on(mut self, key: u64) -> Self {
            self.motion.key = key;
            self
        }

        /// Take a different, usually shorter, time on the way out.
        pub fn exiting_over(mut self, out: Duration) -> Self {
            self.motion.out = Some(out);
            self
        }

        /// Emit `message` once the element has finished hiding.
        ///
        /// The one thing a caller can still legitimately need to know — a snapshot being animated
        /// out has to be released by whoever is holding it.
        pub fn on_hidden(mut self, message: M) -> Self {
            self.motion.on_hidden = Some(message);
            self
        }
    };
}

/// Shared `children`/`diff`/`operate` for a single-child widget whose `layout()` wraps its
/// child in `layout::Node::with_children(outer_size, vec![child])` (as [`Expand`] does)
/// rather than returning the child's node directly (as [`Fade`]/[`Scale`] do, so they don't
/// need this — they forward to the child using their own `layout` as-is).
/// `update`/`mouse_interaction`/`draw`/`overlay` stay hand-written per widget: `Expand` clips
/// the cursor to its own bounds first, because its top-anchored reveal never translates the
/// child out of the interactive area the way a horizontal slide would.
macro_rules! wrapped_child_widget {
    () => {
        fn children(&self) -> Vec<Tree> {
            vec![Tree::new(&self.content)]
        }
    };
}

/// `operate` for a wrapper whose `layout()` nests the child in a node of its own ([`Expand`]).
/// [`Fade`] and [`Scale`] return the child's node as-is and so address it directly.
macro_rules! operate_nested_child {
    () => {
        fn operate(
            &mut self,
            tree: &mut Tree,
            layout: Layout<'_>,
            renderer: &Renderer,
            operation: &mut dyn Operation,
        ) {
            if let Some(child) = layout.children().next() {
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    child,
                    renderer,
                    operation,
                );
            }
        }
    };
}

/// `operate` for a wrapper that passes its own layout straight through to the child.
macro_rules! operate_direct_child {
    () => {
        fn operate(
            &mut self,
            tree: &mut Tree,
            layout: Layout<'_>,
            renderer: &Renderer,
            operation: &mut dyn Operation,
        ) {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        }
    };
}

// ---------------------------------------------------------------------------------------
// Fade
// ---------------------------------------------------------------------------------------

/// Fade `content` in or out over `over`, compositing `backdrop` over it as it goes (iced still has
/// no true opacity). The backdrop should match the surface the content sits on.
///
/// Builder form (Principle VIII): `fade(content, shown, over, roles.surface).animate_in().into()`.
///
/// Takes a colour *role* rather than a resolved colour: a public component API must not name a
/// rendering-stack type (FR-013), and a caller that had to resolve one would need the styling
/// layer to do it — which is exactly the reach this feature closes.
pub fn fade<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    shown: bool,
    over: Duration,
    backdrop: micold_core::tokens::Rgb,
) -> Fade<'a, M> {
    Fade {
        content: content.into(),
        motion: Motion::new(shown, over),
        backdrop: super::style::color(backdrop),
    }
}

/// A fading element. Build it with [`fade`].
pub struct Fade<'a, M, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, M, Theme, Renderer>,
    motion: Motion<M>,
    backdrop: Color,
}

impl<M: Clone> Fade<'_, M> {
    motion_builder!();
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Fade<'_, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    self_animating!(tags::Fade);
    wrapped_child_widget!();
    operate_direct_child!();

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let progress = self.motion.advance(tree, event, shell);
        if progress <= HIDDEN && !reaches_hidden_child(event) {
            return;
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if Motion::<Message>::value(tree) <= HIDDEN {
            return mouse::Interaction::None;
        }
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
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
        let progress = Motion::<Message>::value(tree);
        if progress <= HIDDEN {
            return;
        }
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        let alpha = 1.0 - progress;
        if alpha > HIDDEN {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    ..Default::default()
                },
                iced::Background::Color(Color {
                    a: alpha,
                    ..self.backdrop
                }),
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Fade<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(fade: Fade<'a, Message, Theme, Renderer>) -> Self {
        Element::new(fade)
    }
}

// ---------------------------------------------------------------------------------------
// ViewFade (the main content area's entrance)
// ---------------------------------------------------------------------------------------

/// How long the main content area takes to arrive after what it shows changes.
const VIEW_FADE: Duration = Duration::from_millis(90);

/// The main content area, fading in whenever it starts showing something else.
///
/// Material's "fade through" for a content swap, minus the outgoing half: the previous view is
/// already gone by the time this is built, because what replaced it is a different element tree
/// entirely. Builder form (Principle VIII): `ViewFade::new(content, roles.background).showing(k)`.
pub struct ViewFade<'a, M> {
    content: Element<'a, M>,
    backdrop: micold_core::tokens::Rgb,
    key: u64,
}

impl<'a, M: Clone + 'a> ViewFade<'a, M> {
    /// The content area, over a `backdrop` matching the window behind it.
    pub fn new(content: impl Into<Element<'a, M>>, backdrop: micold_core::tokens::Rgb) -> Self {
        Self {
            content: content.into(),
            backdrop,
            key: 0,
        }
    }

    /// Which content this is. A change of identity replays the entrance; the same identity
    /// re-rendered does not, so ordinary updates do not flicker.
    pub fn showing(mut self, key: u64) -> Self {
        self.key = key;
        self
    }
}

impl<'a, M: Clone + 'a> From<ViewFade<'a, M>> for Element<'a, M> {
    fn from(v: ViewFade<'a, M>) -> Self {
        fade(v.content, true, VIEW_FADE, v.backdrop)
            .restart_on(v.key)
            .into()
    }
}

// ---------------------------------------------------------------------------------------
// HoverReveal (controls that appear when their row is pointed at)
// ---------------------------------------------------------------------------------------

/// How long a row's actions take to appear or disappear. Quick, so they feel like a response to
/// the pointer rather than an animation being played at the user.
const HOVER_REVEAL: Duration = Duration::from_millis(120);

/// Controls that fade in while their row is pointed at, and out again when it is not.
///
/// Occupies its space whether or not it is showing, so a row never reflows under the pointer —
/// that is why it fades rather than appearing. Builder form (Principle VIII):
/// `HoverReveal::new(actions, roles.surface).shown(hovered).into()`.
///
/// Each instance owns its own track (FR-011). Before feature 017 these shared one central animator
/// keyed by a *hash of the row's name*, so two worktrees whose names collided would have faded as
/// one; there is now no identity to collide, because there is no registry to hold it.
pub struct HoverReveal<'a, M> {
    content: Element<'a, M>,
    backdrop: micold_core::tokens::Rgb,
    shown: bool,
}

impl<'a, M: Clone + 'a> HoverReveal<'a, M> {
    /// Hidden controls, over a `backdrop` matching the surface the row sits on.
    pub fn new(content: impl Into<Element<'a, M>>, backdrop: micold_core::tokens::Rgb) -> Self {
        Self {
            content: content.into(),
            backdrop,
            shown: false,
        }
    }

    /// Whether the row is currently pointed at.
    pub fn shown(mut self, shown: bool) -> Self {
        self.shown = shown;
        self
    }
}

impl<'a, M: Clone + 'a> From<HoverReveal<'a, M>> for Element<'a, M> {
    fn from(h: HoverReveal<'a, M>) -> Self {
        fade(h.content, h.shown, HOVER_REVEAL, h.backdrop).into()
    }
}

// ---------------------------------------------------------------------------------------
// Scrim (the dimming behind a modal surface)
// ---------------------------------------------------------------------------------------

/// Dim the window behind a modal surface, fading to `color`'s alpha over `over`.
///
/// A leaf: it has no child, because what it dims is not inside it — it is the layer between the
/// dialog and the window, and [`cdk::overlay`](crate::ui::cdk::overlay) is what puts it there.
///
/// Builder form (Principle VIII): `scrim(color, shown, over).animate_in().on_hidden(msg).into()`.
pub fn scrim<M: Clone>(color: Color, shown: bool, over: Duration) -> Scrim<M> {
    Scrim {
        color,
        motion: Motion::new(shown, over),
    }
}

/// A dimming layer. Build it with [`scrim`].
pub struct Scrim<M> {
    color: Color,
    motion: Motion<M>,
}

impl<M: Clone> Scrim<M> {
    motion_builder!();
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Scrim<Message>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<tags::Scrim>()
    }

    fn state(&self) -> tree::State {
        self.motion.state()
    }

    /// No children to diff — a scrim is a leaf — but its track still has to notice when it is
    /// dimming behind a different dialog than the one it was built for.
    fn diff(&self, tree: &mut Tree) {
        self.motion.resync(tree);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        self.motion.advance(tree, event, shell);
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let progress = Motion::<Message>::value(tree);
        if progress <= HIDDEN {
            return;
        }
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                ..Default::default()
            },
            iced::Background::Color(Color {
                a: self.color.a * progress,
                ..self.color
            }),
        );
    }
}

impl<'a, Message, Theme, Renderer> From<Scrim<Message>> for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(scrim: Scrim<Message>) -> Self {
        Element::new(scrim)
    }
}

// ---------------------------------------------------------------------------------------
// Expand (vertical accordion reveal, top-anchored)
// ---------------------------------------------------------------------------------------

/// Reveal `content` vertically over `over`: fully expanded when `shown`, zero height when not.
/// Unlike the drawer's horizontal reveal (which anchors to the trailing edge, suited to a panel
/// sliding out from behind a fixed handle), `expand` anchors to the *top* — the child is never
/// translated, so it reveals top-down, growing the space below it (feature 009's filter accordion).
///
/// Builder form (Principle VIII): `expand(content, shown, over).into()`.
pub fn expand<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    shown: bool,
    over: Duration,
) -> Expand<'a, M> {
    Expand {
        content: content.into(),
        motion: Motion::new(shown, over),
    }
}

/// A vertically revealing element. Build it with [`expand`].
pub struct Expand<'a, M, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, M, Theme, Renderer>,
    motion: Motion<M>,
}

impl<M: Clone> Expand<'_, M> {
    motion_builder!();
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Expand<'_, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    self_animating!(tags::Expand);
    wrapped_child_widget!();
    operate_nested_child!();

    fn size(&self) -> Size<Length> {
        let inner = self.content.as_widget().size();
        Size::new(inner.width, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let progress = Motion::<Message>::value(tree);
        let child = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let full = child.size();
        let height = (full.height * progress).max(0.0);
        // No translation: the child's top edge stays put, so growing height reveals it
        // top-down (an accordion opening below its trigger), unlike the drawer's edge anchor.
        layout::Node::with_children(Size::new(full.width, height), vec![child])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let progress = self.motion.advance(tree, event, shell);
        if progress <= HIDDEN && !reaches_hidden_child(event) {
            return;
        }
        if let Some(child) = layout.children().next() {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                child,
                clip_cursor_to(cursor, layout.bounds()),
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if Motion::<Message>::value(tree) <= HIDDEN {
            return mouse::Interaction::None;
        }
        match layout.children().next() {
            Some(child) => self.content.as_widget().mouse_interaction(
                &tree.children[0],
                child,
                clip_cursor_to(cursor, layout.bounds()),
                viewport,
                renderer,
            ),
            None => mouse::Interaction::None,
        }
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
        if Motion::<Message>::value(tree) <= HIDDEN {
            return;
        }
        let Some(child) = layout.children().next() else {
            return;
        };
        // Clip to the (animated) visible height so the growing child never overflows.
        renderer.with_layer(layout.bounds(), |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                child,
                cursor,
                viewport,
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let child = layout.children().next()?;
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            child,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Expand<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(expand: Expand<'a, Message, Theme, Renderer>) -> Self {
        Element::new(expand)
    }
}

// ---------------------------------------------------------------------------------------
// Scale (scale about center)
// ---------------------------------------------------------------------------------------

/// The scale applied at `progress` 0.0 — a subtle Material dialog "lift" (grows to full size
/// as it enters, shrinks slightly as it leaves). Kept close to 1.0 so it reads as a lift, not
/// a zoom.
const MIN_SCALE: f32 = 0.96;

/// Lift `content` over `over`: full size when `shown`, [`MIN_SCALE`] about its centre when not.
/// A passthrough widget — layout, events, and the overlay are delegated to the child; only drawing
/// is transformed (via the renderer), so it never reflows the layout around it.
///
/// Builder form (Principle VIII): `scale(content, shown, over).animate_in().into()`.
pub fn scale<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    shown: bool,
    over: Duration,
) -> Scale<'a, M> {
    Scale {
        content: content.into(),
        motion: Motion::new(shown, over),
    }
}

/// A lifting element. Build it with [`scale`].
pub struct Scale<'a, M, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, M, Theme, Renderer>,
    motion: Motion<M>,
}

impl<M: Clone> Scale<'_, M> {
    motion_builder!();
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Scale<'_, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    self_animating!(tags::Scale);
    wrapped_child_widget!();
    operate_direct_child!();

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let progress = self.motion.advance(tree, event, shell);
        if progress <= HIDDEN && !reaches_hidden_child(event) {
            return;
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if Motion::<Message>::value(tree) <= HIDDEN {
            return mouse::Interaction::None;
        }
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
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
        let progress = Motion::<Message>::value(tree);
        if progress <= HIDDEN {
            return;
        }
        let scaling = MIN_SCALE + (1.0 - MIN_SCALE) * progress;
        // At full size, skip the transform layer entirely (identity — nothing to do).
        if (scaling - 1.0).abs() < 0.0001 {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
            return;
        }
        // Scale about the child's center: translate(c) · scale · translate(-c).
        let center = layout.bounds().center();
        let transformation = Transformation::translate(center.x, center.y)
            * Transformation::scale(scaling)
            * Transformation::translate(-center.x, -center.y);
        renderer.with_transformation(transformation, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Scale<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(scale: Scale<'a, Message, Theme, Renderer>) -> Self {
        Element::new(scale)
    }
}
