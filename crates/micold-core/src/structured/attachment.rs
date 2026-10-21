//! Pictures attached to a structured session's user message (feature 941, research R8, FR-027 –
//! FR-029): which files are pictures (by their bytes, never their name), the limits the CLI and the
//! wire accept, and how a drop splits into pictures to attach and paths to insert.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The largest picture the CLI accepts: the Messages API's 5 MB limit on the base64 form.
pub const MAX_IMAGE_BYTES: usize = 3_750_000;
/// The most pictures one message carries.
pub const MAX_IMAGES_PER_MESSAGE: usize = 20;
/// The most picture bytes one message carries in all, so the `SessionUserMessage` frame with its
/// text stays under `MAX_FRAME_LENGTH`.
pub const MAX_MESSAGE_IMAGE_BYTES: usize = 12 * 1024 * 1024;

/// A picture format the CLI accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageMedia {
    Png,
    Jpeg,
    Gif,
    Webp,
}

impl ImageMedia {
    /// The format `bytes` starts with, by magic number; `None` for anything else.
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if bytes.starts_with(b"\xFF\xD8\xFF") {
            Some(Self::Jpeg)
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            Some(Self::Gif)
        } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
            Some(Self::Webp)
        } else {
            None
        }
    }

    /// The media type the CLI's image block names.
    pub fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
        }
    }
}

/// A picture of a draft or a queued message: the file's name (no directory), its format and its
/// bytes. Built by [`admit`], which checks the limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageAttachment {
    name: String,
    media: ImageMedia,
    bytes: Vec<u8>,
}

impl ImageAttachment {
    /// The file name the user dropped.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn media(&self) -> ImageMedia {
        self.media
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// What windows other than the sender see of a picture: no bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentMeta {
    pub name: String,
    pub media: ImageMedia,
    pub size: u64,
}

impl From<&ImageAttachment> for AttachmentMeta {
    fn from(image: &ImageAttachment) -> Self {
        Self {
            name: image.name.clone(),
            media: image.media,
            size: image.bytes.len() as u64,
        }
    }
}

/// Why a file was not attached (FR-029). Each names the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentRefusal {
    /// The file could not be read, or is not a picture of a known format.
    Unreadable { name: String, why: String },
    /// The picture is over [`MAX_IMAGE_BYTES`].
    TooLarge { name: String, size: u64 },
    /// The draft already carries [`MAX_IMAGES_PER_MESSAGE`] pictures.
    TooMany { name: String },
    /// The draft's pictures with this one would pass [`MAX_MESSAGE_IMAGE_BYTES`].
    MessageTooLarge { name: String },
}

impl fmt::Display for AttachmentRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { name, why } => write!(f, "{name} was not attached: {why}"),
            Self::TooLarge { name, size } => write!(
                f,
                "{name} was not attached: it is {size} bytes, and a picture may be at most \
                 {MAX_IMAGE_BYTES} bytes"
            ),
            Self::TooMany { name } => write!(
                f,
                "{name} was not attached: a message carries at most {MAX_IMAGES_PER_MESSAGE} pictures"
            ),
            Self::MessageTooLarge { name } => write!(
                f,
                "{name} was not attached: a message's pictures may be at most {} MiB in all",
                MAX_MESSAGE_IMAGE_BYTES / (1024 * 1024)
            ),
        }
    }
}

/// Attach the file `name` holding `bytes` to a draft that already holds `draft`, or say why not.
/// Checks, in order: a known picture format, [`MAX_IMAGE_BYTES`], [`MAX_IMAGES_PER_MESSAGE`],
/// [`MAX_MESSAGE_IMAGE_BYTES`].
pub fn admit(
    draft: &[ImageAttachment],
    name: &str,
    bytes: Vec<u8>,
) -> Result<ImageAttachment, AttachmentRefusal> {
    let name = name.to_owned();
    let Some(media) = ImageMedia::sniff(&bytes) else {
        return Err(AttachmentRefusal::Unreadable {
            name,
            why: "it is not a PNG, JPEG, GIF or WebP picture".into(),
        });
    };
    if bytes.len() > MAX_IMAGE_BYTES {
        let size = bytes.len() as u64;
        return Err(AttachmentRefusal::TooLarge { name, size });
    }
    if draft.len() >= MAX_IMAGES_PER_MESSAGE {
        return Err(AttachmentRefusal::TooMany { name });
    }
    let held: usize = draft.iter().map(|image| image.bytes.len()).sum();
    if held + bytes.len() > MAX_MESSAGE_IMAGE_BYTES {
        return Err(AttachmentRefusal::MessageTooLarge { name });
    }
    Ok(ImageAttachment { name, media, bytes })
}

/// Check pictures that arrived from elsewhere (the wire, a saved queue) as [`admit`] would have
/// built them one by one: the first that fails is refused.
pub fn check_images(images: &[ImageAttachment]) -> Result<(), AttachmentRefusal> {
    for (at, image) in images.iter().enumerate() {
        let admitted = admit(&images[..at], &image.name, image.bytes.clone())?;
        if admitted.media != image.media {
            return Err(AttachmentRefusal::Unreadable {
                name: image.name.clone(),
                why: "its bytes are not the picture format it names".into(),
            });
        }
    }
    Ok(())
}

/// A drop on a structured pane, split (FR-027, FR-028): the pictures to attach and the other paths
/// (folders included) to insert as text, each in drop order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DropPlan {
    pub images: Vec<PathBuf>,
    pub paths: Vec<PathBuf>,
}

/// Split `paths` by `sniff`, which reads a path's first bytes and answers its picture format
/// (`None` for a folder, an unreadable file or anything else).
pub fn classify_drop(paths: &[PathBuf], sniff: impl Fn(&Path) -> Option<ImageMedia>) -> DropPlan {
    let mut plan = DropPlan::default();
    for path in paths {
        if sniff(path).is_some() {
            plan.images.push(path.clone());
        } else {
            plan.paths.push(path.clone());
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

    fn png_of(len: usize) -> Vec<u8> {
        let mut bytes = PNG.to_vec();
        bytes.resize(len.max(PNG.len()), 0);
        bytes
    }

    fn image(name: &str, len: usize) -> ImageAttachment {
        admit(&[], name, png_of(len)).expect("a small PNG is admitted")
    }

    #[test]
    fn sniff_knows_the_four_formats_by_their_magic_bytes() {
        assert_eq!(ImageMedia::sniff(PNG), Some(ImageMedia::Png));
        assert_eq!(
            ImageMedia::sniff(b"\xFF\xD8\xFF\xE0rest"),
            Some(ImageMedia::Jpeg)
        );
        assert_eq!(ImageMedia::sniff(b"GIF87a..."), Some(ImageMedia::Gif));
        assert_eq!(ImageMedia::sniff(b"GIF89a..."), Some(ImageMedia::Gif));
        assert_eq!(
            ImageMedia::sniff(b"RIFF\x10\0\0\0WEBPVP8 "),
            Some(ImageMedia::Webp)
        );
    }

    #[test]
    fn sniff_refuses_text_and_truncated_headers() {
        for bytes in [
            &b"just some text"[..],
            b"",
            b"\x89PNG\r\n",
            b"\xFF\xD8",
            b"GIF8",
            b"RIFF\x10\0\0\0WEB",
            b"RIFF\x10\0\0\0WAVE",
        ] {
            assert_eq!(ImageMedia::sniff(bytes), None, "{bytes:?}");
        }
    }

    #[test]
    fn a_png_named_txt_is_still_a_png_the_name_is_ignored() {
        let attached = admit(&[], "notes.txt", PNG.to_vec()).expect("admitted");
        assert_eq!(attached.media(), ImageMedia::Png);
        assert_eq!(attached.name(), "notes.txt");
    }

    #[test]
    fn mime_names_each_format() {
        assert_eq!(ImageMedia::Png.mime(), "image/png");
        assert_eq!(ImageMedia::Jpeg.mime(), "image/jpeg");
        assert_eq!(ImageMedia::Gif.mime(), "image/gif");
        assert_eq!(ImageMedia::Webp.mime(), "image/webp");
    }

    #[test]
    fn a_picture_of_exactly_the_limit_is_admitted_one_byte_more_is_too_large() {
        assert!(admit(&[], "edge.png", png_of(MAX_IMAGE_BYTES)).is_ok());
        assert_eq!(
            admit(&[], "big.png", png_of(MAX_IMAGE_BYTES + 1)),
            Err(AttachmentRefusal::TooLarge {
                name: "big.png".into(),
                size: MAX_IMAGE_BYTES as u64 + 1
            })
        );
    }

    #[test]
    fn the_twenty_first_picture_is_too_many() {
        let draft: Vec<_> = (0..MAX_IMAGES_PER_MESSAGE)
            .map(|n| image(&format!("{n}.png"), 32))
            .collect();
        assert_eq!(
            admit(&draft, "21.png", png_of(32)),
            Err(AttachmentRefusal::TooMany {
                name: "21.png".into()
            })
        );
        assert!(admit(&draft[1..], "20.png", png_of(32)).is_ok());
    }

    #[test]
    fn pictures_over_the_message_total_are_refused() {
        let draft = vec![image("a.png", 3_500_000); 3];
        let room = MAX_MESSAGE_IMAGE_BYTES - 3 * 3_500_000;
        assert!(admit(&draft, "fits.png", png_of(room)).is_ok());
        assert_eq!(
            admit(&draft, "over.png", png_of(room + 1)),
            Err(AttachmentRefusal::MessageTooLarge {
                name: "over.png".into()
            })
        );
    }

    #[test]
    fn checks_run_in_order_size_before_count_before_total() {
        let full: Vec<_> = (0..MAX_IMAGES_PER_MESSAGE)
            .map(|n| image(&format!("{n}.png"), 32))
            .collect();
        assert!(matches!(
            admit(&full, "big.png", png_of(MAX_IMAGE_BYTES + 1)),
            Err(AttachmentRefusal::TooLarge { .. })
        ));
        let heavy = vec![image("a.png", MAX_IMAGE_BYTES); 3];
        let mut heavy_and_full = heavy.clone();
        heavy_and_full.extend(full.into_iter().skip(3));
        assert!(matches!(
            admit(&heavy_and_full, "x.png", png_of(MAX_IMAGE_BYTES)),
            Err(AttachmentRefusal::TooMany { .. })
        ));
    }

    #[test]
    fn a_file_that_is_no_picture_is_unreadable() {
        assert!(matches!(
            admit(&[], "a.png", b"plain text".to_vec()),
            Err(AttachmentRefusal::Unreadable { name, .. }) if name == "a.png"
        ));
    }

    #[test]
    fn every_refusal_names_the_file_and_says_why() {
        for refusal in [
            AttachmentRefusal::Unreadable {
                name: "shot.png".into(),
                why: "permission denied".into(),
            },
            AttachmentRefusal::TooLarge {
                name: "shot.png".into(),
                size: 9_000_000,
            },
            AttachmentRefusal::TooMany {
                name: "shot.png".into(),
            },
            AttachmentRefusal::MessageTooLarge {
                name: "shot.png".into(),
            },
        ] {
            let sentence = refusal.to_string();
            assert!(sentence.starts_with("shot.png "), "{sentence}");
            assert!(
                sentence.len() > "shot.png was not attached: ".len() + 10,
                "{sentence}"
            );
        }
        let unreadable = AttachmentRefusal::Unreadable {
            name: "x.png".into(),
            why: "permission denied".into(),
        };
        assert!(unreadable.to_string().contains("permission denied"));
    }

    #[test]
    fn meta_carries_name_media_and_size_but_no_bytes() {
        let attached = image("shot.png", 100);
        assert_eq!(
            AttachmentMeta::from(&attached),
            AttachmentMeta {
                name: "shot.png".into(),
                media: ImageMedia::Png,
                size: 100
            }
        );
    }

    #[test]
    fn check_images_refuses_what_admit_would_refuse() {
        let ok = vec![image("a.png", 64), image("b.png", 64)];
        assert_eq!(check_images(&ok), Ok(()));
        let too_many = vec![image("a.png", 32); MAX_IMAGES_PER_MESSAGE + 1];
        assert!(matches!(
            check_images(&too_many),
            Err(AttachmentRefusal::TooMany { .. })
        ));
        let mut lying = image("c.png", 64);
        lying.media = ImageMedia::Gif;
        assert!(matches!(
            check_images(&[lying]),
            Err(AttachmentRefusal::Unreadable { .. })
        ));
    }

    #[test]
    fn a_drop_splits_into_pictures_and_paths_in_drop_order() {
        let paths: Vec<PathBuf> = ["a.png", "src", "b.txt", "c.jpg", "d.rs"]
            .iter()
            .map(PathBuf::from)
            .collect();
        let sniff = |path: &Path| match path.to_str() {
            Some("a.png") => Some(ImageMedia::Png),
            Some("c.jpg") => Some(ImageMedia::Jpeg),
            _ => None,
        };
        assert_eq!(
            classify_drop(&paths, sniff),
            DropPlan {
                images: vec!["a.png".into(), "c.jpg".into()],
                paths: vec!["src".into(), "b.txt".into(), "d.rs".into()],
            }
        );
    }
}
