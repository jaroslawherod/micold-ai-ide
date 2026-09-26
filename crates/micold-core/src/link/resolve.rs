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
pub fn resolve(link: Link, ctx: &LinkContext) -> Option<ResolvedLink> {
    match classify(&link.address) {
        Address::Web(address) | Address::Mail(address) => Some(ResolvedLink {
            link,
            display: address.clone(),
            target: Target::Url(address),
            needs_confirmation: false,
        }),
        Address::File { host, path } => file(link, &host, &path, ctx),
        Address::NotFollowable => None,
    }
}

/// What a `file://host/path` link opens here (C4–C12).
///
/// The host decides *whose* file it is, and only this machine's own names and the sandbox's own
/// names pass — an unknown name is another machine, decided without any lookup (C18, FR-019).
fn file(link: Link, host: &str, path: &str, ctx: &LinkContext) -> Option<ResolvedLink> {
    let known = |names: &[String]| names.iter().any(|n| n.eq_ignore_ascii_case(host));
    let sandbox_names = ctx.sandbox.as_ref().map(|s| s.host_names.as_slice());
    if !(host.is_empty()
        || host.eq_ignore_ascii_case("localhost")
        || known(&ctx.host_names)
        || sandbox_names.is_some_and(known))
    {
        return None;
    }
    // A sandboxed session's paths are the container's, not this machine's. Until M6 translates them
    // through the shared locations, every one of them is reported as out of reach (C12), which is
    // the safe direction: nothing opens the host's own file of the same name.
    if ctx.sandbox.is_some() {
        return Some(ResolvedLink {
            link,
            display: format!("{path} — not reachable from this machine"),
            target: Target::Unreachable(Reason::NotShared),
            needs_confirmation: false,
        });
    }
    let path = if ctx.windows_host {
        windows_path(path)?
    } else {
        path.to_string()
    };
    Some(ResolvedLink {
        link,
        display: path.clone(),
        target: Target::HostPath(path),
        needs_confirmation: false,
    })
}

/// `/C:/Users/u/a.txt` as `C:\Users\u\a.txt`, or `None` when the path names no drive (C7, C8).
fn windows_path(path: &str) -> Option<String> {
    let drive = path.strip_prefix('/')?;
    let (letter, rest) = drive.split_at_checked(1)?;
    if !letter.chars().all(|c| c.is_ascii_alphabetic()) || !rest.starts_with(':') {
        return None;
    }
    Some(drive.replace('/', "\\"))
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

    /// A session in a sandbox that shares nothing (C12); M6 fills the locations in.
    fn sandboxed() -> LinkContext {
        LinkContext {
            sandbox: Some(SandboxLinkContext {
                host_names: container_host_names("0123456789abcdef"),
                locations: Vec::new(),
                denied: Vec::new(),
            }),
            ..local()
        }
    }

    /// What `address` resolves to in `ctx`, as the pane would ask.
    fn target(address: &str, ctx: &LinkContext) -> Option<Target> {
        resolve(detected(address), ctx).map(|r| r.target)
    }

    /// U35: C4, C5 — a file on this machine, however it named this machine.
    #[test]
    fn a_file_link_to_this_machine_is_a_path_on_it() {
        for address in [
            "file:///home/u/a.txt",
            "file://localhost/home/u/a.txt",
            "file://LocalHost/home/u/a.txt",
            "file://devbox/home/u/a.txt",
            "file://DevBox/home/u/a.txt",
        ] {
            assert_eq!(
                resolve(detected(address), &local()),
                Some(ResolvedLink {
                    link: detected(address),
                    display: "/home/u/a.txt".to_string(),
                    target: Target::HostPath("/home/u/a.txt".to_string()),
                    needs_confirmation: false,
                }),
                "{address} names this machine, so it opens its path and nothing asks first (C4, C5)"
            );
        }
    }

    /// U36: C6, C18 — a guard. Its red is the mutant in tdd/test-list.md: a `File` branch that
    /// accepts any host.
    #[test]
    fn a_file_link_naming_another_machine_is_no_link() {
        for address in [
            "file://otherhost/x",
            "file://server/share/report.docx",
            "file://build-host.invalid/home/u/a.txt",
        ] {
            assert_eq!(
                target(address, &local()),
                None,
                "{address} is a file on another machine, which this one cannot open (C6, C18) — \
                 and no name is looked up to decide it (FR-019)"
            );
        }
    }

    /// U37: C7, C8 — a Windows host reads the drive out of the path.
    #[test]
    fn on_a_windows_host_a_file_path_needs_a_drive() {
        let windows = LinkContext {
            windows_host: true,
            ..local()
        };
        assert_eq!(
            target("file:///C:/Users/u/a.txt", &windows),
            Some(Target::HostPath(r"C:\Users\u\a.txt".to_string())),
            "the drive letter is what makes it a Windows path, and the separators are its own (C7)"
        );
        assert_eq!(
            target("file:///home/u/a.txt", &windows),
            None,
            "a path with no drive names nothing on a Windows machine (C8)"
        );
    }

    /// U41: C12 — a sandboxed session shares nothing yet, which is the safe direction.
    #[test]
    fn a_sandboxed_file_link_outside_every_shared_location_is_not_reachable() {
        assert_eq!(
            resolve(detected("file:///tmp/x"), &sandboxed()),
            Some(ResolvedLink {
                link: detected("file:///tmp/x"),
                display: "/tmp/x — not reachable from this machine".to_string(),
                target: Target::Unreachable(Reason::NotShared),
                needs_confirmation: false,
            }),
            "the hint says the path is not reachable rather than opening the host's own /tmp/x (C12)"
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
            "file://devbox/p/My%20Doc.pdf",
            "file://otherhost/x",
        ];
        let mut followable = 0;
        // Every context resolution reads, so a `HostPath` and an `Unreachable` are both sampled.
        for ctx in [local(), sandboxed()] {
            for address in addresses {
                let Some(resolved) = resolve(detected(address), &ctx) else {
                    continue;
                };
                let opens = match &resolved.target {
                    Target::Url(opens) | Target::HostPath(opens) => opens,
                    Target::Unreachable(_) => continue,
                };
                followable += 1;
                assert_eq!(
                    &resolved.display, opens,
                    "{address}: the hint shows exactly the string handed to the opener (SC-006)"
                );
            }
        }
        assert_eq!(
            followable, 8,
            "the sample holds three web or mail links in each context, and two host paths on this one"
        );
    }
}
