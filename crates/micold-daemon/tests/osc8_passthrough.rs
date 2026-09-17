//! A hyperlink a program declares with OSC 8 reaches the grid through the real PTY (feature 031,
//! U72, research R13).
//!
//! The client marks and opens a declared link from the cell's `hyperlink`, so the claim that
//! matters is that the sequence survives the platform's pseudo-terminal. On Windows that is ConPTY,
//! whose passthrough of OSC 8 depends on the build, which is why this runs on every CI OS rather
//! than being assumed.
//!
//! The child is this test binary, re-executed with [`CHILD`] set, so no shell or `printf` of a
//! particular platform is involved: it prints one link and exits.

use std::io::Write;
use std::time::{Duration, Instant};

use micold_core::session::SessionId;
use micold_daemon::framer::Framer;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;

/// Set on the re-executed child: print the link instead of testing.
const CHILD: &str = "MICOLD_OSC8_PASSTHROUGH_CHILD";
const URI: &str = "https://example.com/manual";
const TEXT: &str = "docs";
const TEST: &str = "an_osc8_link_printed_through_the_pty_reaches_the_grid_cell";

#[test]
fn an_osc8_link_printed_through_the_pty_reaches_the_grid_cell() {
    if std::env::var_os(CHILD).is_some() {
        let mut out = std::io::stdout();
        write!(out, "\x1b]8;;{URI}\x1b\\{TEXT}\x1b]8;;\x1b\\\r\n").unwrap();
        out.flush().unwrap();
        // Stay long enough for the reader to see the output before the PTY closes.
        std::thread::sleep(Duration::from_secs(2));
        return;
    }

    let mut cmd = CommandBuilder::new(std::env::current_exe().expect("the test binary"));
    cmd.args([TEST, "--exact", "--nocapture", "--test-threads=1"]);
    cmd.env(CHILD, "1");
    let session = PtySession::spawn(SessionId::new(), cmd, 1000, Some((80, 24)))
        .expect("spawn the child through the PTY supervisor");

    let mut framer = Framer::new(session.id());
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut declared: Vec<(String, String)> = Vec::new();
    while Instant::now() < deadline {
        let frame = framer.frame(session.term(), true, None);
        declared = frame
            .lines
            .iter()
            .flat_map(|line| {
                let chars: Vec<char> = line.text.chars().collect();
                let hyperlinks = &frame.hyperlinks;
                line.extras.iter().filter_map(move |extra| {
                    let uri = hyperlinks.get(extra.hyperlink? as usize)?;
                    Some((
                        chars
                            .get(extra.col as usize)
                            .map(|c| c.to_string())
                            .unwrap_or_default(),
                        uri.clone(),
                    ))
                })
            })
            .collect();
        if !declared.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = session.kill();

    let text: String = declared.iter().map(|(c, _)| c.as_str()).collect();
    assert_eq!(
        text, TEXT,
        "the declared run's cells are the text the child printed: {declared:?}"
    );
    assert!(
        declared.iter().all(|(_, uri)| uri == URI),
        "every cell of the run carries the declared URI (FR-002): {declared:?}"
    );
}
