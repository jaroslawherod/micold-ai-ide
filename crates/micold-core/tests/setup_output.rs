//! Feature 1289, A4 (output): a setup run's output, split into lines as a terminal shows them and
//! capped at the last `KEEP_LINES` lines (FR-021).

use micold_core::setup::output::{OutputBuffer, Stream, KEEP_LINES};

fn lines(done: Vec<(u64, String)>) -> Vec<String> {
    done.into_iter().map(|(_, line)| line).collect()
}

#[test]
fn a_push_returns_the_lines_it_completed_with_their_indices() {
    let mut buffer = OutputBuffer::new();
    assert_eq!(
        buffer.push(Stream::Stdout, b"one\ntwo\nthr"),
        vec![(0, "one".to_owned()), (1, "two".to_owned())]
    );
    assert_eq!(
        buffer.push(Stream::Stdout, b"ee\n"),
        vec![(2, "three".to_owned())],
        "a line split across two pushes is joined"
    );
}

#[test]
fn crlf_and_lf_both_end_a_line() {
    let mut buffer = OutputBuffer::new();
    assert_eq!(lines(buffer.push(Stream::Stdout, b"a\r\nb\n")), ["a", "b"]);
    // A `\r\n` split across two pushes is still one line end.
    assert_eq!(
        lines(buffer.push(Stream::Stdout, b"c\r")),
        Vec::<String>::new()
    );
    assert_eq!(lines(buffer.push(Stream::Stdout, b"\nd\n")), ["c", "d"]);
}

#[test]
fn a_lone_carriage_return_clears_the_partial_line() {
    let mut buffer = OutputBuffer::new();
    assert_eq!(
        lines(buffer.push(Stream::Stdout, b"10%\r50%\r100%\n")),
        ["100%"],
        "a progress bar keeps only its last state"
    );
    assert_eq!(lines(buffer.push(Stream::Stdout, b"abc\rxy\n")), ["xy"]);
}

#[test]
fn stdout_and_stderr_partials_are_kept_apart() {
    let mut buffer = OutputBuffer::new();
    assert!(buffer.push(Stream::Stdout, b"out-").is_empty());
    assert_eq!(lines(buffer.push(Stream::Stderr, b"err\n")), ["err"]);
    assert_eq!(lines(buffer.push(Stream::Stdout, b"line\n")), ["out-line"]);
}

#[test]
fn ansi_csi_and_osc_sequences_are_removed() {
    let mut buffer = OutputBuffer::new();
    assert_eq!(
        lines(buffer.push(
            Stream::Stdout,
            b"\x1b]0;title\x07\x1b[31mred\x1b[0m \x1b[1;32mgreen\x1b[m\n"
        )),
        ["red green"]
    );
    // An OSC ended by ST (ESC \), and a CSI split across two pushes.
    assert!(buffer
        .push(Stream::Stdout, b"\x1b]8;;http://x\x1b\\link\x1b[3")
        .is_empty());
    assert_eq!(
        lines(buffer.push(Stream::Stdout, b"4mblue\n")),
        ["linkblue"]
    );
}

#[test]
fn invalid_utf8_is_decoded_lossily() {
    let mut buffer = OutputBuffer::new();
    assert_eq!(
        lines(buffer.push(Stream::Stdout, b"caf\xc3\xa9 \xff!\n")),
        ["café \u{fffd}!"]
    );
}

#[test]
fn finish_flushes_each_partial_line() {
    let mut buffer = OutputBuffer::new();
    buffer.push(Stream::Stdout, b"no newline");
    buffer.push(Stream::Stderr, b"nor here");
    assert_eq!(
        buffer.finish(),
        vec![(0, "no newline".to_owned()), (1, "nor here".to_owned())]
    );
    assert!(buffer.finish().is_empty(), "nothing is left to flush");
    let (first, backlog) = buffer.since(0);
    assert_eq!(
        (first, backlog),
        (0, vec!["no newline".to_owned(), "nor here".to_owned()])
    );
}

#[test]
fn past_keep_lines_the_oldest_line_goes_and_is_counted() {
    assert_eq!(KEEP_LINES, 10_000, "FR-021: at least the last 10,000 lines");
    let mut buffer = OutputBuffer::new();
    let total = KEEP_LINES as u64 + 3;
    for i in 0..total {
        let done = buffer.push(Stream::Stdout, format!("line {i}\n").as_bytes());
        assert_eq!(
            done,
            vec![(i, format!("line {i}"))],
            "indices keep counting"
        );
    }
    assert_eq!(buffer.first_index(), 3, "three lines were dropped");
    assert_eq!(buffer.next_index(), total);

    let (first, backlog) = buffer.since(0);
    assert_eq!(first, 3, "the backlog starts at the oldest line kept");
    assert_eq!(backlog.len(), KEEP_LINES);
    assert_eq!(backlog[0], "line 3");

    let (first, backlog) = buffer.since(total - 2);
    assert_eq!(first, total - 2);
    assert_eq!(
        backlog,
        vec![format!("line {}", total - 2), format!("line {}", total - 1)]
    );

    let (first, backlog) = buffer.since(total + 5);
    assert_eq!((first, backlog.len()), (total, 0), "nothing past the end");
}
