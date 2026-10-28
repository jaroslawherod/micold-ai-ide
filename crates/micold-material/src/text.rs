//! `Text` — the library's wrapper around the rendering stack's text widget (Principle VIII).
//!
//! Call sites used to pick a *number*: 135 of them named a raw size from the type scale, and each
//! one was free to pick a different number for the same kind of text. A label that should have been
//! `LABEL` was `BODY` in four places, and nothing could tell you which was intended.
//!
//! A call site now names a **role** — what this text *is* — and the role decides the size. That is
//! what makes feature 018's type scale a single edit: roles gain weight and line height in one
//! place, and every call site follows without being touched.
//!
//! Parity: each role resolves to exactly the size that role's call sites use today (FR-005).

use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::Range;

use crate::roboto::ROBOTO;
use crate::selectable_text::{highlight, split_at, Hit, Measure, Selectable, Shape};
use crate::style;
use iced::widget::text::{LineHeight, Wrapping};
use iced::widget::{rich_text, span, text};
use iced::{Element, Font, Length};
use micold_core::tokens::{typography, Rgb, Roles};

/// What a piece of text *is*, rather than how big it is.
///
/// The sidebar roles are separate rather than a modifier because the sidebar runs at its own
/// density — an explicit, auditable 80% of the body/label roles — and that decision belongs in one
/// place rather than at every sidebar call site.
///
/// # Choosing between [`Body`](Self::Body), [`Caption`](Self::Caption) and [`Label`](Self::Label)
///
/// Material splits these on *what the text does*, not on how big it is, and the split is the whole
/// reason `Caption` and `Label` are both here at 12dp:
///
/// - **prose** — anything that reads as a sentence, whatever its size — is a `body` role, set at
///   weight 400. Sentences at weight 500 read as shouting, and this application had a lot of them:
///   error lines, diagnostics, "This repository has no other branches."
/// - **an interface label** — a terse word or fragment naming a thing, never a sentence — is a
///   `label` role, at weight 500. "git", "Type", "Branch:".
///
/// So `Caption` and `Label` are the same size and differ only in weight, deliberately. Picking
/// between them is picking whether the text is something you *read* or something you *scan*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeRole {
    /// The largest role. Used once, for the empty-state headline.
    Display,
    /// A dialog or screen title.
    Headline,
    /// A screen-level title — the largest thing inside a surface rather than above it.
    Title,
    /// A heading introducing a group *within* a surface: a sidebar's header, a list's subheading.
    ///
    /// Distinct from [`Title`](Self::Title) because 22dp at weight 400 is a screen title, and a
    /// group heading nested inside a panel that already has one has to sit below it in the
    /// hierarchy — Material's `title_medium` is 16dp at weight 500, which reads as a heading
    /// through weight rather than through size.
    Section,
    /// Ordinary running text — the default for anything that is not one of the others.
    Body,
    /// Supporting prose: hints, error lines, diagnostics, explanatory sentences.
    ///
    /// The same size as [`Label`](Self::Label) and deliberately a *lighter* weight, because this is
    /// text to be read rather than scanned. See the type-level docs.
    Caption,
    /// Text on something you can press: a button, a menu item, a chip.
    ///
    /// Material's `label_large`, and one of the most recognisable things about the type system —
    /// button text is medium weight. Rendering it at the body weight is the same size and reads as
    /// a different design language, which is exactly the gap feature 018 exists to close.
    Action,
    /// A terse interface label naming a thing: a field's name, a badge, a count.
    Label,
    /// A worktree or session name in the sidebar.
    SidebarName,
    /// A tag chip's text in the sidebar.
    SidebarTag,
    /// A session line in the sidebar.
    SidebarSession,
    /// The sidebar's *current* session — the one the main area is showing (feature 024, FR-003a).
    ///
    /// The same size and line height as [`SidebarSession`](Self::SidebarSession), differing only
    /// in weight, so the row that is current reads as heavier without reflowing the list. That is
    /// the point: the selected-row tint says "current" in colour, and this says it again in a
    /// channel that survives greyscale and a colour-vision deficit.
    ///
    /// Weight is the only non-colour channel available without inventing a token — 400 and 500 are
    /// the only weights the scale specifies — and the row's other slots are spoken for: its leading
    /// slot holds the activity dot and only that (BUG-005), and its name is already tinted by
    /// lifecycle.
    SidebarSessionCurrent,
    /// A sidebar state group's header (feature 1125, R8): 12 px at the scale's heavier weight,
    /// `label_medium`, so a header reads as a heading over the 12 px rows it names.
    SidebarGroup,
    /// An Overview card's title (feature 1120, FR-004): 17/22 at weight 600, a card-scoped role
    /// declared off the Material scale in `typography::SCOPED` (R10, BUG-1298 FR-022).
    CardTitle,
    /// An Overview card's 13 px semibold words: the state word, the issue link, the status words
    /// that say something is wrong (BUG-1298).
    CardStrong,
    /// An Overview card's type word in its header: 13 px at weight 500 (BUG-1298 FR-022).
    CardType,
    /// An Overview card's 13 px small text: branch, numbers, caption, status words (BUG-1298).
    CardSmall,
    /// An Overview card's 12 px semibold labels: "NOW" and the pull request pills (BUG-1298).
    CardTag,
}

impl TypeRole {
    /// The Material 3 role this resolves to (contract §2.4, §2.5).
    ///
    /// The mapping is deliberately a *narrowing*: the scale has fifteen roles and this enum names
    /// the twelve the application actually distinguishes, plus `CardTitle`, which is off the scale
    /// by declaration (`typography::SCOPED`). The others exist in the scale and are reachable, but
    /// no call site is required to use one — Material's true display sizes (36–57) are larger than
    /// anything this app renders.
    ///
    /// This table is the whole of the app's typographic vocabulary, so it is pinned by
    /// `tests/type_role_mapping.rs` rather than left to be re-derived by reading call sites.
    pub fn resolved(self) -> typography::TypeRole {
        match self {
            // Feature 003's `DISPLAY` (32) was a headline in Material's vocabulary, not a display.
            TypeRole::Display => typography::HEADLINE_LARGE,
            TypeRole::Headline => typography::HEADLINE_SMALL,
            TypeRole::Title => typography::TITLE_LARGE,
            TypeRole::Section => typography::TITLE_MEDIUM,
            TypeRole::Body => typography::BODY_MEDIUM,
            TypeRole::Caption => typography::BODY_SMALL,
            TypeRole::Action => typography::LABEL_LARGE,
            TypeRole::Label => typography::LABEL_MEDIUM,
            TypeRole::SidebarName => typography::SIDEBAR_NAME,
            TypeRole::SidebarTag => typography::SIDEBAR_TAG,
            TypeRole::SidebarSession => typography::SIDEBAR_SESSION,
            // `LABEL_MEDIUM` rather than a new `SIDEBAR_SESSION_CURRENT` alias in core: it is
            // already 12/16 at weight 500, exactly `SIDEBAR_SESSION` (`BODY_SMALL`) with the
            // weight changed. Adding an alias would widen `typography::SIDEBAR` and the core tests
            // that enumerate it, to say something the scale already says.
            TypeRole::SidebarSessionCurrent => typography::LABEL_MEDIUM,
            TypeRole::SidebarGroup => typography::LABEL_MEDIUM,
            TypeRole::CardTitle => typography::OVERVIEW_CARD_TITLE,
            TypeRole::CardStrong => typography::OVERVIEW_CARD_STRONG,
            TypeRole::CardType => typography::OVERVIEW_CARD_TYPE,
            TypeRole::CardSmall => typography::OVERVIEW_CARD_SMALL,
            TypeRole::CardTag => typography::OVERVIEW_CARD_TAG,
        }
    }

    /// Every role a call site can name, so a test or the showcase can enumerate them without
    /// restating the list (and drifting from it).
    pub const ALL: [TypeRole; 18] = [
        TypeRole::Display,
        TypeRole::Headline,
        TypeRole::Title,
        TypeRole::Section,
        TypeRole::Body,
        TypeRole::Caption,
        TypeRole::Action,
        TypeRole::Label,
        TypeRole::SidebarName,
        TypeRole::SidebarTag,
        TypeRole::SidebarSession,
        TypeRole::SidebarSessionCurrent,
        TypeRole::SidebarGroup,
        TypeRole::CardTitle,
        TypeRole::CardStrong,
        TypeRole::CardType,
        TypeRole::CardSmall,
        TypeRole::CardTag,
    ];

    /// The role's name as the enum spells it — for the showcase's per-role labels and for test
    /// failures that need to name the offender.
    pub fn name(self) -> &'static str {
        match self {
            TypeRole::Display => "Display",
            TypeRole::Headline => "Headline",
            TypeRole::Title => "Title",
            TypeRole::Section => "Section",
            TypeRole::Body => "Body",
            TypeRole::Caption => "Caption",
            TypeRole::Action => "Action",
            TypeRole::Label => "Label",
            TypeRole::SidebarName => "SidebarName",
            TypeRole::SidebarTag => "SidebarTag",
            TypeRole::SidebarSession => "SidebarSession",
            TypeRole::SidebarSessionCurrent => "SidebarSessionCurrent",
            TypeRole::SidebarGroup => "SidebarGroup",
            TypeRole::CardTitle => "CardTitle",
            TypeRole::CardStrong => "CardStrong",
            TypeRole::CardType => "CardType",
            TypeRole::CardSmall => "CardSmall",
            TypeRole::CardTag => "CardTag",
        }
    }

    /// The role's size in dp.
    pub fn size(self) -> f32 {
        self.resolved().size
    }

    /// The font this role is set in — Roboto at the role's weight.
    ///
    /// Only 400, 500 and 600 occur, and they are the three faces that ship. Anything else would silently
    /// resolve to whichever face the matcher considered closest.
    pub fn font(self) -> Font {
        Font {
            weight: match self.resolved().weight {
                w if w >= 600 => iced::font::Weight::Semibold,
                w if w >= 500 => iced::font::Weight::Medium,
                _ => iced::font::Weight::Normal,
            },
            ..ROBOTO
        }
    }

    /// The role's line height in dp, as a plain number.
    ///
    /// For the caller that needs the *number* rather than the renderer's type — sizing a panel
    /// whose height depends on how tall its lines are. Everything that renders text wants
    /// [`Self::line_height`] instead.
    pub fn line_height_dp(self) -> f32 {
        self.resolved().line_height
    }

    /// The role's line height, as an absolute dp value rather than a multiple of the size.
    ///
    /// Absolute because that is how Material states it, and because a ratio drifts: `20/14` is not
    /// a number anyone would write down, and rounding it per role reintroduces exactly the
    /// per-call-site variation the scale removes (FR-007).
    pub fn line_height(self) -> LineHeight {
        LineHeight::Absolute(self.resolved().line_height.into())
    }
}

/// How strongly the text reads against its surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Emphasis {
    /// Inherits the surrounding foreground colour.
    Inherited,
    /// The muted foreground, for supporting text.
    Muted,
    /// An explicit role colour.
    Tinted(Rgb),
    /// An explicit role colour at the disabled opacity.
    Disabled(Rgb),
}

/// A piece of text at a type role. Builder form (Principle VIII):
/// `Text::new("Cancel", TypeRole::Body, roles).muted().into()`.
pub struct Text<'a, M> {
    content: Cow<'a, str>,
    role: TypeRole,
    roles: Roles,
    emphasis: Emphasis,
    width: Option<Length>,
    height: Option<Length>,
    align_y: Option<iced::alignment::Vertical>,
    wrapping: Option<Wrapping>,
    monospace: bool,
    select: Option<Select<'a, M>>,
    marker: PhantomData<M>,
}

/// What [`Text::selectable`] was given: the selected bytes, and the wrapper that answers hits
/// (feature 1550).
struct Select<'a, M> {
    selected: Option<Range<usize>>,
    make: SelectableOf<'a, M>,
}

type SelectableOf<'a, M> = Box<dyn FnOnce(Element<'a, M>, Measure) -> Selectable<'a, M> + 'a>;

impl<'a, M: 'a> Text<'a, M> {
    /// Text reading `content` at `role`, themed by `roles`. Inherits the surrounding foreground
    /// colour; call [`Self::muted`] or [`Self::tint`] to change that.
    pub fn new(content: impl Into<Cow<'a, str>>, role: TypeRole, roles: Roles) -> Self {
        Self {
            content: content.into(),
            role,
            roles,
            emphasis: Emphasis::Inherited,
            width: None,
            height: None,
            align_y: None,
            wrapping: None,
            monospace: false,
            select: None,
            marker: PhantomData,
        }
    }

    /// Supporting text: the muted foreground rather than the full-strength one.
    pub fn muted(mut self) -> Self {
        self.emphasis = Emphasis::Muted;
        self
    }

    /// Colour the text with an explicit role — for text that carries a meaning of its own, such as
    /// an error line or a tag's own accent.
    pub fn tint(mut self, color: Rgb) -> Self {
        self.emphasis = Emphasis::Tinted(color);
        self
    }

    /// Colour the text with a role at the disabled opacity — the label of a control that sets its
    /// own colours and is disabled (feature 613, FR-017).
    pub fn disabled_tint(mut self, color: Rgb) -> Self {
        self.emphasis = Emphasis::Disabled(color);
        self
    }

    /// Lay the text out at a given width — most often `Length::Fill`, so a following element is
    /// pushed to the far edge of its row.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Lay the text out at a given height, and optionally align it within that height. Used where
    /// a line has to match the height of something beside it.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Align the text vertically within its height.
    pub fn align_y(mut self, align: iced::alignment::Vertical) -> Self {
        self.align_y = Some(align);
        self
    }

    /// How the text breaks when it does not fit the width it is given.
    ///
    /// Left unset, the rendering stack wraps on **word** boundaries, which is right for prose and
    /// wrong for the one thing this application shows most: a filesystem path. A path contains no
    /// spaces, so word wrapping cannot break it at all — it lays out at its full natural width and
    /// widens whatever contains it. [`Wrapping::WordOrGlyph`] keeps word breaks where they exist
    /// and falls back to breaking mid-token where they do not (feature 029, FR-009).
    pub fn wrapping(mut self, wrapping: Wrapping) -> Self {
        self.wrapping = Some(wrapping);
        self
    }

    /// Set the text in `Font::MONOSPACE`, the face [`DiffView`](super::DiffView) draws code in, at
    /// the role's size and line height — for tool input, tool results and raw events (feature 939,
    /// research R11).
    pub fn monospace(mut self) -> Self {
        self.monospace = true;
        self
    }

    /// Make the text selectable (feature 1550, contracts/selectable-text.md): hits are sent as
    /// `on_hit(slot, hit)`, and the bytes of `selected` are drawn on the selection's fill in its
    /// ink, wrapped rows included. With `selected = None` it draws exactly as without this call.
    pub fn selectable<K: Clone + 'static>(
        mut self,
        slot: K,
        selected: Option<Range<usize>>,
        on_hit: impl Fn(K, Hit) -> M + 'a,
    ) -> Self {
        self.select = Some(Select {
            selected,
            make: Box::new(move |child, measure| Selectable::new(child, measure, slot, on_hit)),
        });
        self
    }
}

/// `content` as one rich text, the bytes of `selected` on the selection's fill in its ink; the
/// rest in the text's own colour (`None`: inherited).
fn highlighted<'a, M: 'a>(
    content: &str,
    selected: &Range<usize>,
    colour: Option<iced::Color>,
    roles: Roles,
) -> iced::widget::text::Rich<'a, (), M> {
    let (fill, ink) = highlight(roles);
    let floor = |mut i: usize| {
        i = i.min(content.len());
        while !content.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let selected = floor(selected.start)..floor(selected.end);
    let pieces: Vec<iced::widget::text::Span<'a, (), Font>> =
        split_at([0..content.len()], Some(&selected))
            .into_iter()
            .map(|(range, on)| {
                let piece = span(content[range].to_owned());
                match (on, colour) {
                    (true, _) => piece
                        .color(style::color(ink))
                        .background(style::color(fill)),
                    (false, Some(c)) => piece.color(c),
                    (false, None) => piece,
                }
            })
            .collect();
    rich_text(pieces)
}

impl<'a, M: 'a> From<Text<'a, M>> for Element<'a, M> {
    fn from(mut t: Text<'a, M>) -> Self {
        let Some(select) = t.select.take() else {
            return plain(t);
        };
        let font = if t.monospace {
            Font::MONOSPACE
        } else {
            t.role.font()
        };
        let shape = Shape {
            size: t.role.size(),
            line_height: t.role.line_height(),
            font,
            wrapping: t.wrapping.unwrap_or_default(),
            shaping: iced::advanced::text::Shaping::default(),
        };
        let measure = Measure::plain(&t.content, shape);
        let child = match &select.selected {
            None => plain(t),
            Some(selected) => {
                let colour = match t.emphasis {
                    Emphasis::Inherited => None,
                    Emphasis::Muted => Some(style::color(t.roles.on_surface_variant)),
                    Emphasis::Tinted(c) => Some(style::color(c)),
                    Emphasis::Disabled(c) => Some(style::disabled_color(c)),
                };
                let mut widget = highlighted(&t.content, selected, colour, t.roles)
                    .size(t.role.size())
                    .font(font)
                    .line_height(t.role.line_height())
                    .wrapping(shape.wrapping);
                if let Some(width) = t.width {
                    widget = widget.width(width);
                }
                if let Some(height) = t.height {
                    widget = widget.height(height);
                }
                if let Some(align) = t.align_y {
                    widget = widget.align_y(align);
                }
                widget.into()
            }
        };
        (select.make)(child, measure).into()
    }
}

/// The text as the rendering stack's text widget, as every caller without a selection draws it.
fn plain<'a, M: 'a>(t: Text<'a, M>) -> Element<'a, M> {
    {
        let mut widget = text(t.content)
            .size(t.role.size())
            .font(if t.monospace {
                Font::MONOSPACE
            } else {
                t.role.font()
            })
            .line_height(t.role.line_height());
        match t.emphasis {
            // No style call at all, so the text inherits — which is what an unstyled call site
            // does today. Setting an explicit colour here would silently override a container's
            // foreground and would not be parity.
            Emphasis::Inherited => {}
            Emphasis::Muted => widget = widget.style(style::muted(t.roles)),
            Emphasis::Tinted(color) => widget = widget.color(style::color(color)),
            Emphasis::Disabled(color) => widget = widget.color(style::disabled_color(color)),
        }
        if let Some(width) = t.width {
            widget = widget.width(width);
        }
        if let Some(height) = t.height {
            widget = widget.height(height);
        }
        if let Some(align) = t.align_y {
            widget = widget.align_y(align);
        }
        if let Some(wrapping) = t.wrapping {
            widget = widget.wrapping(wrapping);
        }
        widget.into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{roles, rows_with, Mounted};
    use super::*;

    fn painted_font(t: Text<'static, String>) -> Font {
        let painted = Mounted::new(t).painted_text();
        painted
            .iter()
            .find(|p| p.content == "cargo test")
            .unwrap_or_else(|| panic!("the text is painted: {painted:?}"))
            .font
    }

    /// Feature 939 (contract R-5, research R11): `.monospace()` draws the role's text in
    /// `Font::MONOSPACE`, the face `DiffView` draws code in, at the role's size.
    #[test]
    fn monospace_draws_the_monospace_face_at_the_roles_size() {
        let r = roles();
        assert_eq!(
            painted_font(Text::new("cargo test", TypeRole::Body, r).monospace()),
            Font::MONOSPACE
        );
        let painted =
            Mounted::new(Text::<String>::new("cargo test", TypeRole::Body, r).monospace())
                .painted_text();
        assert_eq!(
            painted[0].size,
            TypeRole::Body.size(),
            "the role still sizes it"
        );
    }

    fn selectable(range: Option<std::ops::Range<usize>>) -> Text<'static, String> {
        Text::new(WRAPPED, TypeRole::Body, roles())
            .width(Length::Fill)
            .selectable("t", range, |_: &str, hit| hit.offset.to_string())
    }

    /// Long enough to wrap in [`ROOM`].
    const WRAPPED: &str = "the quick brown fox jumps over the lazy dog again and again";
    const ROOM: iced::Size = iced::Size::new(160.0, 200.0);

    fn drawn(t: Text<'static, String>) -> (Vec<u8>, u32) {
        Mounted::new_in(t, ROOM).drawn(iced::Color::WHITE)
    }

    /// Feature 1550 T-4 (gate): selectable with nothing selected draws pixel-equal to the text
    /// without `.selectable`.
    #[test]
    fn selectable_text_with_nothing_selected_draws_exactly_as_the_plain_text() {
        let plain = Text::new(WRAPPED, TypeRole::Body, roles()).width(Length::Fill);
        assert_eq!(drawn(selectable(None)), drawn(plain));
    }

    /// Feature 1550 T-4: a selected range is drawn on `primary_container`, on each wrapped row it
    /// covers, and nothing outside it is.
    #[test]
    fn a_selected_range_is_drawn_on_the_selection_fill_across_wrapped_rows() {
        let r = roles();
        let (none, width) = drawn(selectable(None));
        assert!(rows_with(&none, width, r.primary_container).is_empty());
        let (some, width) = drawn(selectable(Some(4..WRAPPED.len() - 4)));
        let rows = rows_with(&some, width, r.primary_container);
        let row = TypeRole::Body.line_height_dp() as u32;
        let first = rows.first().copied().unwrap_or(u32::MAX);
        let last = rows.last().copied().unwrap_or(0);
        assert!(first < row, "the fill starts on the first row: {rows:?}");
        assert!(last >= 2 * row, "and runs onto the third: {rows:?}");
        assert!(
            !rows_with(&some, width, r.on_primary_container).is_empty(),
            "the selected glyphs are drawn in on_primary_container"
        );
    }

    /// Without the step, the text keeps its role's Roboto face.
    #[test]
    fn plain_text_keeps_its_roles_face() {
        assert_eq!(
            painted_font(Text::new("cargo test", TypeRole::Body, roles())),
            TypeRole::Body.font()
        );
    }
}
