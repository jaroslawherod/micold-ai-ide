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
    let mut element = element;
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, WINDOW);

    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
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
}

impl StateUnderTest {
    pub fn new(state: micold_client::app::State) -> Self {
        Self {
            state,
            connection: micold_client::ui::ConnectionStatus::Connected,
        }
    }

    pub fn connection(mut self, connection: micold_client::ui::ConnectionStatus) -> Self {
        self.connection = connection;
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

    resolve(element, renderer)
}

/// Find the anchor covering a path, if any — used to name an element in a failure (FR-004).
pub fn anchor_for<'a>(anchors: &'a [Anchor], path: &[usize]) -> Option<&'a Anchor> {
    anchors.iter().find(|a| a.path == path)
}

/// Emit the whole fixture: one global header, then one section per covered state (contract §1).
pub fn emit_fixture(
    states: &[CoveredState],
    renderer: &iced::Renderer,
    scheme: micold_core::theme::ColorScheme,
) -> String {
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

    for covered in states {
        out.push('\n');
        out.push_str(&format!("## {}\n", covered.name));
        for anchor in covered.anchors {
            out.push_str(&format!(
                "@ {} -> {}\n",
                anchor.name,
                path_token(anchor.path)
            ));
        }
        for record in records_for(covered, renderer, scheme) {
            out.push_str(&format_record(&record));
            out.push('\n');
        }
    }

    out
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
    use iced::advanced::Widget;

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
    let containing_width = |p: iced::Point| -> f32 {
        boxes
            .iter()
            .filter(|b| {
                p.x >= b.x - 0.5
                    && p.x <= b.x + b.width + 0.5
                    && p.y >= b.y - 0.5
                    && p.y <= b.y + b.height + 0.5
            })
            .max_by_key(|b| b.path.len())
            .map(|b| b.width)
            .unwrap_or(f32::INFINITY)
    };

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
                    let allowed = containing_width(*position).min(clip_bounds.width);
                    // A tenth of a pixel is normalisation noise, not an overflow.
                    if allowed.is_finite() && paragraph.min_bounds.width > allowed + 0.1 {
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
                        });
                    }
                }
            }
        }
    }

    found
}
