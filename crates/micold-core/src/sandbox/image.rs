//! Image references: parsing, moving-tag detection, and the pull/import/build decision.
//!
//! FR-024a requires an offline path to the image, which is what keeps "works fully offline" true
//! rather than nearly true (Principle IV); FR-024c makes the maintainers' own rebuild loop a
//! supported path, and FR-024d requires refusing a stale one.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::basic_image::{is_released_image, BASIC_IMAGE};

/// The registry namespace the release publishes into — this repository's owner on GHCR.
///
/// Split out from [`DEFAULT_IMAGE`] so the release workflow has one string to check itself
/// against. `.github/workflows/release.yml` greps this file for the repository it is about to push
/// to, and fails the release if it does not appear: a default that points somewhere nothing is
/// published is exactly the state this feature shipped in until 0.11.0, and it is invisible —
/// the app looks correct, and only a user's first pull discovers the name resolves to nothing.
pub const DEFAULT_IMAGE_REPOSITORY: &str = "ghcr.io/jaroslawherod/micold-daemon";

/// The registry reference the app ships with. An immutable version tag, never a moving one — a
/// moving tag can change under a running sandbox, which is the case [`ImageRef::is_moving`] exists
/// to detect.
///
/// The tag is the application's own version, so the client and the image it asks for are released
/// together (FR-024). What makes that reference resolvable is the `image` job in
/// `.github/workflows/release.yml`, which builds this exact tag for amd64 and arm64 and pushes it
/// before the release leaves draft — so a published release cannot name an image that does not
/// exist.
pub const DEFAULT_IMAGE: &str = concat!(
    "ghcr.io/jaroslawherod/micold-daemon:",
    env!("CARGO_PKG_VERSION")
);

/// The namespace `DEFAULT_IMAGE` used to name, and which nothing was ever published into.
///
/// Correcting the constant (FR-024) fixed the default. It did not fix the *files* — settings are
/// stored as values, not as "whatever the app defaults to today", so every user who had opened the
/// sandbox section before 0.12.0 has this namespace written to disk and would keep asking a
/// registry for an image that has never existed there. [`ImageSource::repair_retired_namespace`]
/// is what reaches them.
pub(crate) const RETIRED_IMAGE_REPOSITORY: &str = "ghcr.io/micold/micold-daemon";

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
    /// The image reference, e.g. `ghcr.io/jaroslawherod/micold-daemon:0.11.0`.
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

    /// Replace a reference into the retired namespace with today's default.
    ///
    /// Applied on read, alongside the budget clamp, for the reason rule S-7 gives: a value the user
    /// did not choose and cannot act on is repaired when the file is opened rather than reported.
    /// And they did not choose it — nothing was ever pushed to `ghcr.io/micold`, so any registry
    /// reference naming it is a default the app wrote there itself, tag included. Replacing the
    /// whole reference rather than rewriting the namespace is deliberate: the persisted tag is the
    /// version that wrote the file, and the new namespace has no such tag either.
    ///
    /// Scoped to [`ImageSourceKind::Registry`]: an imported archive or a local build named after
    /// that namespace is a name for something the user has on disk, and renaming it would break a
    /// working setup to fix a reference nothing pulls.
    pub fn repair_retired_namespace(&mut self) {
        if self.kind != ImageSourceKind::Registry {
            return;
        }
        let repository = self
            .reference
            .rsplit_once(':')
            .map(|(repo, _)| repo)
            .unwrap_or(&self.reference);
        if repository == RETIRED_IMAGE_REPOSITORY {
            self.reference = DEFAULT_IMAGE.to_string();
        }
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
    /// "My own image" with no reference and no archive.
    MissingImage,
    /// The reference names the released daemon image (FR-003).
    ReleasedImage,
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
            ImageSourceProblem::MissingImage => {
                "Enter an image reference or choose an archive.".to_string()
            }
            ImageSourceProblem::ReleasedImage => RELEASED_IMAGE_REFUSED.to_string(),
            ImageSourceProblem::UnusedArchivePath => {
                "This archive path is not used by the selected image source.".to_string()
            }
        }
    }
}

/// Which image a container daemon starts from (feature 806, FR-001): exactly two choices.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImageChoice {
    /// The pinned basic image ([`BASIC_IMAGE`]); the client copies its own daemon into it.
    #[default]
    Basic,
    /// The user's own image, which must run an SSH server.
    Own(OwnImage),
}

/// Where the user's own image comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnImage {
    /// A reference the runtime pulls (or finds locally).
    Reference(String),
    /// An archive produced by `docker save`, loaded with the runtime's `load`.
    Archive(PathBuf),
}

/// What the form holds while "My own image" is edited: both fields, either may be empty.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OwnImageDraft {
    /// The reference field.
    pub reference: String,
    /// The archive picker's value.
    pub archive: Option<PathBuf>,
}

/// The local name an archive's image is tagged with after `load`, since an archive names no
/// reference the container could be created from.
pub const LOADED_ARCHIVE_IMAGE: &str = "micold-own-image:archive";

/// The message for a reference naming the released daemon image (FR-003).
pub const RELEASED_IMAGE_REFUSED: &str =
    "The released daemon image is no longer used; choose the basic image.";

impl OwnImageDraft {
    /// The image this draft stands for, or the problem to show beside the image field.
    pub fn validate(&self) -> Result<OwnImage, ImageSourceProblem> {
        if let Some(path) = &self.archive {
            return Ok(OwnImage::Archive(path.clone()));
        }
        let reference = self.reference.trim();
        if reference.is_empty() {
            return Err(ImageSourceProblem::MissingImage);
        }
        if let Err(e) = ImageRef::parse(reference) {
            return Err(ImageSourceProblem::Unparseable(e));
        }
        if is_released_image(reference) {
            return Err(ImageSourceProblem::ReleasedImage);
        }
        Ok(OwnImage::Reference(reference.to_string()))
    }
}

impl From<&ImageChoice> for OwnImageDraft {
    fn from(choice: &ImageChoice) -> Self {
        match choice {
            ImageChoice::Own(OwnImage::Reference(r)) => Self {
                reference: r.clone(),
                archive: None,
            },
            ImageChoice::Own(OwnImage::Archive(p)) => Self {
                reference: String::new(),
                archive: Some(p.clone()),
            },
            ImageChoice::Basic => Self::default(),
        }
    }
}

impl ImageChoice {
    /// The reference the runtime pulls and the container records: [`BASIC_IMAGE`] for `Basic`; for
    /// an archive, [`LOADED_ARCHIVE_IMAGE`], the name it is tagged with once loaded.
    pub fn reference(&self) -> &str {
        match self {
            ImageChoice::Basic => BASIC_IMAGE,
            ImageChoice::Own(OwnImage::Reference(r)) => r,
            ImageChoice::Own(OwnImage::Archive(_)) => LOADED_ARCHIVE_IMAGE,
        }
    }

    /// Everything wrong with this choice, as values.
    pub fn validate(&self) -> Vec<ImageSourceProblem> {
        match self {
            ImageChoice::Basic => Vec::new(),
            ImageChoice::Own(_) => OwnImageDraft::from(self)
                .validate()
                .err()
                .into_iter()
                .collect(),
        }
    }
}

impl ImageSource {
    /// What this stored (pre-806) source becomes, and whether it was the released image the
    /// migration moves off (contracts/daemon-settings.md, read rules).
    pub fn into_choice(mut self) -> (ImageChoice, bool) {
        self.repair_retired_namespace();
        match self.kind {
            ImageSourceKind::Registry if is_released_image(&self.reference) => {
                (ImageChoice::Basic, true)
            }
            ImageSourceKind::ImportedFile => match self.path {
                Some(path) => (ImageChoice::Own(OwnImage::Archive(path)), false),
                None => (ImageChoice::Basic, false),
            },
            _ if self.reference.trim().is_empty() => (ImageChoice::Basic, false),
            _ => (ImageChoice::Own(OwnImage::Reference(self.reference)), false),
        }
    }
}

/// A parsed image reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRef {
    /// The registry host, if the reference named one.
    pub registry: Option<String>,
    /// The repository path, e.g. `jaroslawherod/micold-daemon`.
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
    fn the_default_choice_is_the_digest_pinned_basic_image() {
        assert_eq!(ImageChoice::default(), ImageChoice::Basic);
        assert_eq!(ImageChoice::Basic.reference(), BASIC_IMAGE);
        assert!(ImageRef::parse(BASIC_IMAGE).unwrap().digest.is_some());
    }

    #[test]
    fn an_own_image_draft_is_refused_naming_what_is_wrong() {
        let draft = |reference: &str| OwnImageDraft {
            reference: reference.into(),
            archive: None,
        };
        assert_eq!(draft("").validate(), Err(ImageSourceProblem::MissingImage));
        assert!(draft("")
            .validate()
            .unwrap_err()
            .message()
            .contains("image"));
        let e = draft("::bad ref::").validate().unwrap_err();
        assert!(matches!(e, ImageSourceProblem::Unparseable(_)), "{e:?}");
        let released = draft("ghcr.io/jaroslawherod/micold-daemon:0.30.0")
            .validate()
            .unwrap_err();
        assert_eq!(
            released.message(),
            "The released daemon image is no longer used; choose the basic image."
        );
        assert_eq!(
            draft(" alpine:3 ").validate(),
            Ok(OwnImage::Reference("alpine:3".into()))
        );
        let archive = OwnImageDraft {
            reference: String::new(),
            archive: Some("/x.tar".into()),
        };
        assert_eq!(archive.validate(), Ok(OwnImage::Archive("/x.tar".into())));
    }

    #[test]
    fn each_stored_source_becomes_the_choice_the_contract_names() {
        let src = |kind, reference: &str, path: Option<&str>| ImageSource {
            kind,
            reference: reference.into(),
            path: path.map(PathBuf::from),
        };
        let reg = ImageSourceKind::Registry;
        let own = |r: &str| ImageChoice::Own(OwnImage::Reference(r.into()));
        for released in [
            "ghcr.io/jaroslawherod/micold-daemon:0.30.0",
            "ghcr.io/micold/micold-daemon:0.10.0",
            "ghcr.io/jaroslawherod/micold-daemon",
        ] {
            assert_eq!(
                src(reg, released, None).into_choice(),
                (ImageChoice::Basic, true)
            );
        }
        assert_eq!(
            src(reg, "acme/x:1", None).into_choice(),
            (own("acme/x:1"), false)
        );
        assert_eq!(
            src(ImageSourceKind::LocalBuild, "micold-daemon:dev", None).into_choice(),
            (own("micold-daemon:dev"), false)
        );
        assert_eq!(
            src(ImageSourceKind::ImportedFile, "x:1", Some("/a.tar")).into_choice(),
            (ImageChoice::Own(OwnImage::Archive("/a.tar".into())), false)
        );
    }

    #[test]
    fn a_choice_round_trips_through_json_in_the_contract_shape() {
        let json = serde_json::to_string(&ImageChoice::Basic).unwrap();
        assert_eq!(json, r#"{"kind":"basic"}"#);
        let own = ImageChoice::Own(OwnImage::Reference("a:1".into()));
        assert_eq!(
            serde_json::to_string(&own).unwrap(),
            r#"{"kind":"own","reference":"a:1"}"#
        );
        assert_eq!(
            serde_json::from_str::<ImageChoice>(&json).unwrap(),
            ImageChoice::Basic
        );
    }

    #[test]
    fn a_persisted_reference_into_the_retired_namespace_is_replaced_by_the_default() {
        // The exact value found on this developer's own machine on 2026-08-27: written by the app
        // itself, at a version whose default named a namespace nothing was ever pushed to.
        let mut src = ImageSource {
            kind: ImageSourceKind::Registry,
            reference: "ghcr.io/micold/micold-daemon:0.10.0".into(),
            path: None,
        };
        src.repair_retired_namespace();
        assert_eq!(src.reference, DEFAULT_IMAGE);
    }

    #[test]
    fn a_reference_the_user_chose_is_left_alone() {
        for reference in [
            "ghcr.io/jaroslawherod/micold-daemon:0.11.0",
            "registry.example.com/micold/micold-daemon:1.0.0",
            "ghcr.io/micold/something-else:0.10.0",
        ] {
            let mut src = ImageSource {
                kind: ImageSourceKind::Registry,
                reference: reference.into(),
                path: None,
            };
            src.repair_retired_namespace();
            assert_eq!(src.reference, reference, "{reference} should be untouched");
        }
    }

    #[test]
    fn the_retired_namespace_is_not_the_one_being_published_to() {
        // If these ever agreed, the repair above would rewrite every correct reference to itself
        // forever, and a real regression of the namespace would be indistinguishable from a fix.
        assert_ne!(DEFAULT_IMAGE_REPOSITORY, RETIRED_IMAGE_REPOSITORY);
    }

    #[test]
    fn an_archive_or_local_build_named_after_the_retired_namespace_is_left_alone() {
        // These name something on disk, not something to pull. Rewriting them would break a setup
        // that works in order to fix a reference nothing resolves.
        for kind in [ImageSourceKind::ImportedFile, ImageSourceKind::LocalBuild] {
            let mut src = ImageSource {
                kind,
                reference: "ghcr.io/micold/micold-daemon:0.10.0".into(),
                path: Some(PathBuf::from("/tmp/img.tar")),
            };
            src.repair_retired_namespace();
            assert_eq!(src.reference, "ghcr.io/micold/micold-daemon:0.10.0");
        }
    }

    #[test]
    fn a_registry_reference_round_trips() {
        let r = ImageRef::parse("ghcr.io/jaroslawherod/micold-daemon:0.11.0").unwrap();
        assert_eq!(r.registry.as_deref(), Some("ghcr.io"));
        assert_eq!(r.repository, "jaroslawherod/micold-daemon");
        assert_eq!(r.tag.as_deref(), Some("0.11.0"));
        assert_eq!(
            r.to_reference(),
            "ghcr.io/jaroslawherod/micold-daemon:0.11.0"
        );
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
        let r = ImageRef::parse("jaroslawherod/micold-daemon:0.1.0").unwrap();
        assert_eq!(r.registry, None);
        assert_eq!(r.repository, "jaroslawherod/micold-daemon");
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
        assert_eq!(
            ImageRef::parse(":dev"),
            Err(ImageRefError::MissingRepository)
        );
        assert!(matches!(
            ImageRef::parse("micold daemon:dev"),
            Err(ImageRefError::InvalidCharacter(_))
        ));
    }

    #[test]
    fn the_shipped_default_is_a_parseable_immutable_reference() {
        // If this ever fails, the app ships pointing at something that can change underneath it.
        let r = ImageSource::default().parsed().expect("default parses");
        assert!(
            !r.is_moving(),
            "the shipped default must not be a moving tag"
        );
    }

    #[test]
    fn the_default_names_the_repository_the_release_publishes_to() {
        // Two constants, one fact. The release workflow greps this file for
        // `DEFAULT_IMAGE_REPOSITORY`'s value and refuses to publish a release whose image job
        // would push anywhere else; that check is only worth anything while the reference the app
        // actually resolves is built from the same string. Splitting them and letting them drift
        // would leave the grep passing against a constant nothing reads.
        let r = ImageSource::default().parsed().expect("default parses");
        assert_eq!(
            format!(
                "{}/{}",
                r.registry.as_deref().unwrap_or_default(),
                r.repository
            ),
            DEFAULT_IMAGE_REPOSITORY
        );
        assert_eq!(
            r.tag.as_deref(),
            Some(env!("CARGO_PKG_VERSION")),
            "the image is tagged with the application version it ships beside (FR-024)"
        );
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
        assert!(
            !problems[0].is_fatal(),
            "a moving tag warns, it does not block"
        );
    }
}
