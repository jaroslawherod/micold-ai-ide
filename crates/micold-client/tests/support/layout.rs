//! Headless layout measurement for the layout snapshot parity gate (feature 019).
//!
//! Resolves the application's widget tree with no display, no GPU and no window manager, and turns
//! the result into normalised records that can be committed as a fixture and asserted byte-for-byte
//! (FR-001, FR-002, FR-012).
//!
//! Nothing here is reachable from the application. This module measures it; it never changes it
//! (FR-019).

use std::borrow::Cow;
use std::future::Future;
use std::sync::Once;
use std::task::{Context, Poll, Waker};

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::{Element, Font, Pixels, Rectangle, Size, Vector};

/// The measuring basis. See `tests/fixtures/FONT-PROVENANCE.md` for why this is pinned rather than
/// inherited from the host (research R2).
pub const REFERENCE_FONT_BYTES: &[u8] = include_bytes!("../fixtures/Roboto-Regular.ttf");

/// The family name the pinned face reports (name ID 1).
pub const REFERENCE_FONT_FAMILY: &str = "Roboto";

/// The one canonical window size every covered state resolves at (FR-008b).
pub const WINDOW: Size = Size::new(1280.0, 800.0);

/// The renderer's default text size.
pub const DEFAULT_TEXT_SIZE: f32 = 16.0;

/// The spacing between pumped redraw frames in [`resolve_revealing`].
///
/// Only its *non-zero-ness* matters. A track steps a fixed amount per redraw rather than by
/// elapsed time, and reads the event's `Instant` solely to tell one frame from a re-delivery of
/// the same one — so this sets how many steps get taken, never how large a step is.
pub const FRAME: std::time::Duration = std::time::Duration::from_millis(16);

/// Which pass produced a record (FR-009, research R5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// The base widget tree.
    Base,
    /// A widget-attached overlay, reached through `Widget::overlay`.
    Overlay,
}

impl Layer {
    /// The token written into the fixture.
    pub fn token(self) -> &'static str {
        match self {
            Layer::Base => "base",
            Layer::Overlay => "over",
        }
    }
}

/// One element's resolved geometry within one covered state.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutRecord {
    /// Depth-first child indices from the root — this element's identity (research R3).
    pub path: Vec<usize>,
    pub layer: Layer,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Poll a future that is known to be immediately ready.
///
/// The tiny-skia headless constructor does no I/O, so one poll suffices; this avoids pulling an
/// executor into the test scaffolding.
pub fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = Box::pin(f);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::hint::spin_loop();
    }
}

/// The pinned reference font, as iced names it.
pub fn reference_font() -> Font {
    Font::with_name(REFERENCE_FONT_FAMILY)
}

/// Build the headless renderer: CPU rasteriser, real text shaping, no GPU and no window (FR-001).
///
/// `Some("tiny-skia")` is load-bearing rather than cosmetic. `iced_wgpu`'s `Headless::new` returns
/// `None` on its first line when the hint is not `"wgpu"` — before it constructs a `wgpu::Instance`
/// or requests an adapter — so the fallback renderer picks the CPU rasteriser without the GPU ever
/// being probed.
pub fn renderer() -> iced::Renderer {
    static LOADED: Once = Once::new();
    LOADED.call_once(|| {
        iced::advanced::graphics::text::font_system()
            .write()
            .expect("the global font system lock was poisoned")
            .load_font(Cow::Borrowed(REFERENCE_FONT_BYTES));
    });

    block_on(<iced::Renderer as Headless>::new(
        reference_font(),
        Pixels(DEFAULT_TEXT_SIZE),
        Some("tiny-skia"),
    ))
    .expect("the tiny-skia headless renderer must construct without a GPU")
}

/// Round to one decimal place and normalise `-0.0` to `0.0` (FR-012, contract §2).
///
/// A tenth of a logical pixel is far below anything a person can see and far above floating-point
/// noise, so this absorbs churn without hiding movement. The `-0.0` case matters because it is
/// equal to `0.0` as a number but different as bytes, and the fixture is compared as bytes.
pub fn normalise(value: f32) -> f32 {
    let rounded = (value * 10.0).round() / 10.0;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}

/// Format one geometry value at fixed precision, right-aligned to a stable width (contract §2).
///
/// Explicitly *not* `{:?}`, which prints the shortest round-tripping form and therefore varies with
/// the value rather than with the format.
pub fn format_value(value: f32) -> String {
    format!("{:>8.1}", normalise(value))
}

/// The element's identity: its depth-first index path, with the root written as `0` (research R3).
pub fn path_token(path: &[usize]) -> String {
    let mut out = String::from("0");
    for index in path {
        out.push('/');
        out.push_str(&index.to_string());
    }
    out
}

/// Render one record as its fixture line (contract §1).
///
/// The path column is indented by depth so the file reads as a tree; the numeric columns are fixed
/// width so they stay aligned regardless of depth, and a depth change shifts exactly one column.
pub fn format_record(record: &LayoutRecord) -> String {
    let indented = format!(
        "{}{}",
        " ".repeat(2 * record.path.len()),
        path_token(&record.path)
    );

    format!(
        "{:<4} {:<32}{}{}{}{}",
        record.layer.token(),
        indented,
        format_value(record.x),
        format_value(record.y),
        format_value(record.width),
        format_value(record.height),
    )
}

/// Walk a laid-out node depth-first, emitting one record per element in tree order (FR-002).
///
/// The order is the tree's own and is never sorted — sorting would conceal a structural reordering,
/// which is one of the changes this gate exists to report.
pub fn walk(layout: Layout<'_>, layer: Layer) -> Vec<LayoutRecord> {
    fn descend(
        layout: Layout<'_>,
        layer: Layer,
        path: &mut Vec<usize>,
        out: &mut Vec<LayoutRecord>,
    ) {
        let bounds = layout.bounds();
        out.push(LayoutRecord {
            path: path.clone(),
            layer,
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        });

        for (index, child) in layout.children().enumerate() {
            path.push(index);
            descend(child, layer, path, out);
            path.pop();
        }
    }

    let mut out = Vec::new();
    descend(layout, layer, &mut Vec::new(), &mut out);
    out
}

/// Resolve an element at the canonical window size and record its base tree plus any
/// widget-attached overlay (FR-009).
///
/// Two passes, because this application builds overlays two ways. Dialogs and menus are composed
/// in-tree and the base walk already sees them; `material::Select` wraps `pick_list`, a genuine
/// `Widget::overlay` implementor whose dropdown is laid out separately and is invisible to the base
/// walk (research R5).
pub fn resolve<'a, M: 'a>(element: Element<'a, M>, renderer: &iced::Renderer) -> Vec<LayoutRecord> {
    resolve_pressing(element, renderer, None)
}

/// As [`resolve`], but pressing the node at `press_at` first (see [`StateUnderTest::pressing`]).
///
/// The press is dispatched, then the tree is laid out **again** before either walk. A widget that
/// opens on press changes its own size — that is the whole reason the second layout exists — and
/// recording the pre-press node while walking the post-press overlay would produce a fixture whose
/// two halves describe different moments.
pub fn resolve_pressing<'a, M: 'a>(
    element: Element<'a, M>,
    renderer: &iced::Renderer,
    press_at: Option<&[usize]>,
) -> Vec<LayoutRecord> {
    use iced::advanced::{clipboard, mouse, Shell};

    let mut element = element;
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, WINDOW);

    let mut node = element.as_widget_mut().layout(&mut tree, renderer, &limits);

    if let Some(path) = press_at {
        // Settle the entrance transition before pressing anything.
        //
        // A dialog mounts at progress 0 on purpose — `Motion::enter`, "a dialog is mounted
        // precisely because it is opening" — and `Fade::update` returns early below `HIDDEN` for
        // every event that is not a `Window` event. A press dispatched into a freshly built tree is
        // therefore swallowed before it reaches anything. That is not a defect; it is a modal
        // refusing clicks it has not finished appearing for. It cost a probe over all 128 nodes of
        // the add-worktree dialog, every one of which changed nothing, to notice.
        //
        // Redraws are pumped without re-laying out between them. `Fade` is layout-neutral, so
        // there is nothing to recompute, and eight full layouts per state per scheme would cost
        // more than the state is worth. The assumption is not load-bearing: if the settle were
        // insufficient the control would stay shut, and the covered state would produce no overlay
        // records — which `every_overlay_state_records_an_overlay` fails on.
        const SETTLE_FRAMES: u32 = 8;
        let origin = std::time::Instant::now();
        let mut settle_messages: Vec<M> = Vec::new();
        for frame in 0..SETTLE_FRAMES {
            let mut shell = Shell::new(&mut settle_messages);
            element.as_widget_mut().update(
                &mut tree,
                &iced::Event::Window(iced::window::Event::RedrawRequested(
                    origin + FRAME * frame,
                )),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                renderer,
                &mut clipboard::Null,
                &mut shell,
                &Rectangle::with_size(WINDOW),
            );
        }

        let target = walk(Layout::new(&node), Layer::Base)
            .into_iter()
            .find(|r| r.path == path)
            .unwrap_or_else(|| {
                panic!(
                    "no node at {} to press — the tree changed shape, so re-point the path \
                     against layout_snapshot.txt",
                    path_token(path),
                )
            });
        assert!(
            target.width > 0.0 && target.height > 0.0,
            "the node at {} has no area, so a press lands on nothing and the control this covered \
             state means to open will stay shut while every assertion still passes",
            path_token(path),
        );

        let mut messages: Vec<M> = Vec::new();
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(iced::Point::new(
                target.x + target.width / 2.0,
                target.y + target.height / 2.0,
            )),
            renderer,
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(WINDOW),
        );

        node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    }

    let mut records = walk(Layout::new(&node), Layer::Base);

    if let Some(mut overlay) = element.as_widget_mut().overlay(
        &mut tree,
        Layout::new(&node),
        renderer,
        &Rectangle::with_size(WINDOW),
        Vector::ZERO,
    ) {
        let overlay_node = overlay.as_overlay_mut().layout(renderer, WINDOW);
        records.extend(walk(Layout::new(&overlay_node), Layer::Overlay));
    }

    records
}

// --- Covered states, anchors and the fixture (T018, T021) -------------------------------------

/// A name bound to a path, for the elements a failure should be able to talk about (research R3).
///
/// `layout::Node` carries no name, type or id, so a path is the only identity available. Anchors
/// are advisory for *recording* — every element is recorded whether anchored or not — and
/// load-bearing for *reporting*.
#[derive(Debug, Clone)]
pub struct Anchor {
    pub name: &'static str,
    pub path: &'static [usize],
}

/// A named, reproducible configuration of the application from which a layout can be resolved.
pub struct CoveredState {
    pub name: &'static str,
    /// Constructs the application state from fixed data. Never reads the developer's workspace,
    /// configuration or session store (FR-007).
    pub build: fn() -> StateUnderTest,
    pub anchors: &'static [Anchor],
}

/// Everything `ui::view` needs, owned so the covered state can hand it over as one value.
pub struct StateUnderTest {
    pub state: micold_client::app::State,
    pub connection: micold_client::ui::ConnectionStatus,
    /// A node to press before recording. See [`StateUnderTest::pressing`].
    pub press_at: Option<&'static [usize]>,
}

impl StateUnderTest {
    pub fn new(state: micold_client::app::State) -> Self {
        Self {
            state,
            connection: micold_client::ui::ConnectionStatus::Connected,
            press_at: None,
        }
    }

    pub fn connection(mut self, connection: micold_client::ui::ConnectionStatus) -> Self {
        self.connection = connection;
        self
    }

    /// Press the node at `path` before recording, to open a dropdown that lives in widget state.
    ///
    /// `material::Select` wraps `pick_list`, whose open/closed flag is private widget-tree state
    /// with no public accessor — it cannot be set, only *caused*. A left press with the cursor
    /// inside the control's bounds is the documented way in (`pick_list.rs`, `ButtonPressed`), so
    /// the covered state drives the widget the way a person would rather than reaching into it.
    ///
    /// This is what makes the overlay pass produce anything. Dialogs and menus elsewhere in this
    /// application are composed in-tree and the base walk already sees them; a `pick_list` dropdown
    /// is laid out through `Widget::overlay` and is invisible until it is open.
    pub fn pressing(mut self, path: &'static [usize]) -> Self {
        self.press_at = Some(path);
        self
    }
}

/// Resolve one covered state in a given colour scheme.
pub fn records_for(
    covered: &CoveredState,
    renderer: &iced::Renderer,
    scheme: micold_core::theme::ColorScheme,
) -> Vec<LayoutRecord> {
    let mut under = (covered.build)();
    under.state.theme_pref = match scheme {
        micold_core::theme::ColorScheme::Light => micold_core::theme::ThemePreference::Light,
        micold_core::theme::ColorScheme::Dark => micold_core::theme::ThemePreference::Dark,
    };

    let element = micold_client::ui::view(
        &under.state,
        None,
        None,
        0,
        None,
        &micold_core::env_include::EnvIncludeOutcome::Disabled,
        &under.connection,
    );

    resolve_pressing(element, renderer, under.press_at)
}

// --- Mid-reveal resolution (BUG-001) -----------------------------------------------------------

/// A covered state pinned partway through an animation.
///
/// Separate from [`CoveredState`] on purpose. The geometry fixture deliberately does not record
/// mid-animation geometry (T030 names it as an excluded boundary), and it should not start: a
/// snapshot of a frame partway through a reveal would churn on any change to a duration or an
/// easing curve, which is motion's business rather than layout's. These states exist to be asserted
/// *about* — by the containment invariant — not to be recorded.
pub struct RevealingState {
    pub name: &'static str,
    /// The state before the transition begins, with the animated thing closed.
    pub build: fn() -> StateUnderTest,
    /// Opens it. Applied after the tree has settled closed, so the track has somewhere to travel.
    pub toward: fn(&mut micold_client::app::State),
    /// Redraw frames to pump after the change. See [`resolve_revealing`] for what a frame is worth.
    pub frames: usize,
    /// The fixture path of the node doing the revealing — the one whose height is animated.
    ///
    /// Named rather than discovered so that "is this pinned mid-reveal?" and "does it escape?" are
    /// two independent questions. Deriving the first from the second would make a *fixed* `Expand`
    /// — which escapes nothing — report as a mis-configured pin instead of as a fix.
    pub node: &'static str,
    /// What fraction of its *fully open* height the revealing node should stand at, as a check on
    /// `frames`. Progress is not observable from outside the widget, so this is the proxy.
    ///
    /// Measured against the open height rather than against the child's, because the child's height
    /// is exactly what BUG-001 gets wrong: with the defect the child stays full size, and with it
    /// fixed the child is clipped to the parent — so that ratio reads 1.0 on fixed code at every
    /// moment and cannot tell a settled reveal from a running one. The open height is the same
    /// number either way.
    pub expect_between: (f32, f32),
}

/// Resolve a state partway through its reveal.
///
/// **Why this is deterministic**, despite sounding like it depends on timing: a `Progress` advances
/// by a fixed step per redraw rather than by elapsed wall-clock time (`cdk/motion.rs` — "A track
/// advances by a fixed amount per redraw ... so a transition's real duration is only nominal").
/// `step_for(90ms)` is `16/90 ≈ 0.1778`, so frame *n* lands at exactly `0.1778n` on every machine.
/// The `Instant` inside the redraw event is never read — `Progress::on_event` matches only the
/// variant — so nothing here reads a clock.
///
/// The sequence matters. The tree is built from the *closed* element so each track mounts settled
/// at zero (`Motion::initial`); mounting it open would rest it at one, with nothing to animate.
/// Then the state is opened, the tree is diffed onto the new element — which preserves the track,
/// since the tag is unchanged — and only then do frames advance it.
pub fn resolve_revealing(
    revealing: &RevealingState,
    renderer: &iced::Renderer,
    scheme: micold_core::theme::ColorScheme,
) -> Vec<LayoutRecord> {
    use iced::advanced::clipboard;
    use iced::advanced::mouse;
    use iced::advanced::Shell;

    let mut under = (revealing.build)();
    under.state.theme_pref = match scheme {
        micold_core::theme::ColorScheme::Light => micold_core::theme::ThemePreference::Light,
        micold_core::theme::ColorScheme::Dark => micold_core::theme::ThemePreference::Dark,
    };

    let limits = layout::Limits::new(Size::ZERO, WINDOW);
    let viewport = Rectangle::with_size(WINDOW);

    // Settle closed. The element is dropped before the state is mutated — it borrows it — but the
    // tree outlives it and carries the tracks.
    let mut tree = {
        let element = view_of(&under);
        Tree::new(element.as_widget())
    };

    (revealing.toward)(&mut under.state);

    let mut element = view_of(&under);
    tree.diff(element.as_widget());

    let mut messages = Vec::new();
    let mut clipboard = clipboard::Null;

    // Each frame carries a *distinct* `Instant`, and that is load-bearing rather than cosmetic.
    // `Progress` guards against the runtime re-delivering one redraw event several times by
    // recording the last `Instant` it advanced on and ignoring a repeat (`cdk/motion.rs` —
    // `last_frame`). Pumping the same instant `frames` times therefore advances the track *once*;
    // this apparatus did exactly that until the guard landed, and read 0.178 where it expected
    // 0.356. The step size is still not derived from these values — only their inequality is.
    let origin = std::time::Instant::now();

    let mut node = element
        .as_widget_mut()
        .layout(&mut tree, renderer, &limits);

    for frame in 0..revealing.frames {
        let event = iced::Event::Window(iced::window::Event::RedrawRequested(
            origin + FRAME * frame as u32,
        ));
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &event,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        node = element
            .as_widget_mut()
            .layout(&mut tree, renderer, &limits);
    }

    walk(Layout::new(&node), Layer::Base)
}

/// Resolve the same state with the reveal already finished — the reference [`RevealingState`]'s
/// `expect_between` is measured against.
///
/// No frames are pumped: a track mounted at its target rests there (`Motion::initial`), which is
/// the same reason a dialog built already-open does not animate into existence.
pub fn resolve_revealed(
    revealing: &RevealingState,
    renderer: &iced::Renderer,
    scheme: micold_core::theme::ColorScheme,
) -> Vec<LayoutRecord> {
    let mut under = (revealing.build)();
    under.state.theme_pref = match scheme {
        micold_core::theme::ColorScheme::Light => micold_core::theme::ThemePreference::Light,
        micold_core::theme::ColorScheme::Dark => micold_core::theme::ThemePreference::Dark,
    };
    (revealing.toward)(&mut under.state);

    let mut element = view_of(&under);
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, WINDOW);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, renderer, &limits);

    walk(Layout::new(&node), Layer::Base)
}

/// `ui::view` with the arguments every covered state passes.
fn view_of(under: &StateUnderTest) -> Element<'_, micold_client::app::Message> {
    micold_client::ui::view(
        &under.state,
        None,
        None,
        0,
        None,
        &micold_core::env_include::EnvIncludeOutcome::Disabled,
        &under.connection,
    )
}

/// Find the anchor covering a path, if any — used to name an element in a failure (FR-004).
pub fn anchor_for<'a>(anchors: &'a [Anchor], path: &[usize]) -> Option<&'a Anchor> {
    anchors.iter().find(|a| a.path == path)
}

/// Resolve every covered state once per scheme, and remember it.
///
/// Six tests need these records and the naive form resolved ~71 full views to get them: three
/// emitted the whole fixture independently and the scheme check walked every state twice. Text
/// shaping dominates, so that was the entire cost of the gate — 23s against SC-006's 10s budget.
/// Resolving once per scheme brings it to 18.
///
/// Safe to cache because the records are deterministic by requirement (FR-005), which is asserted
/// independently in `layout_apparatus.rs`; if that ever stopped holding, caching would be the least
/// of the problems.
pub fn cached_records(
    states: &'static [CoveredState],
    renderer: &iced::Renderer,
    scheme: micold_core::theme::ColorScheme,
) -> &'static [Vec<LayoutRecord>] {
    use std::sync::OnceLock;
    static LIGHT: OnceLock<Vec<Vec<LayoutRecord>>> = OnceLock::new();
    static DARK: OnceLock<Vec<Vec<LayoutRecord>>> = OnceLock::new();

    let cell = match scheme {
        micold_core::theme::ColorScheme::Light => &LIGHT,
        micold_core::theme::ColorScheme::Dark => &DARK,
    };

    cell.get_or_init(|| {
        states
            .iter()
            .map(|covered| records_for(covered, renderer, scheme))
            .collect()
    })
}

/// Render the fixture text from records already resolved (contract §1).
pub fn emit_from(states: &[CoveredState], records: &[Vec<LayoutRecord>]) -> String {
    let mut out = String::new();
    out.push_str("# layout snapshot v1\n");
    out.push_str("# renderer: tiny-skia\n");
    out.push_str("# font: Roboto-Regular.ttf\n");
    out.push_str(&format!(
        "# window: {:.1}x{:.1}\n",
        WINDOW.width, WINDOW.height
    ));
    out.push_str("# scheme: light (dark asserted byte-identical, not recorded)\n");
    out.push_str(
        "# regenerate: UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client layout_snapshot\n",
    );

    for (covered, records) in states.iter().zip(records.iter()) {
        out.push('\n');
        out.push_str(&format!("## {}\n", covered.name));
        for anchor in covered.anchors {
            out.push_str(&format!(
                "@ {} -> {}\n",
                anchor.name,
                path_token(anchor.path)
            ));
        }
        for record in records {
            out.push_str(&format_record(record));
            out.push('\n');
        }
    }

    out
}

/// What [`compare_or_regenerate`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The fixture was rewritten, because regeneration was asked for explicitly.
    Regenerated,
    /// The fixture matched. Nothing was written.
    Matched,
}

/// Compare `generated` against the committed fixture, or rewrite it when explicitly asked
/// (FR-013, contract §6).
///
/// This lives here, rather than inline in the gate, so `layout_snapshot_regeneration.rs` can assert
/// the write behaviour against the *same code the gate runs* instead of against a restatement of
/// it. A test that reimplements the branch it is checking passes whatever the gate does.
///
/// Panics on a mismatch and on a missing fixture — both are the gate failing, and both must leave
/// the file exactly as they found it. A gate that rewrites its own baseline when it fails is not a
/// gate; it is a recorder that reports success.
pub fn compare_or_regenerate(
    fixture_path: &std::path::Path,
    generated: &str,
    regenerate: bool,
    describe: impl Fn(&str, &str) -> String,
) -> Outcome {
    if regenerate {
        std::fs::write(fixture_path, generated).expect("could not write the fixture");
        eprintln!("layout snapshot regenerated at {}", fixture_path.display());
        return Outcome::Regenerated;
    }

    let committed = std::fs::read_to_string(fixture_path).unwrap_or_else(|_| {
        panic!(
            "{} is missing. It is never written by a normal run — regenerate it deliberately with \
             UPDATE_LAYOUT_SNAPSHOT=1",
            fixture_path.display(),
        )
    });

    if generated != committed {
        panic!("{}", describe(&committed, generated));
    }

    Outcome::Matched
}

/// Emit the whole fixture, resolving each covered state once per scheme (contract §1).
pub fn emit_fixture(
    states: &'static [CoveredState],
    renderer: &iced::Renderer,
    scheme: micold_core::theme::ColorScheme,
) -> String {
    emit_from(states, cached_records(states, renderer, scheme))
}

// --- Text overflow (feature 019, T025 follow-up) ----------------------------------------------

/// One piece of text drawn wider than the space it was given.
#[derive(Debug, Clone)]
pub struct Overflow {
    /// What was drawn.
    pub content: String,
    /// The width the shaped paragraph actually wants.
    pub natural_width: f32,
    /// The width it was clipped to.
    pub allowed_width: f32,
    /// The layout node the text was attributed to — the deepest one containing its origin.
    /// Written as a fixture path (`0/2/1`), so it can be looked up in `layout_snapshot.txt`.
    pub node_path: String,
}

impl Overflow {
    pub fn excess(&self) -> f32 {
        self.natural_width - self.allowed_width
    }
}

/// Draw an element and report every piece of text painted wider than its clip.
///
/// **Why this exists, and why the geometry fixture does not replace it.** The defect this feature
/// was built for — an over-long sidebar label drawn over its close button — is invisible to a
/// record of layout nodes. The label is `Length::Fill`, so its node is always exactly the width its
/// parent allots, defect or not. What overflows is the *paragraph painted inside* that node, which
/// feature 017's own fix put plainly: "only the node was bounded, and nothing clips a paragraph to
/// its node."
///
/// So this asks the renderer what it actually drew. `min_bounds` is what the shaped text wants;
/// `clip_bounds` is what it was allowed. Wanting more than it was allowed is the defect.
pub fn text_overflows<'a, M: 'a>(
    element: Element<'a, M>,
    renderer: &mut iced::Renderer,
) -> Vec<Overflow> {
    use iced::advanced::Renderer as _;

    let mut element = element;
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, WINDOW);
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    let viewport = Rectangle::with_size(WINDOW);

    renderer.reset(viewport);
    element.as_widget().draw(
        &tree,
        renderer,
        &iced::Theme::Light,
        &iced::advanced::renderer::Style::default(),
        Layout::new(&node),
        iced::advanced::mouse::Cursor::Unavailable,
        &viewport,
    );

    let inner = match renderer {
        iced_renderer::fallback::Renderer::Secondary(tiny_skia) => tiny_skia,
        iced_renderer::fallback::Renderer::Primary(_) => {
            panic!("the overflow check needs the CPU rasteriser; see support::layout::renderer")
        }
    };

    // The box the text belongs to is its layout node, not whatever clip the widget happened to
    // pass. A widget that forgets to clip reports the whole viewport — precisely the defect — so
    // comparing against the recorded clip would exonerate the bug being looked for.
    //
    // "Its node" means the **deepest** node whose bounds contain the text's origin: the innermost
    // box the text starts inside. An earlier version took the *narrowest* containing node, which
    // is not the same thing and is wrong in both directions — an overlapping sibling in a stack
    // can be narrower than the text's own node and steal the attribution, inventing an overflow
    // that does not exist.
    //
    // Text that overhangs so far that its origin falls outside its own node resolves to an
    // ancestor, which is wider, so this errs toward silence rather than toward crying wolf. That
    // is the right direction for a gate whose findings are meant to be trusted.
    let boxes = walk(Layout::new(&node), Layer::Base);
    let containing_node = |p: iced::Point| -> Option<&LayoutRecord> {
        boxes
            .iter()
            .filter(|b| {
                p.x >= b.x - 0.5
                    && p.x <= b.x + b.width + 0.5
                    && p.y >= b.y - 0.5
                    && p.y <= b.y + b.height + 0.5
            })
            .max_by_key(|b| b.path.len())
    };

    // Set `LAYOUT_OVERFLOW_DEBUG=1` to report every piece of drawn text with its attribution,
    // not only the ones that overflow. The question "why did this *not* fire?" is otherwise
    // unanswerable from the outside, which is how a false positive survived once already.
    let report_everything = std::env::var("LAYOUT_OVERFLOW_DEBUG").is_ok();

    let mut found = Vec::new();
    for layer in inner.layers() {
        for item in &layer.text {
            // `Item` itself is not nameable from outside the crate; `as_slice` reaches its
            // contents without naming it.
            for text in item.as_slice() {
                if let iced::advanced::graphics::text::Text::Paragraph {
                    paragraph,
                    position,
                    clip_bounds,
                    ..
                } = text
                {
                    let node = containing_node(*position);
                    let allowed = node
                        .map(|n| n.width)
                        .unwrap_or(f32::INFINITY)
                        .min(clip_bounds.width);
                    let node_path = node
                        .map(|n| path_token(&n.path))
                        .unwrap_or_else(|| "(no containing node)".to_string());
                    // A tenth of a pixel is normalisation noise, not an overflow.
                    if report_everything
                        || (allowed.is_finite() && paragraph.min_bounds.width > allowed + 0.1)
                    {
                        // Recover what was drawn, so a failure names the string rather than
                        // leaving the reader to hunt for a widget by its width.
                        let content = paragraph
                            .upgrade()
                            .map(|p| {
                                p.buffer()
                                    .lines
                                    .iter()
                                    .map(|line| line.text())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            })
                            .unwrap_or_else(|| "(paragraph already dropped)".to_string());

                        found.push(Overflow {
                            content,
                            natural_width: paragraph.min_bounds.width,
                            allowed_width: allowed,
                            node_path,
                        });
                    }
                }
            }
        }
    }

    found
}

// --- Containment (BUG-001) ---------------------------------------------------------------------

/// A child layout node that extends outside its parent's box.
#[derive(Debug, Clone)]
pub struct Escape {
    pub parent_path: String,
    pub child_path: String,
    pub layer: Layer,
    /// Which edge it escapes past, and by how far in pixels.
    pub edge: &'static str,
    pub overhang: f32,
}

/// Report every layout node laid out beyond the bounds of the node that owns it.
///
/// **Why this is a third gate rather than a fixture line.** `layout_snapshot.txt` already records
/// every one of these boxes, so a violation is present in data the geometry gate holds — but a
/// byte-compare fixture cannot report it. A fixture records whatever it is shown as correct, so a
/// defect that predates it is regenerated into the expected value and becomes the baseline. A
/// snapshot catches *changes*; a violated invariant is a *defect*, and only an assertion about the
/// numbers can name it.
///
/// The motivating case is BUG-001: `material::Expand` reports a shrunken height to its parent while
/// its child keeps full height, relying on a draw-time clip that does not take effect. The child
/// paints over whatever moved up into the vacated space. That is invisible to the overflow gate,
/// which compares widths only, and invisible to the fixture, which would simply record it.
///
/// Layers are checked separately: a widget-attached overlay is laid out against the window, not
/// against the node it hangs off, so comparing across the two would report every overlay.
///
/// **Parked nodes are exempt, and the exemption is the invariant's own premise.** A node laid out
/// entirely off the window cannot paint over anything, because there is nothing where it is.
/// `material::NavigationDrawer` relies on this: its inactive child is translated by `-f32::MAX / 4`
/// so the tree, node list and child list stay index-aligned without it occupying space
/// (`navigation_drawer.rs:97`). Without the exemption that rail is reported in every sidebar state
/// at an overhang of 8.5e37px, which is a sentinel rather than a measurement.
///
/// This does buy silence about content pushed off-screen accidentally, which is a real defect class
/// — but a different one, and one the geometry fixture does catch as a change.
pub fn escapes(records: &[LayoutRecord], tolerance: f32) -> Vec<Escape> {
    let find = |layer: Layer, path: &[usize]| -> Option<&LayoutRecord> {
        records
            .iter()
            .find(|r| r.layer == layer && r.path == path)
    };

    let window = Rectangle::with_size(WINDOW);
    let on_window = |r: &LayoutRecord| -> bool {
        r.x < window.width && r.y < window.height && r.x + r.width > 0.0 && r.y + r.height > 0.0
    };

    let mut found = Vec::new();

    for child in records {
        let Some((_, parent_path)) = child.path.split_last() else {
            continue; // a root has nothing to escape from
        };
        let Some(parent) = find(child.layer, parent_path) else {
            continue;
        };
        if !on_window(child) {
            continue; // parked, not escaping
        }

        // Worst edge only: one node reported four times says no more than one node reported once,
        // and the widest overhang is the one worth naming.
        let edges = [
            ("left", parent.x - child.x),
            ("top", parent.y - child.y),
            (
                "right",
                (child.x + child.width) - (parent.x + parent.width),
            ),
            (
                "bottom",
                (child.y + child.height) - (parent.y + parent.height),
            ),
        ];

        if let Some((edge, overhang)) = edges
            .into_iter()
            .filter(|(_, over)| *over > tolerance)
            .max_by(|a, b| a.1.total_cmp(&b.1))
        {
            found.push(Escape {
                parent_path: path_token(parent_path),
                child_path: path_token(&child.path),
                layer: child.layer,
                edge,
                overhang,
            });
        }
    }

    found
}
