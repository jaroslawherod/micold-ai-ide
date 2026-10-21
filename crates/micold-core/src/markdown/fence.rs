//! A code fence's info string → the tag shown and the grammar token highlighted (research R4).

/// The info string's first word, as written: cut at whitespace, `,` or `{`.
pub(super) fn first_word(info: &str) -> &str {
    let info = info.trim_start();
    let end = info
        .find(|c: char| c.is_whitespace() || c == ',' || c == '{')
        .unwrap_or(info.len());
    &info[..end]
}

/// The grammar token for a fence's tag: its first word, lower-cased, through the alias table;
/// `None` for an empty tag (research R4).
pub fn fence_token(tag: &str) -> Option<String> {
    let word = first_word(tag);
    if word.is_empty() {
        return None;
    }
    let lower = word.to_lowercase();
    let token = match lower.as_str() {
        "shell" | "bash" | "zsh" | "console" => "sh",
        "golang" => "go",
        "yml" => "yaml",
        "jsonc" => "json",
        "c++" => "cpp",
        other => other,
    };
    Some(token.to_string())
}

#[cfg(test)]
mod tests {
    use super::fence_token;

    fn token(tag: &str) -> Option<String> {
        fence_token(tag)
    }

    fn some(token: &str) -> Option<String> {
        Some(token.to_string())
    }

    #[test]
    fn the_first_word_is_cut_at_whitespace_a_comma_or_a_brace() {
        assert_eq!(token("rust ignore"), some("rust"));
        assert_eq!(token("rust,no_run"), some("rust"));
        assert_eq!(token("python{.line-numbers}"), some("python"));
        assert_eq!(token("  go\tmain"), some("go"));
    }

    #[test]
    fn the_token_is_lower_cased() {
        assert_eq!(token("Rust"), some("rust"));
        assert_eq!(token("TOML"), some("toml"));
    }

    #[test]
    fn aliases_map_to_their_grammar_token() {
        for shell in ["shell", "bash", "zsh", "console", "Bash"] {
            assert_eq!(token(shell), some("sh"), "{shell}");
        }
        assert_eq!(token("golang"), some("go"));
        assert_eq!(token("yml"), some("yaml"));
        assert_eq!(token("jsonc"), some("json"));
        assert_eq!(token("c++"), some("cpp"));
    }

    #[test]
    fn extensions_and_names_pass_through() {
        assert_eq!(token("ts"), some("ts"));
        assert_eq!(token("rs"), some("rs"));
        assert_eq!(token("py"), some("py"));
        assert_eq!(token("teapot-lang"), some("teapot-lang"));
    }

    #[test]
    fn an_empty_or_blank_tag_has_no_token() {
        assert_eq!(token(""), None);
        assert_eq!(token("   "), None);
        assert_eq!(token("\t\n"), None);
    }
}
