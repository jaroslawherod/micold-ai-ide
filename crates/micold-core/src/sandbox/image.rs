//! Image references: parsing, moving-tag detection, and the pull/import/build decision.
//!
//! FR-024a requires an offline path to the image, which is what keeps "works fully offline" true
//! rather than nearly true (Principle IV); FR-024c makes the maintainers' own rebuild loop a
//! supported path, and FR-024d requires refusing a stale one.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The registry reference the app ships with. An immutable version tag, never a moving one — a
/// moving tag can change under a running sandbox, which is the case [`ImageRef::is_moving`] exists
/// to detect.
pub const DEFAULT_IMAGE: &str = concat!(
    "ghcr.io/micold/micold-daemon:",
    env!("CARGO_PKG_VERSION")
);

/// The tag `mise run image` produces, and the one a stale-image refusal names (FR-024c).
pub const DEV_IMAGE_TAG: &str = "micold-daemon:dev";

/// Tags that move: the same name can resolve to different content on different days.
///
/// Not a stylistic objection. FR-024b requires the app to notice, because a user whose sandbox was
/// built from yesterday's `:latest` is running something the app cannot name in a bug report.
const MOVING_TAGS: &[&str] = &["latest", "main", "master", "edge", "nightly", "dev"];

/// How the image is obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageSourceKind {
    /// Pulled from a registry. The default, and the only one needing the network.
    #[default]
    Registry,
    /// Loaded from a local archive produced by `docker save` — Principle IV's offline path
    /// (FR-024a). Without this, "works fully offline" would be true only after a first online run.
    ImportedFile,
    /// Built from this working tree by `mise run image` (FR-024c). Held to a stricter staleness
    /// check than the other two, because a locally built image and the client that talks to it
    /// came from the same tree and have no business disagreeing (research R8).
    LocalBuild,
}

/// Where the sandbox image comes from (FR-024 … FR-024d).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageSource {
    /// Which acquisition path applies.
    #[serde(default)]
    pub kind: ImageSourceKind,
    /// The image reference, e.g. `ghcr.io/micold/micold-daemon:0.27.0`.
    #[serde(default = "default_reference")]
    pub reference: String,
    /// The archive to load, set only when [`Self::kind`] is [`ImageSourceKind::ImportedFile`].
    #[serde(default)]
    pub path: Option<PathBuf>,
}

fn default_reference() -> String {
    DEFAULT_IMAGE.to_string()
}

impl Default for ImageSource {
    fn default() -> Self {
        Self {
            kind: ImageSourceKind::default(),
            reference: default_reference(),
            path: None,
        }
    }
}

impl ImageSource {
    /// The parsed reference, or the reason it is unusable.
    pub fn parsed(&self) -> Result<ImageRef, ImageRefError> {
        ImageRef::parse(&self.reference)
    }

    /// Whether a fingerprint mismatch against this source is a refusal.
    ///
    /// Deliberately asymmetric (contracts/protocol-delta.md §2). A released client and a released
    /// daemon are built separately and legitimately carry different fingerprints, so refusing there
    /// would break every normal install. A locally built image that disagrees with the client is
    /// stale by definition, because both came from the same working tree.
    pub fn refuses_fingerprint_mismatch(&self) -> bool {
        self.kind == ImageSourceKind::LocalBuild
    }

    /// Everything wrong with this source, as values rather than prose.
    pub fn validate(&self) -> Vec<ImageSourceProblem> {
        let mut out = Vec::new();
        match self.parsed() {
            Ok(r) => {
                if r.is_moving() {
                    out.push(ImageSourceProblem::MovingTag {
                        tag: r.tag.clone().unwrap_or_default(),
                    });
                }
            }
            Err(e) => out.push(ImageSourceProblem::Unparseable(e)),
        }
        if self.kind == ImageSourceKind::ImportedFile && self.path.is_none() {
            out.push(ImageSourceProblem::MissingArchivePath);
        }
        if self.kind != ImageSourceKind::ImportedFile && self.path.is_some() {
            out.push(ImageSourceProblem::UnusedArchivePath);
        }
        out
    }
}

/// Something wrong with an [`ImageSource`], reported to the view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageSourceProblem {
    /// The reference does not parse.
    Unparseable(ImageRefError),
    /// A warning, not an error: the app works, but cannot name what it is running (FR-024b).
    MovingTag { tag: String },
    /// An import with nothing to import.
    MissingArchivePath,
    /// An archive path on a source that will never read it — a leftover from a changed mind.
    UnusedArchivePath,
}

impl ImageSourceProblem {
    /// Whether this stops the sandbox from starting, as opposed to warning about it.
    pub fn is_fatal(&self) -> bool {
        !matches!(self, ImageSourceProblem::MovingTag { .. })
    }

    /// The message the view shows.
    pub fn message(&self) -> String {
        match self {
            ImageSourceProblem::Unparseable(e) => e.message(),
            ImageSourceProblem::MovingTag { tag } => format!(
                "`{tag}` is a moving tag: the image behind it can change while a sandbox built \
                 from the previous one is still running. Prefer a version tag."
            ),
            ImageSourceProblem::MissingArchivePath => {
                "Importing from a file needs the archive's path.".to_string()
            }
            ImageSourceProblem::UnusedArchivePath => {
                "This archive path is not used by the selected image source.".to_string()
            }
        }
    }
}

/// A parsed image reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRef {
    /// The registry host, if the reference named one.
    pub registry: Option<String>,
    /// The repository path, e.g. `micold/micold-daemon`.
    pub repository: String,
    /// The tag, if the reference is tagged rather than digest-pinned.
    pub tag: Option<String>,
    /// The digest, if the reference is pinned. A pinned reference cannot move.
    pub digest: Option<String>,
}

/// Why a reference could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageRefError {
    /// Nothing but whitespace.
    Empty,
    /// A tag or digest with no name in front of it.
    MissingRepository,
    /// A component the runtimes will not accept.
    InvalidCharacter(char),
}

impl ImageRefError {
    /// The message the view shows.
    pub fn message(&self) -> String {
        match self {
            ImageRefError::Empty => "An image reference is required.".to_string(),
            ImageRefError::MissingRepository => {
                "This reference has a tag but no image name.".to_string()
            }
            ImageRefError::InvalidCharacter(c) => {
                format!("`{c}` is not valid in an image reference.")
            }
        }
    }
}

impl ImageRef {
    /// Parse a reference of the form `[registry/]repository[:tag|@digest]`.
    ///
    /// Hand-written rather than pulled in as a dependency: the grammar is small, and this is the
    /// same trade the constitution's dependency constraint asks for elsewhere in the crate.
    pub fn parse(input: &str) -> Result<Self, ImageRefError> {
        let s = input.trim();
        if s.is_empty() {
            return Err(ImageRefError::Empty);
        }
        if let Some(c) = s
            .chars()
            .find(|c| c.is_whitespace() || matches!(c, '\\' | '"' | '\'' | ' '))
        {
            return Err(ImageRefError::InvalidCharacter(c));
        }

        // A digest binds tighter than a tag, and `@` cannot appear in either.
        let (name, digest) = match s.split_once('@') {
            Some((n, d)) => (n, Some(d.to_string())),
            None => (s, None),
        };

        // The colon that introduces a tag is the last one *after* the final slash — a registry may
        // carry a port (`localhost:5000/img`), and that colon is not a tag separator.
        let last_slash = name.rfind('/');
        let tag_sep = name.rfind(':').filter(|i| match last_slash {
            Some(sl) => *i > sl,
            None => true,
        });
        let (name, tag) = match tag_sep {
            Some(i) => (&name[..i], Some(name[i + 1..].to_string())),
            None => (name, None),
        };
        if name.is_empty() {
            return Err(ImageRefError::MissingRepository);
        }

        // A first component is a registry only if it looks like a host: it carries a dot, a colon,
        // or is exactly `localhost`. Otherwise it is the first path segment of the repository.
        let (registry, repository) = match name.split_once('/') {
            Some((head, rest))
                if head.contains('.') || head.contains(':') || head == "localhost" =>
            {
                (Some(head.to_string()), rest.to_string())
            }
            _ => (None, name.to_string()),
        };
        if repository.is_empty() {
            return Err(ImageRefError::MissingRepository);
        }

        Ok(ImageRef {
            registry,
            repository,
            tag,
            digest,
        })
    }

    /// Whether the content behind this reference can change without the reference changing.
    ///
    /// A digest cannot move by construction. An untagged reference means `:latest`, which can — so
    /// omitting the tag is treated as the moving case rather than the neutral one.
    pub fn is_moving(&self) -> bool {
        if self.digest.is_some() {
            return false;
        }
        match &self.tag {
            None => true,
            Some(t) => MOVING_TAGS.iter().any(|m| m.eq_ignore_ascii_case(t)),
        }
    }

    /// The reference as the runtime spells it.
    pub fn to_reference(&self) -> String {
        let mut out = String::new();
        if let Some(r) = &self.registry {
            out.push_str(r);
            out.push('/');
        }
        out.push_str(&self.repository);
        if let Some(d) = &self.digest {
            out.push('@');
            out.push_str(d);
        } else if let Some(t) = &self.tag {
            out.push(':');
            out.push_str(t);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_registry_reference_round_trips() {
        let r = ImageRef::parse("ghcr.io/micold/micold-daemon:0.27.0").unwrap();
        assert_eq!(r.registry.as_deref(), Some("ghcr.io"));
        assert_eq!(r.repository, "micold/micold-daemon");
        assert_eq!(r.tag.as_deref(), Some("0.27.0"));
        assert_eq!(r.to_reference(), "ghcr.io/micold/micold-daemon:0.27.0");
    }

    #[test]
    fn a_registry_port_is_not_mistaken_for_a_tag() {
        // The colon in `localhost:5000` introduces a port, not a tag. Getting this wrong would
        // silently pull `localhost` at tag `5000/img`.
        let r = ImageRef::parse("localhost:5000/micold-daemon:dev").unwrap();
        assert_eq!(r.registry.as_deref(), Some("localhost:5000"));
        assert_eq!(r.repository, "micold-daemon");
        assert_eq!(r.tag.as_deref(), Some("dev"));
    }

    #[test]
    fn a_bare_name_has_no_registry() {
        let r = ImageRef::parse("micold-daemon:dev").unwrap();
        assert_eq!(r.registry, None);
        assert_eq!(r.repository, "micold-daemon");
    }

    #[test]
    fn a_first_segment_without_a_dot_is_part_of_the_repository() {
        let r = ImageRef::parse("micold/micold-daemon:0.1.0").unwrap();
        assert_eq!(r.registry, None);
        assert_eq!(r.repository, "micold/micold-daemon");
    }

    #[test]
    fn a_digest_pins_the_reference() {
        let r = ImageRef::parse("micold-daemon@sha256:abc123").unwrap();
        assert_eq!(r.digest.as_deref(), Some("sha256:abc123"));
        assert_eq!(r.tag, None);
        assert!(!r.is_moving(), "a digest cannot move");
    }

    #[test]
    fn moving_tags_are_detected_including_the_omitted_one() {
        // An omitted tag means `:latest`, so it is the moving case, not the neutral one.
        assert!(ImageRef::parse("micold-daemon").unwrap().is_moving());
        assert!(ImageRef::parse("micold-daemon:latest").unwrap().is_moving());
        assert!(ImageRef::parse("micold-daemon:LATEST").unwrap().is_moving());
        assert!(ImageRef::parse("micold-daemon:dev").unwrap().is_moving());
        assert!(!ImageRef::parse("micold-daemon:0.27.0").unwrap().is_moving());
    }

    #[test]
    fn unparseable_references_are_classified_not_panicked() {
        assert_eq!(ImageRef::parse("   "), Err(ImageRefError::Empty));
        assert_eq!(ImageRef::parse(":dev"), Err(ImageRefError::MissingRepository));
        assert!(matches!(
            ImageRef::parse("micold daemon:dev"),
            Err(ImageRefError::InvalidCharacter(_))
        ));
    }

    #[test]
    fn the_shipped_default_is_a_parseable_immutable_reference() {
        // If this ever fails, the app ships pointing at something that can change underneath it.
        let r = ImageSource::default().parsed().expect("default parses");
        assert!(!r.is_moving(), "the shipped default must not be a moving tag");
    }

    #[test]
    fn only_a_local_build_refuses_on_a_fingerprint_mismatch() {
        // The asymmetry research R8 requires: released client and daemon are built separately and
        // legitimately differ; a locally built pair came from one tree and must not.
        let local = ImageSource {
            kind: ImageSourceKind::LocalBuild,
            reference: DEV_IMAGE_TAG.to_string(),
            path: None,
        };
        assert!(local.refuses_fingerprint_mismatch());
        assert!(!ImageSource::default().refuses_fingerprint_mismatch());
    }

    #[test]
    fn an_import_without_an_archive_is_a_fatal_problem() {
        let src = ImageSource {
            kind: ImageSourceKind::ImportedFile,
            reference: "micold-daemon:0.27.0".to_string(),
            path: None,
        };
        let problems = src.validate();
        assert_eq!(problems, vec![ImageSourceProblem::MissingArchivePath]);
        assert!(problems[0].is_fatal());
    }

    #[test]
    fn a_moving_tag_warns_without_blocking() {
        let src = ImageSource {
            kind: ImageSourceKind::Registry,
            reference: "micold-daemon:latest".to_string(),
            path: None,
        };
        let problems = src.validate();
        assert_eq!(problems.len(), 1);
        assert!(!problems[0].is_fatal(), "a moving tag warns, it does not block");
    }
}
