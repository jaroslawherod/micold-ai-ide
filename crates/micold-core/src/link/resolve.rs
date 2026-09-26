//! What a link opens on this machine (research R8, R10; contract link-recognition §4).

use super::{
    address::{classify, Address},
    Link,
};

/// What resolution may consult, all known without I/O.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkContext {
    /// This machine's hostname and its first DNS label.
    pub host_names: Vec<String>,
    /// The client runs on Windows: a parameter, not a `cfg`.
    pub windows_host: bool,
    /// `Some` iff the session runs in a sandbox with known shared locations.
    pub sandbox: Option<SandboxLinkContext>,
}

/// The sandbox's side of resolution (research R10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxLinkContext {
    /// The container id's 12-character prefix and the full id.
    pub host_names: Vec<String>,
    /// Most specific first; excludes the secret mount and every location the container lacks.
    pub locations: Vec<SharedLocation>,
    /// Host paths never returned, even through a shared location.
    pub denied: Vec<String>,
}

/// A container path shared with a host path (the spec's *Shared location*).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedLocation {
    pub container: String,
    pub host: String,
}

/// What the hint displays and what an activation carries (FR-008, SC-006).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedLink {
    pub link: Link,
    /// Exactly what the hint shows.
    pub display: String,
    pub target: Target,
    /// `true` iff the session is sandboxed and `target` is a host path (FR-018a).
    pub needs_confirmation: bool,
}

/// Where an activation goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A web or mail address, handed to the operating system as is.
    Url(String),
    /// A file or folder on this machine, not yet checked to exist.
    HostPath(String),
    /// A sandboxed file path outside every shared location.
    Unreachable(Reason),
}

/// Why a target cannot be reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    NotShared,
}

/// The names this machine answers to in a `file://` address (FR-012).
///
/// The hostname as reported, plus its first DNS label when that is shorter: `ls --hyperlink=always`
/// prints whichever of the two the machine calls itself, and both mean this machine. An empty name
/// gives none, so a link naming a host opens nothing rather than the wrong thing.
pub fn host_names_from(raw: &str) -> Vec<String> {
    if raw.is_empty() {
        return Vec::new();
    }
    let mut names = vec![raw.to_string()];
    let label = raw.split('.').next().unwrap_or(raw);
    if label != raw && !label.is_empty() {
        names.push(label.to_string());
    }
    names
}

/// The names a container answers to (FR-018, C11).
///
/// A container's hostname is its id's 12-character prefix, which is also what its shell prompt
/// shows; a program may print the full id instead. An id shorter than the prefix is its own prefix.
pub fn container_host_names(id: &str) -> Vec<String> {
    let prefix = id.get(..CONTAINER_ID_PREFIX).unwrap_or(id);
    let mut names = vec![prefix.to_string()];
    if prefix != id {
        names.push(id.to_string());
    }
    names
}

/// How much of a container id its hostname is (C11).
const CONTAINER_ID_PREFIX: usize = 12;

/// What `link` opens on this machine, or `None` when it is not followable here (FR-008, FR-012,
/// FR-018).
pub fn resolve(link: Link, _ctx: &LinkContext) -> Option<ResolvedLink> {
    match classify(&link.address) {
        Address::Web(address) | Address::Mail(address) => Some(ResolvedLink {
            link,
            display: address.clone(),
            target: Target::Url(address),
            needs_confirmation: false,
        }),
        Address::NotFollowable => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::LinkOrigin;

    /// A session on this machine, not sandboxed, not on Windows.
    fn local() -> LinkContext {
        LinkContext {
            host_names: vec!["devbox".to_string()],
            windows_host: false,
            sandbox: None,
        }
    }

    fn detected(address: &str) -> Link {
        Link {
            address: address.to_string(),
            origin: LinkOrigin::Detected,
            cells: Vec::new(),
        }
    }

    #[test]
    fn a_web_or_mail_link_opens_its_address_as_shown() {
        for address in ["https://a.example/x?y=1", "mailto:team@example.com"] {
            let link = detected(address);
            assert_eq!(
                resolve(link.clone(), &local()),
                Some(ResolvedLink {
                    link,
                    display: address.to_string(),
                    target: Target::Url(address.to_string()),
                    needs_confirmation: false,
                }),
                "{address} opens as written, the hint shows it as written, and nothing asks first"
            );
        }
    }

    #[test]
    fn a_declared_uri_that_is_not_followable_resolves_to_nothing() {
        for address in [
            "vscode://file/home/u/a.rs",
            "javascript:alert(1)",
            "gopher://a.example/",
        ] {
            let link = Link {
                address: address.to_string(),
                origin: LinkOrigin::Declared,
                cells: Vec::new(),
            };
            assert_eq!(
                resolve(link, &local()),
                None,
                "a program may declare {address}, but it is never offered as a link"
            );
        }
    }

    /// Replaced by contract link-recognition C4–C10 when file links land (T042).
    #[test]
    fn a_file_link_resolves_to_nothing_before_file_links_land() {
        assert_eq!(
            resolve(detected("file:///tmp/x"), &local()),
            None,
            "file links open nothing until their resolution exists"
        );
    }

    /// U140: the names a `file://` host is compared against, from this machine's hostname.
    #[test]
    fn this_machines_names_are_its_hostname_and_its_first_dns_label() {
        assert_eq!(
            host_names_from("build.example.com"),
            vec!["build.example.com".to_string(), "build".to_string()],
            "a qualified name answers to itself and to its first label (FR-012)"
        );
        assert_eq!(
            host_names_from("devbox"),
            vec!["devbox".to_string()],
            "a dotless name is listed once, not twice"
        );
        assert_eq!(
            host_names_from(""),
            Vec::<String>::new(),
            "no hostname is no name: a file link naming any host then opens nothing"
        );
    }

    /// U141: a container reports itself by the short id a shell prompt shows, or by the full one.
    #[test]
    fn a_containers_names_are_its_twelve_character_prefix_and_its_full_id() {
        let full = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert_eq!(
            container_host_names(full),
            vec!["0123456789ab".to_string(), full.to_string()],
            "a 64-character id answers to its 12-character prefix and to itself (C11)"
        );
        assert_eq!(
            container_host_names("0123456789ab"),
            vec!["0123456789ab".to_string()],
            "an id that is its own prefix is listed once"
        );
        assert_eq!(
            container_host_names("01234567"),
            vec!["01234567".to_string()],
            "an id shorter than the prefix length is its own prefix (C11)"
        );
    }

    #[test]
    fn what_the_hint_shows_is_exactly_what_opens() {
        let addresses = [
            "https://a.example/x?y=1#z",
            "HTTP://LOCALHOST:8080/",
            "mailto:team@example.com?subject=Hi%20there",
            "vscode://file/home/u/a.rs",
            "file:///tmp/x",
        ];
        let mut followable = 0;
        for address in addresses {
            let Some(resolved) = resolve(detected(address), &local()) else {
                continue;
            };
            followable += 1;
            let opens = match &resolved.target {
                Target::Url(opens) | Target::HostPath(opens) => opens,
                Target::Unreachable(_) => continue,
            };
            assert_eq!(
                &resolved.display, opens,
                "{address}: the hint shows exactly the string handed to the opener (SC-006)"
            );
        }
        assert_eq!(followable, 3, "the sample holds three followable links");
    }
}
