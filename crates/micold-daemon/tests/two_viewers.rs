//! Issue 973 -- two clients viewing one terminal each get every change. The framer is shared, so a
//! delta computed for one viewer must not be spent: the other still holds the old screen.

#[allow(dead_code)] // shared helpers; this test uses only some
mod support;

use micold_core::session::SessionId;
use micold_daemon::framer::{Framer, Viewer};
use support::DrivenTerm;

fn text(frame: &micold_core::protocol::grid::GridFrame) -> String {
    frame
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_change_framed_for_one_viewer_is_still_framed_for_the_other() {
    let mut vt = DrivenTerm::new(80, 24, 1_000);
    let mut framer = Framer::new(SessionId::new());
    let (mut first, mut second) = (Viewer::default(), Viewer::default());
    framer.frame_for(&mut first, &vt.term, true, None);
    framer.frame_for(&mut second, &vt.term, true, None);

    vt.feed("GOT-hello\r\n");

    let for_first = framer.frame_for(&mut first, &vt.term, false, None);
    let for_second = framer.frame_for(&mut second, &vt.term, false, None);
    assert!(text(&for_first).contains("GOT-hello"), "{for_first:?}");
    assert!(text(&for_second).contains("GOT-hello"), "{for_second:?}");
}

#[test]
fn a_viewer_that_missed_a_resize_gets_a_full_frame() {
    let vt = DrivenTerm::new(80, 24, 1_000);
    let mut framer = Framer::new(SessionId::new());
    let (mut first, mut second) = (Viewer::default(), Viewer::default());
    framer.frame_for(&mut first, &vt.term, true, None);
    framer.frame_for(&mut second, &vt.term, true, None);

    vt.term
        .lock()
        .resize(alacritty_terminal::term::test::TermSize::new(100, 30));

    let for_first = framer.frame_for(&mut first, &vt.term, false, None);
    let for_second = framer.frame_for(&mut second, &vt.term, false, None);
    assert!(for_first.full && for_second.full);
    assert_eq!(for_first.generation, for_second.generation);
}
