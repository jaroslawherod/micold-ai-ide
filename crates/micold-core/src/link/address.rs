//! Which kind of address a link carries (research R8; contract link-recognition §4).

/// A link's address, classified by its scheme (FR-011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Address {
    /// `http` or `https`, verbatim.
    Web(String),
    /// `mailto`, verbatim.
    Mail(String),
    /// A `file://host/path` address, with `path` percent-decoded (FR-012, C9).
    ///
    /// `host` is empty when the address named none (`file:///…`). Whether the host is *this*
    /// machine, and what the path means here, is `resolve`'s question.
    File { host: String, path: String },
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
    } else if starts_with("file://") {
        file(&uri["file://".len()..])
    } else {
        Address::NotFollowable
    }
}

/// `host/path` after a `file://` scheme (C4–C10).
///
/// The path keeps its leading `/`, as every caller reads it as absolute. A path that cannot be
/// decoded is no address at all, rather than one pointing somewhere the escape did not name.
fn file(rest: &str) -> Address {
    let Some(slash) = rest.find('/') else {
        return Address::NotFollowable;
    };
    let (host, path) = rest.split_at(slash);
    match percent_decode(path) {
        Some(path) => Address::File {
            host: host.to_string(),
            path,
        },
        None => Address::NotFollowable,
    }
}

/// `text` with each `%XX` turned into its byte, or `None` for an invalid escape, a result that is
/// not UTF-8, or one holding a control character (C10).
///
/// Both digits must be hex digits of their own: an integer parser also reads a sign, which would
/// turn `%+A` into a newline. A control character is no part of a path, and a decoded newline in a
/// path would forge a second line in the hover hint.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        let hex = text.get(i + 1..i + 3)?;
        if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        out.push(u8::from_str_radix(hex, 16).ok()?);
        i += 3;
    }
    let decoded = String::from_utf8(out).ok()?;
    (!decoded.chars().any(char::is_control)).then_some(decoded)
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

    /// U31: C9 — the host and the path, with the escapes a program prints decoded.
    #[test]
    fn a_file_address_carries_its_host_and_its_decoded_path() {
        assert_eq!(
            classify("file:///home/u/a.txt"),
            Address::File {
                host: String::new(),
                path: "/home/u/a.txt".to_string(),
            },
            "an empty host is this machine (C4)"
        );
        assert_eq!(
            classify("file://build.example.com/p/My%20Doc.pdf"),
            Address::File {
                host: "build.example.com".to_string(),
                path: "/p/My Doc.pdf".to_string(),
            },
            "the host is kept as written and %20 is decoded to a space (C5, C9)"
        );
        assert_eq!(
            classify("FILE://localhost/tmp/x"),
            Address::File {
                host: "localhost".to_string(),
                path: "/tmp/x".to_string(),
            },
            "the file scheme is matched whatever its case"
        );
        assert_eq!(
            classify("file://host"),
            Address::NotFollowable,
            "a file address with no path is no file (FR-001)"
        );
    }

    /// U32: C10 — a guard. Its red is the mutant in tdd/test-list.md: a decoder that passes an
    /// invalid escape through literally.
    #[test]
    fn a_file_path_that_cannot_be_decoded_is_not_followable() {
        for uri in [
            "file:///p/%ZZ",
            "file:///p/%2",
            "file:///p/%",
            // `%FF` alone is not UTF-8.
            "file:///p/%FF",
            // A sign is not a hex digit, however willingly an integer parser reads one (review A
            // finding 2).
            "file:///p/%+A",
            "file:///p/%-1",
            // A control character is no part of a path, and a decoded newline would forge a second
            // line in the hover hint (review A finding 2).
            "file:///tmp/%00x",
            "file:///tmp/a%0Ab",
        ] {
            assert_eq!(
                classify(uri),
                Address::NotFollowable,
                "{uri} cannot be decoded into a path, so it opens nothing (C10)"
            );
        }
        assert_eq!(
            classify("file:///p/%C3%A9.txt"),
            Address::File {
                host: String::new(),
                path: "/p/é.txt".to_string(),
            },
            "a well-formed multi-byte escape decodes to its character"
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
