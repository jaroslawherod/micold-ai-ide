//! `Lifts`: how far each Overview card's shadow has eased in (feature 1299, D3, research R3).
//!
//! The grid owns the expanded card's shadow, so it can keep a card that is settling back drawn
//! over its neighbours while the shadow fades: a shadow faded in the card's own tree would be
//! painted over by every card drawn after it. One track per card that is lifted or settling,
//! keyed by its location so a re-sort mid-transition keeps each shadow on its card; a card at
//! rest with no shadow has no entry.

use crate::ui::cdk::motion::Progress;
use crate::ui::material::overview_card::CARD_MOTION;
use iced::advanced::Shell;
use iced::Event;
use micold_core::session::SessionLocation;
use micold_core::tokens::motion::STANDARD;

/// The lifted and settling cards' shadow strengths. See the module documentation.
#[derive(Debug, Default)]
pub(super) struct Lifts(Vec<(SessionLocation, Progress)>);

/// A track resting at `at`, on the standard curve (R5).
fn track(at: f32) -> Progress {
    Progress::new(at).easing(STANDARD.x1, STANDARD.y1, STANDARD.x2, STANDARD.y2)
}

impl Lifts {
    /// The lifts of a grid whose state is created with `expanded` already expanded: that card
    /// starts lifted, so nothing animates into existence (R8).
    pub(super) fn seeded(expanded: Option<&SessionLocation>) -> Self {
        Self(
            expanded
                .map(|at| (at.clone(), track(1.0)))
                .into_iter()
                .collect(),
        )
    }

    /// Step every track on `event`: the `expanded` card's toward full, every other toward none,
    /// over [`CARD_MOTION`], or at once when `instant` (FR-007). A newly expanded card gets a
    /// track from none, a card no longer `present` loses its track, and a track that has settled
    /// to none is dropped, so a grid at rest asks for no frame (FR-009).
    pub(super) fn step<M>(
        &mut self,
        event: &Event,
        expanded: Option<&SessionLocation>,
        present: impl Fn(&SessionLocation) -> bool,
        instant: bool,
        shell: &mut Shell<'_, M>,
    ) {
        if let Some(at) = expanded {
            if !self.0.iter().any(|(loc, _)| loc == at) {
                self.0.push((at.clone(), track(0.0)));
            }
        }
        self.0.retain(|(at, _)| present(at));
        for (at, progress) in &mut self.0 {
            let target = if Some(&*at) == expanded { 1.0 } else { 0.0 };
            if instant {
                progress.restart_at(target);
            } else {
                progress.on_frame(event, target, CARD_MOTION, shell);
            }
        }
        self.0
            .retain(|(at, progress)| Some(at) == expanded || progress.value() > 0.0);
    }

    /// How much of `at`'s shadow to draw: its target when `instant` (full when `expanded`), else
    /// its track's value, else none (expanded but not stepped yet: it eases in from none).
    pub(super) fn strength(&self, at: &SessionLocation, expanded: bool, instant: bool) -> f32 {
        if instant {
            return if expanded { 1.0 } else { 0.0 };
        }
        self.0
            .iter()
            .find(|(loc, _)| loc == at)
            .map_or(0.0, |(_, progress)| progress.value())
    }

    /// The cards whose shadow is fading out: a track above none that is not the `expanded` card.
    pub(super) fn settling<'s>(
        &'s self,
        expanded: Option<&'s SessionLocation>,
    ) -> impl Iterator<Item = &'s SessionLocation> + 's {
        self.0
            .iter()
            .filter(move |(at, progress)| Some(at) != expanded && progress.value() > 0.0)
            .map(|(at, _)| at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::test_support::{DONE, FRAME_MS, MIDWAY};
    use iced::window::{self, RedrawRequest};
    use std::time::{Duration, Instant};

    fn loc(name: &str) -> SessionLocation {
        SessionLocation::Worktree(name.to_string())
    }

    /// A rig stepping `Lifts` on frame ticks [`FRAME_MS`] apart from `origin`.
    struct Rig {
        lifts: Lifts,
        origin: Instant,
        present: Vec<SessionLocation>,
    }

    impl Rig {
        fn new(lifts: Lifts) -> Self {
            Self {
                lifts,
                origin: Instant::now(),
                present: vec![loc("a"), loc("b"), loc("c")],
            }
        }

        /// One step on `event`: the frame asked for next.
        fn step_on(
            &mut self,
            event: &Event,
            expanded: Option<&SessionLocation>,
            instant: bool,
        ) -> RedrawRequest {
            let mut messages: Vec<()> = Vec::new();
            let mut shell = Shell::new(&mut messages);
            let present = self.present.clone();
            self.lifts.step(
                event,
                expanded,
                |at| present.contains(at),
                instant,
                &mut shell,
            );
            shell.redraw_request()
        }

        /// Ticks from `from` to `until` ms inclusive: each tick's time, the strengths of `read`
        /// after it and the frame asked for.
        fn frames(
            &mut self,
            from: u64,
            until: u64,
            expanded: Option<&SessionLocation>,
            read: &[&SessionLocation],
        ) -> Vec<(u64, Vec<f32>, RedrawRequest)> {
            (from..=until)
                .step_by(FRAME_MS as usize)
                .map(|ms| {
                    let now = self.origin + Duration::from_millis(ms);
                    let event = Event::Window(window::Event::RedrawRequested(now));
                    let asked = self.step_on(&event, expanded, false);
                    let s = read
                        .iter()
                        .map(|at| self.lifts.strength(at, Some(*at) == expanded, false))
                        .collect();
                    (ms, s, asked)
                })
                .collect()
        }
    }

    fn at_tick(ticks: &[(u64, Vec<f32>, RedrawRequest)], ms: u64) -> &[f32] {
        &ticks.iter().find(|(t, ..)| *t == ms).expect("a tick").1
    }

    /// SC-001, FR-003, FR-009: an expanded card's lift is part-way at ~75 ms and full by 150 ms
    /// and a frame; collapsed, it falls back the same way and its entry is dropped at rest, and
    /// the tick after it rests asks for nothing.
    #[test]
    fn a_lift_eases_in_and_out_and_is_dropped_at_rest() {
        let a = loc("a");
        let mut rig = Rig::new(Lifts::default());
        assert_eq!(rig.lifts.strength(&a, true, false), 0.0, "not stepped yet");
        let up = rig.frames(0, DONE, Some(&a), &[&a]);
        let mid = at_tick(&up, MIDWAY)[0];
        assert!(0.0 < mid && mid < 1.0, "at ~75 ms the lift is {mid}");
        assert_eq!(at_tick(&up, DONE)[0], 1.0);
        let rest = rig.frames(DONE + FRAME_MS, DONE + FRAME_MS, Some(&a), &[&a]);
        assert_eq!(rest[0].2, RedrawRequest::Wait);
        let start = DONE + 100;
        let down = rig.frames(start, start + DONE, None, &[&a]);
        let mid = at_tick(&down, start + MIDWAY)[0];
        assert!(0.0 < mid && mid < 1.0, "at ~75 ms out the lift is {mid}");
        assert_eq!(at_tick(&down, start + DONE)[0], 0.0);
        assert!(rig.lifts.0.is_empty(), "the settled entry is dropped");
        let after = rig.frames(
            start + DONE + FRAME_MS,
            start + DONE + FRAME_MS,
            None,
            &[&a],
        );
        assert_eq!(after[0].2, RedrawRequest::Wait);
    }

    /// FR-004: a collapse at ~75 ms turns back from where the lift was and ends within 150 ms and
    /// a frame of it.
    #[test]
    fn a_reversed_lift_continues_from_its_value() {
        let a = loc("a");
        let mut rig = Rig::new(Lifts::default());
        let up = rig.frames(0, MIDWAY, Some(&a), &[&a]);
        let reached = at_tick(&up, MIDWAY)[0];
        let start = MIDWAY + FRAME_MS;
        let down = rig.frames(start, start + DONE, None, &[&a]);
        let next = down[0].1[0];
        assert!(
            next < reached && reached - next < 0.3,
            "{reached} -> {next}"
        );
        assert_eq!(at_tick(&down, start + DONE)[0], 0.0);
    }

    /// FR-005, US2-3: A collapsing while B expands: A falls and B rises on the same ticks, and
    /// only A is settling.
    #[test]
    fn two_lifts_run_independently() {
        let (a, b) = (loc("a"), loc("b"));
        let mut rig = Rig::new(Lifts::seeded(Some(&a)));
        let ticks = rig.frames(0, MIDWAY, Some(&b), &[&a, &b]);
        for pair in ticks.windows(2) {
            let (was, now) = (&pair[0].1, &pair[1].1);
            assert!(now[0] < was[0] && now[1] > was[1], "{was:?} -> {now:?}");
        }
        let settling: Vec<_> = rig.lifts.settling(Some(&b)).collect();
        assert_eq!(settling, [&a]);
    }

    /// R8: a seeded card is lifted at once, and asks for no frame.
    #[test]
    fn a_seeded_lift_is_full_with_no_frame_asked() {
        let a = loc("a");
        let mut rig = Rig::new(Lifts::seeded(Some(&a)));
        assert_eq!(rig.lifts.strength(&a, true, false), 1.0);
        let first = rig.frames(0, 0, Some(&a), &[&a]);
        assert_eq!(first[0].1[0], 1.0);
        assert_eq!(first[0].2, RedrawRequest::Wait);
    }

    /// Edge Cases: a card removed mid-transition loses its lift.
    #[test]
    fn a_location_no_longer_present_is_dropped() {
        let a = loc("a");
        let mut rig = Rig::new(Lifts::default());
        rig.frames(0, MIDWAY, Some(&a), &[&a]);
        rig.present.retain(|at| at != &a);
        rig.frames(MIDWAY + FRAME_MS, MIDWAY + FRAME_MS, None, &[]);
        assert!(rig.lifts.0.is_empty());
        assert_eq!(rig.lifts.strength(&a, false, false), 0.0);
    }

    /// FR-007: with `instant` every lift is at its target on the first step, asking for nothing.
    #[test]
    fn instant_lifts_reach_their_targets_on_the_first_step() {
        let (a, b) = (loc("a"), loc("b"));
        let mut rig = Rig::new(Lifts::seeded(Some(&a)));
        let tick = Event::Window(window::Event::RedrawRequested(rig.origin));
        assert_eq!(rig.step_on(&tick, Some(&b), true), RedrawRequest::Wait);
        assert_eq!(rig.lifts.strength(&b, true, false), 1.0);
        assert_eq!(rig.lifts.strength(&a, false, false), 0.0);
        assert_eq!(rig.lifts.settling(Some(&b)).count(), 0);
        assert_eq!(
            rig.lifts.strength(&a, true, true),
            1.0,
            "instant reads the target"
        );
        assert_eq!(
            rig.lifts.strength(&b, false, true),
            0.0,
            "instant reads the target"
        );
    }
}
