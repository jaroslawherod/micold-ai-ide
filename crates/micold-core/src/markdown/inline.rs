//! Inline events → styled spans (research R3): emphasis, inline code and links as styles, and
//! everything else the subset does not cover as its literal text, so no received text is hidden.

use pulldown_cmark::{Event, LinkType, Tag, TagEnd};

use super::block::{Inlines, Link, Span, Style};
use crate::link::address::{classify, Address};
use crate::link::detect::detect;

/// Whether `target` may be opened: a web or mail address (FR-004).
fn activatable(target: &str) -> bool {
    matches!(classify(target), Address::Web(_) | Address::Mail(_))
}

/// `text` with every line ending as `"\n"`, so CRLF text gives the blocks LF text gives.
pub(super) fn lf(text: &str) -> String {
    if text.contains('\r') {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text.to_string()
    }
}

/// Whether `tag` is an inline (span-level) one.
pub(super) fn is_inline(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Emphasis
            | Tag::Strong
            | Tag::Strikethrough
            | Tag::Superscript
            | Tag::Subscript
            | Tag::Link { .. }
            | Tag::Image { .. }
    )
}

/// Whether `end` closes an inline tag.
pub(super) fn is_inline_end(end: &TagEnd) -> bool {
    matches!(
        end,
        TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript
            | TagEnd::Link
            | TagEnd::Image
    )
}

/// An image being read: its address and its description so far.
struct Image {
    address: String,
    description: String,
    nested: u32,
}

/// Spans built from one run of inline events.
#[derive(Default)]
pub(super) struct Inline {
    spans: Vec<Span>,
    /// Per span: an image's text, where no bare address is looked for.
    images: Vec<bool>,
    bold: u32,
    italic: u32,
    strike: u32,
    links: Vec<Link>,
    image: Option<Image>,
}

impl Inline {
    /// Takes one inline event. A task-list mark is the caller's (it belongs to the list item).
    pub(super) fn feed(&mut self, event: Event<'_>) {
        if let Some(image) = &mut self.image {
            match event {
                Event::Start(Tag::Image { .. }) => image.nested += 1,
                Event::End(TagEnd::Image) if image.nested > 0 => image.nested -= 1,
                Event::End(TagEnd::Image) => {
                    let image = self.image.take().expect("an image is being read");
                    let text = format!("{} ({})", image.description, image.address);
                    let link = self.links.last().cloned();
                    self.push(&text, false, link, true);
                }
                Event::SoftBreak | Event::HardBreak => image.description.push(' '),
                Event::Text(t) | Event::Code(t) | Event::InlineHtml(t) | Event::Html(t) => {
                    image.description.push_str(&lf(&t));
                }
                _ => {}
            }
            return;
        }
        let (text, code) = match event {
            Event::Start(Tag::Strong) => return self.bold += 1,
            Event::End(TagEnd::Strong) => return self.bold = self.bold.saturating_sub(1),
            Event::Start(Tag::Emphasis) => return self.italic += 1,
            Event::End(TagEnd::Emphasis) => return self.italic = self.italic.saturating_sub(1),
            Event::Start(Tag::Strikethrough) => return self.strike += 1,
            Event::End(TagEnd::Strikethrough) => {
                return self.strike = self.strike.saturating_sub(1);
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => {
                let target = match link_type {
                    LinkType::Email => format!("mailto:{dest_url}"),
                    _ => dest_url.to_string(),
                };
                return self.links.push(Link {
                    activatable: activatable(&target),
                    target,
                });
            }
            Event::End(TagEnd::Link) => {
                self.links.pop();
                return;
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                return self.image = Some(Image {
                    address: dest_url.to_string(),
                    description: String::new(),
                    nested: 0,
                });
            }
            Event::Text(t)
            | Event::InlineHtml(t)
            | Event::Html(t)
            | Event::InlineMath(t)
            | Event::DisplayMath(t) => (lf(&t), false),
            Event::Code(t) => (lf(&t), true),
            Event::FootnoteReference(label) => (format!("[^{label}]"), false),
            Event::SoftBreak => (" ".to_string(), false),
            Event::HardBreak => ("\n".to_string(), false),
            _ => return,
        };
        let link = self.links.last().cloned();
        self.push(&text, code, link, false);
    }

    /// Appends `text` in the current style, joined to the last span when it is in the same one.
    fn push(&mut self, text: &str, code: bool, link: Option<Link>, image: bool) {
        if text.is_empty() {
            return;
        }
        let style = Style {
            bold: self.bold > 0,
            italic: self.italic > 0,
            strike: self.strike > 0,
            code,
        };
        match self.spans.last_mut() {
            Some(last)
                if last.style == style
                    && last.link == link
                    && self.images.last() == Some(&image) =>
            {
                last.text.push_str(text);
            }
            _ => {
                self.spans.push(Span {
                    text: text.to_string(),
                    style,
                    link,
                });
                self.images.push(image);
            }
        }
    }

    /// The spans, with bare web and mail addresses in plain text split out as links (R3). An
    /// image still open (its end has not arrived) is shown as written so far.
    pub(super) fn finish(mut self) -> Inlines {
        if let Some(image) = self.image.take() {
            let text = format!("{} ({})", image.description, image.address);
            let link = self.links.last().cloned();
            self.push(&text, false, link, true);
        }
        let mut out = Vec::with_capacity(self.spans.len());
        for (span, image) in self.spans.into_iter().zip(self.images) {
            if image || span.link.is_some() || span.style.code {
                out.push(span);
                continue;
            }
            autolink(span, &mut out);
        }
        out
    }
}

/// `span` cut at each bare web or mail address it holds, the addresses as activatable links.
fn autolink(span: Span, out: &mut Vec<Span>) {
    let found = detect(&span.text);
    if found.is_empty() {
        out.push(span);
        return;
    }
    let bytes: Vec<usize> = span
        .text
        .char_indices()
        .map(|(i, _)| i)
        .chain([span.text.len()])
        .collect();
    let piece = |text: &str, link: Option<Link>| Span {
        text: text.to_string(),
        style: span.style,
        link,
    };
    let mut at = 0;
    for range in found {
        let (start, end) = (bytes[range.start], bytes[range.end]);
        let target = &span.text[start..end];
        if !activatable(target) {
            continue;
        }
        if start > at {
            out.push(piece(&span.text[at..start], None));
        }
        out.push(piece(
            target,
            Some(Link {
                target: target.to_string(),
                activatable: true,
            }),
        ));
        at = end;
    }
    if at < span.text.len() {
        out.push(piece(&span.text[at..], None));
    }
}
