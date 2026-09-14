//! Which kind of address a link carries (research R8; contract link-recognition §4).

/// A link's address, classified by its scheme (FR-011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Address {
    /// `http` or `https`, verbatim.
    Web(String),
    /// `mailto`, verbatim.
    Mail(String),
    /// Anything this feature never opens.
    NotFollowable,
}

/// What kind of address `uri` is (FR-011).
pub fn classify(uri: &str) -> Address {
    if uri.starts_with("http://") || uri.starts_with("https://") {
        Address::Web(uri.to_string())
    } else if uri.starts_with("mailto:") {
        Address::Mail(uri.to_string())
    } else {
        Address::NotFollowable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_and_mail_addresses_classify_verbatim() {
        for web in ["https://a.example/x?y=1#z", "http://localhost:8080/"] {
            assert_eq!(
                classify(web),
                Address::Web(web.to_string()),
                "{web} is a web address, passed on exactly as written"
            );
        }
        assert_eq!(
            classify("mailto:team@example.com?subject=Hi%20there"),
            Address::Mail("mailto:team@example.com?subject=Hi%20there".to_string()),
            "a mail address is passed on exactly as written, escapes included"
        );
    }
}
