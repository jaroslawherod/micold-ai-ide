//! What `Ellipsized` and `LineClamped` share: a shaped paragraph cached against what it was shaped
//! for, and the draw that anchors it and clips it to the widget's own bounds. The cut algorithms and
//! the sizing of the two widgets differ and stay in their modules.

use iced::advanced::layout::Layout;
use iced::advanced::text::{self, Paragraph as _, Text as CoreText};
use iced::{alignment, Color, Font, Pixels, Rectangle, Size};

use super::text::TypeRole;

/// The shaped label, plus the key `K` it was shaped for.
///
/// Shaping is the expensive part and the cut shapes the text many times over, so the inputs that
/// decide the outcome are remembered: a layout with the same key reuses the paragraph.
pub(super) struct State<P, K> {
    pub(super) paragraph: P,
    /// What the paragraph holds: the text, or its cut form.
    pub(super) fitted: String,
    key: Option<K>,
}

impl<P: Default, K> Default for State<P, K> {
    fn default() -> Self {
        // No key: nothing was shaped yet, so the first layout always measures.
        Self {
            paragraph: P::default(),
            fitted: String::new(),
            key: None,
        }
    }
}

impl<P, K: PartialEq> State<P, K> {
    /// Whether the cached paragraph was shaped for `key`.
    pub(super) fn holds(&self, key: &K) -> bool {
        self.key.as_ref() == Some(key)
    }

    /// Cache `paragraph`, holding `fitted`, as shaped for `key`.
    pub(super) fn store(&mut self, paragraph: P, fitted: String, key: K) {
        self.paragraph = paragraph;
        self.fitted = fitted;
        self.key = Some(key);
    }
}

/// Draw `paragraph` anchored in `layout`, clipped to the widget's own bounds, not the viewport:
/// truncation should already have made overflow impossible, and a belt that only holds when the
/// braces do is not a belt.
pub(super) fn draw_paragraph<R>(
    renderer: &mut R,
    paragraph: &R::Paragraph,
    layout: Layout<'_>,
    viewport: &Rectangle,
    color: Color,
) where
    R: text::Renderer<Font = Font>,
{
    let bounds = layout.bounds();
    let anchor = bounds.anchor(
        paragraph.min_bounds(),
        paragraph.align_x(),
        paragraph.align_y(),
    );
    let clip = bounds.intersection(viewport).unwrap_or(bounds);
    renderer.fill_paragraph(paragraph, anchor, color, clip);
}

/// How wide `content` shapes on one line at `role`: what a bar that picks between texts by the room
/// it has measures them with (`StatusBar`'s give-way plan, the review bar's counts, feature 1270).
pub fn width_of(content: &str, role: TypeRole) -> f32 {
    <iced::Renderer as text::Renderer>::Paragraph::with_text(CoreText {
        content,
        bounds: Size::INFINITE,
        size: Pixels(role.size()),
        line_height: text::LineHeight::default(),
        font: role.font(),
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
    })
    .min_bounds()
    .width
}
