//! Cutting the CLI's stdout into lines (research R4).

/// Accumulates bytes and yields whole lines.
#[derive(Debug, Default)]
pub struct LineSplitter {
    buf: Vec<u8>,
}

impl LineSplitter {
    /// Add `bytes`; return every line they complete.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(bytes);
        Vec::new()
    }

    /// The trailing line with no newline, at end of input.
    pub fn finish(&mut self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines_of(chunks: &[&[u8]]) -> (Vec<String>, Option<String>) {
        let mut splitter = LineSplitter::default();
        let mut lines = Vec::new();
        for chunk in chunks {
            lines.extend(splitter.push(chunk));
        }
        (lines, splitter.finish())
    }

    #[test]
    fn a_line_split_across_two_chunks_comes_out_whole() {
        let (lines, rest) = lines_of(&[b"{\"type\":\"sys", b"tem\"}\n"]);
        assert_eq!(
            lines,
            vec!["{\"type\":\"system\"}".to_string()],
            "a read boundary inside a line does not cut it (R4)"
        );
        assert_eq!(rest, None, "nothing is left after a terminated line");
    }

    #[test]
    fn one_trailing_carriage_return_is_removed_and_a_second_is_kept() {
        let (lines, _) = lines_of(&[b"crlf\r\n", b"two\r\r\n"]);
        assert_eq!(
            lines,
            vec!["crlf".to_string(), "two\r".to_string()],
            "a CRLF line ending loses its \\r, and only one \\r is the line ending's (R4)"
        );
    }

    #[test]
    fn an_eight_mebibyte_line_comes_out_whole_and_unsplit() {
        const EIGHT_MIB: usize = 8 * 1024 * 1024;
        let long = "x".repeat(EIGHT_MIB);
        let mut splitter = LineSplitter::default();
        let mut lines = Vec::new();
        for chunk in long.as_bytes().chunks(64 * 1024) {
            lines.extend(splitter.push(chunk));
        }
        lines.extend(splitter.push(b"\n"));
        assert_eq!(lines.len(), 1, "a long line is never split (R4)");
        assert_eq!(lines[0].len(), EIGHT_MIB, "a long line is never truncated (R4)");
    }

    #[test]
    fn finish_yields_a_trailing_partial_line() {
        let (lines, rest) = lines_of(&[b"one\n", b"{\"type\":\"res"]);
        assert_eq!(lines, vec!["one".to_string()]);
        assert_eq!(
            rest,
            Some("{\"type\":\"res".to_string()),
            "the unterminated last line is kept at end of input (FR-014)"
        );
    }

    #[test]
    fn finish_yields_nothing_when_the_buffer_is_empty() {
        let (_, rest) = lines_of(&[b"one\n"]);
        assert_eq!(rest, None, "no line is invented at end of input");
    }

    #[test]
    fn invalid_utf8_is_decoded_lossily() {
        let (lines, _) = lines_of(&[b"bad \xff byte\n"]);
        assert_eq!(
            lines,
            vec!["bad \u{FFFD} byte".to_string()],
            "a line that is not UTF-8 is kept, not dropped (R4, FR-014)"
        );
    }

    #[test]
    fn an_empty_line_between_two_lines_is_yielded() {
        let (lines, _) = lines_of(&[b"a\n\nb\n"]);
        assert_eq!(
            lines,
            vec!["a".to_string(), String::new(), "b".to_string()],
            "every line becomes one event, an empty one too (FR-014)"
        );
    }
}
