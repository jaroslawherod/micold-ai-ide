//! What a link opens on this machine (research R8, R10; contract link-recognition §4).

use super::Link;

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
