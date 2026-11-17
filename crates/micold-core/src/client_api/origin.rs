//! The `Origin` allow-list of `/api/ws` (FR-004, research R6, R11): exact origins, compared on
//! scheme, host and port, scheme and host case-insensitively, the default ports 80 and 443
//! normalised. No wildcard; `Origin: null` and a malformed value are refused. A request with no
//! `Origin` (not a browser) is accepted from a loopback peer only.

/// An origin reduced to what is compared.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Normalised {
    scheme: String,
    host: String,
    port: u16,
}

/// `text` as `scheme://host[:port]`, or `None` when it is not one (`null` included).
fn normalise(text: &str) -> Option<Normalised> {
    let (scheme, rest) = text.trim().split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme.is_empty()
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    {
        return None;
    }
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    if rest.is_empty() || rest.contains(['/', '?', '#', '@', ' ']) {
        return None;
    }
    let (host, port) = if let Some(v6) = rest.strip_prefix('[') {
        let (inside, after) = v6.split_once(']')?;
        let port = match after {
            "" => None,
            p => Some(p.strip_prefix(':')?),
        };
        (format!("[{}]", inside.to_ascii_lowercase()), port)
    } else {
        match rest.split_once(':') {
            Some((host, port)) => (host.to_ascii_lowercase(), Some(port)),
            None => (rest.to_ascii_lowercase(), None),
        }
    };
    if host.is_empty() || host == "[]" {
        return None;
    }
    let port = match port {
        Some(p) => p.parse().ok()?,
        None => match scheme.as_str() {
            "http" | "ws" => 80,
            "https" | "wss" => 443,
            _ => return None,
        },
    };
    Some(Normalised { scheme, host, port })
}

/// The origins allowed to open `/api/ws`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OriginAllowList {
    origins: Vec<Normalised>,
}

impl OriginAllowList {
    /// The list of `origins`, or the first one that is not an exact origin.
    pub fn new<S: AsRef<str>>(origins: impl IntoIterator<Item = S>) -> Result<Self, String> {
        let origins = origins
            .into_iter()
            .map(|o| {
                let o = o.as_ref();
                normalise(o)
                    .ok_or_else(|| format!("{o:?} is not an origin like http://127.0.0.1:8731"))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { origins })
    }

    /// Whether an upgrade with `origin` (the header, when present) from a peer that is or is not
    /// on loopback may go on.
    pub fn allows(&self, origin: Option<&str>, peer_is_loopback: bool) -> bool {
        match origin {
            None => peer_is_loopback,
            Some(text) => normalise(text).is_some_and(|o| self.origins.contains(&o)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> OriginAllowList {
        OriginAllowList::new(["http://127.0.0.1:8731", "https://Example.COM"]).unwrap()
    }

    #[test]
    fn an_allowed_origin_is_allowed() {
        assert!(list().allows(Some("http://127.0.0.1:8731"), false));
    }

    #[test]
    fn scheme_and_host_compare_case_insensitively() {
        assert!(list().allows(Some("HTTPS://example.com"), false));
    }

    #[test]
    fn default_ports_are_normalised() {
        assert!(list().allows(Some("https://example.com:443"), false));
        let plain = OriginAllowList::new(["http://localhost"]).unwrap();
        assert!(plain.allows(Some("http://localhost:80"), false));
    }

    #[test]
    fn the_same_host_on_another_port_is_refused() {
        assert!(!list().allows(Some("http://127.0.0.1:9999"), true));
    }

    #[test]
    fn another_scheme_is_refused() {
        assert!(!list().allows(Some("https://127.0.0.1:8731"), true));
    }

    #[test]
    fn a_foreign_origin_is_refused() {
        assert!(!list().allows(Some("https://evil.example"), true));
    }

    #[test]
    fn null_and_malformed_origins_are_refused() {
        for bad in [
            "null",
            "",
            "127.0.0.1:8731",
            "http://",
            "http://a:b",
            "http://a/x",
            "*",
        ] {
            assert!(!list().allows(Some(bad), true), "{bad:?}");
        }
    }

    #[test]
    fn no_wildcard_entry_is_accepted() {
        assert!(OriginAllowList::new(["*"]).is_err());
        assert!(OriginAllowList::new(["http://*:8731"])
            .unwrap()
            .allows(Some("http://*:8731"), false));
        assert!(!OriginAllowList::new(["http://*:8731"])
            .unwrap()
            .allows(Some("http://a:8731"), false));
    }

    #[test]
    fn no_origin_is_accepted_from_loopback_only() {
        assert!(list().allows(None, true));
        assert!(!list().allows(None, false));
    }

    #[test]
    fn ipv6_origins_compare_on_their_port() {
        let v6 = OriginAllowList::new(["http://[::1]:8731"]).unwrap();
        assert!(v6.allows(Some("http://[::1]:8731"), false));
        assert!(!v6.allows(Some("http://[::1]:8732"), false));
    }
}
