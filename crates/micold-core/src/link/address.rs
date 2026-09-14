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
    let starts_with = |scheme: &str| {
        uri.get(..scheme.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(scheme))
    };
    if starts_with("http://") || starts_with("https://") {
        Address::Web(uri.to_string())
    } else if starts_with("mailto:") {
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

    #[test]
    fn the_scheme_classifies_whatever_its_case() {
        assert_eq!(
            classify("HTTPS://A.EXAMPLE/X"),
            Address::Web("HTTPS://A.EXAMPLE/X".to_string()),
            "an upper-case web scheme is still a web address, and keeps its case"
        );
        assert_eq!(
            classify("Http://a.example"),
            Address::Web("Http://a.example".to_string()),
            "a mixed-case web scheme is a web address"
        );
        assert_eq!(
            classify("MailTo:team@example.com"),
            Address::Mail("MailTo:team@example.com".to_string()),
            "a mixed-case mail scheme is a mail address"
        );
    }

    #[test]
    fn application_script_data_and_unknown_schemes_are_not_followable() {
        for uri in [
            "vscode://file/home/u/a.rs",
            "slack://channel?team=T1&id=C1",
            "zoommtg://zoom.us/join?confno=1",
            "javascript:alert(1)",
            "data:text/html;base64,PHNjcmlwdD4=",
            "vbscript:msgbox(1)",
            "gopher://a.example/",
        ] {
            assert_eq!(
                classify(uri),
                Address::NotFollowable,
                "{uri} opens an application, runs code, carries its own content or is unknown"
            );
        }
    }
}
