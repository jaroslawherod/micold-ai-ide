//! The basic image: an SSH server and the tools a session needs, and no daemon or AI CLI (feature
//! 806, FR-002).
//!
//! The client copies its own daemon build into a container started from this image over SSH and
//! installs the AI CLIs the user selected, so the image no longer has to match the app's version.
//! It is built from `packaging/basic/` and published by `.github/workflows/basic-image.yml` under
//! the content hash of that directory, independently of any release.
//!
//! [`BASIC_IMAGE_CONTENT_HASH`] is the hash of the tree the client expects; `scripts/basic-image-hash.sh
//! --check` compares it with `packaging/basic/` (`CURRENT` or `DRIFT <pinned> <tree>`). The digest
//! pin the client pulls by is added once the publish workflow has run on `main` (tasks T049), since
//! no digest exists before then. See `specs/806-container-daemon-ssh-provisioning/contracts/basic-image.md`.

use super::image::{ImageRef, DEFAULT_IMAGE_REPOSITORY, RETIRED_IMAGE_REPOSITORY};

/// Where `basic-image.yml` publishes the basic image.
pub const BASIC_IMAGE_REPOSITORY: &str = "ghcr.io/jaroslawherod/micold-basic";

/// The content hash of `packaging/basic/` this client was built against: the first 16 hex of
/// `scripts/basic-image-hash.sh`. It is also the tag the publish workflow pushes the image under.
///
/// Change it in the same commit as any file under `packaging/basic/`;
/// `scripts/tests/basic-image-hash.test.sh` fails while it disagrees with the tree.
pub const BASIC_IMAGE_CONTENT_HASH: &str = "722a7d9ef0d6aadb";

/// The tag a local build of `packaging/basic/` carries (`docker build -t micold-basic:dev`), and the
/// one the real-runtime suite starts from by default.
pub const DEV_BASIC_IMAGE: &str = "micold-basic:dev";

/// The repositories of the released all-in-one daemon image (feature 027): the one the release
/// publishes and the retired namespace nothing was ever published into. A stored image naming one
/// of them is the app's old default, which the migration replaces with the basic image, and not a
/// user's own image.
pub const RELEASED_IMAGE_REPOSITORIES: [&str; 2] =
    [DEFAULT_IMAGE_REPOSITORY, RETIRED_IMAGE_REPOSITORY];

/// Whether `reference` names a released daemon image, under any tag or digest or none.
///
/// Matched on the parsed repository, so `ghcr.io/jaroslawherod/micold-daemon:0.30.0`,
/// `…@sha256:…` and the bare name all count, and a local `micold-daemon:dev` or the basic image do
/// not. An unparseable reference is not a released image.
pub fn is_released_image(reference: &str) -> bool {
    let Ok(parsed) = ImageRef::parse(reference) else {
        return false;
    };
    let name = match &parsed.registry {
        Some(registry) => format!("{registry}/{}", parsed.repository),
        None => parsed.repository,
    };
    RELEASED_IMAGE_REPOSITORIES.contains(&name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn a_released_repository_is_released_under_any_tag_or_digest_or_none() {
        for repository in RELEASED_IMAGE_REPOSITORIES {
            for reference in [
                repository.to_string(),
                format!("{repository}:0.30.0"),
                format!("{repository}:latest"),
                format!("{repository}@{DIGEST}"),
                format!("{repository}:0.30.0@{DIGEST}"),
            ] {
                assert!(is_released_image(&reference), "{reference}");
            }
        }
    }

    #[test]
    fn the_basic_image_a_local_build_and_another_image_are_not_released() {
        for reference in [
            format!("{BASIC_IMAGE_REPOSITORY}:0123456789abcdef"),
            format!("{BASIC_IMAGE_REPOSITORY}@{DIGEST}"),
            "micold-daemon:dev".to_string(),
            DEV_BASIC_IMAGE.to_string(),
            "docker.io/library/alpine:3".to_string(),
            String::new(),
        ] {
            assert!(!is_released_image(&reference), "{reference}");
        }
    }

    #[test]
    fn the_released_repositories_are_the_published_and_the_retired_one() {
        assert!(RELEASED_IMAGE_REPOSITORIES.contains(&"ghcr.io/jaroslawherod/micold-daemon"));
        assert!(RELEASED_IMAGE_REPOSITORIES.contains(&"ghcr.io/micold/micold-daemon"));
    }

    #[test]
    fn the_content_hash_is_sixteen_lowercase_hex() {
        assert_eq!(
            BASIC_IMAGE_CONTENT_HASH.len(),
            16,
            "{BASIC_IMAGE_CONTENT_HASH:?}"
        );
        assert!(
            BASIC_IMAGE_CONTENT_HASH
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "{BASIC_IMAGE_CONTENT_HASH:?}"
        );
    }
}
