//! Frame-time statistics (feature 018, T000z/T076a — FR-039b, FR-039c, SC-018).
//!
//! SC-018 asks for three frame-time figures recorded on the same machine, and until now nothing in
//! this workspace could produce one. This is the pure half: the accumulator that turns a stream of
//! per-frame durations into the summary that gets written down. It carries all the decision logic —
//! which samples count, how the percentile is defined, what an empty run reports — so Principle I
//! covers it here rather than leaving it in rendering glue where no test can reach it.
//!
//! What it deliberately does *not* do is decide when a frame happens. That is the client's job.

use std::time::Duration;

use micold_core::frame_probe::{
    FrameProbe, ProbeConfig, Scene, SceneFacts, DEFAULT_WARM_UP, REFERENCE_WORKTREES,
};

/// Milliseconds, spelled out, because a bare `Duration::from_millis` at every call site buries the
/// numbers the assertions are actually about.
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

// ---------------------------------------------------------------------------------------------
// Warm-up: the first frames are not representative and must not be counted
// ---------------------------------------------------------------------------------------------

/// The first frame after a scene is composed pays for pipeline and glyph-cache warm-up that no
/// later frame pays. Averaging it in would make a fast build look slow, and would do so
/// *inconsistently* between two runs — which is the one thing a trend figure cannot afford.
#[test]
fn warm_up_samples_are_discarded() {
    let mut probe = FrameProbe::new(2);

    probe.record(ms(90)); // warm-up, discarded
    probe.record(ms(80)); // warm-up, discarded
    probe.record(ms(4));
    probe.record(ms(6));

    let summary = probe.summary().expect("two counted samples");
    assert_eq!(summary.frames, 2, "warm-up samples must not be counted");
    assert_eq!(summary.mean, ms(5));
    assert_eq!(
        summary.max,
        ms(6),
        "the discarded 90ms frame must not become the max"
    );
}

/// A probe that has seen only warm-up has no data, and must say so rather than reporting zeros.
#[test]
fn a_probe_that_has_only_warmed_up_has_no_summary() {
    let mut probe = FrameProbe::new(2);
    probe.record(ms(90));
    probe.record(ms(80));

    assert!(probe.summary().is_none());
}

/// Zero warm-up is legitimate — a caller measuring an already-warm scene should not be forced to
/// throw away good samples.
#[test]
fn zero_warm_up_counts_every_sample() {
    let mut probe = FrameProbe::new(0);
    probe.record(ms(4));

    let summary = probe.summary().expect("one sample");
    assert_eq!(summary.frames, 1);
    assert_eq!(summary.mean, ms(4));
}

// ---------------------------------------------------------------------------------------------
// The empty case
// ---------------------------------------------------------------------------------------------

/// An empty probe reports nothing at all. It must not report a mean of zero: a zero frame time is
/// a *result*, and a run that collected no data is not one. Recording "0.00 ms" in §B8 would be
/// worse than recording nothing, because it looks like a measurement.
#[test]
fn an_empty_probe_reports_no_summary() {
    let probe = FrameProbe::new(0);
    assert!(probe.summary().is_none());
    assert_eq!(probe.counted(), 0);
}

// ---------------------------------------------------------------------------------------------
// The statistics themselves
// ---------------------------------------------------------------------------------------------

/// One sample is its own mean, p95 and max. Degenerate, and worth pinning: percentile code that
/// indexes `n - 1` or `n * 0.95` without care panics or underflows exactly here.
#[test]
fn a_single_sample_is_its_own_mean_p95_and_max() {
    let mut probe = FrameProbe::new(0);
    probe.record(ms(7));

    let s = probe.summary().expect("one sample");
    assert_eq!(s.frames, 1);
    assert_eq!(s.mean, ms(7));
    assert_eq!(s.p95, ms(7));
    assert_eq!(s.max, ms(7));
}

/// The percentile is **nearest-rank on the sorted samples**: `ceil(0.95 * n)`, 1-indexed. Pinned
/// with a hand-checkable set rather than described, because "p95" names a family of definitions
/// that disagree, and two builds measured under different definitions are not comparable.
///
/// With n = 20, ceil(0.95 * 20) = 19, so p95 is the 19th smallest — the second largest.
#[test]
fn p95_is_the_nearest_rank_sample() {
    let mut probe = FrameProbe::new(0);
    for i in 1..=20 {
        probe.record(ms(i));
    }

    let s = probe.summary().expect("twenty samples");
    assert_eq!(
        s.p95,
        ms(19),
        "nearest-rank p95 of 1..=20 is the 19th smallest"
    );
    assert_eq!(s.max, ms(20));
}

/// With n = 10, ceil(0.95 * 10) = 10 — p95 and max coincide. Small runs cannot distinguish a tail
/// from a peak, and the summary must not pretend otherwise.
#[test]
fn p95_meets_max_on_small_runs() {
    let mut probe = FrameProbe::new(0);
    for i in 1..=10 {
        probe.record(ms(i));
    }

    let s = probe.summary().expect("ten samples");
    assert_eq!(s.p95, s.max);
}

/// Arrival order must not change the summary. The probe is fed by a render loop whose ordering is
/// incidental; two runs that saw the same frames in a different order are the same measurement.
#[test]
fn the_summary_does_not_depend_on_arrival_order() {
    let ascending = {
        let mut p = FrameProbe::new(0);
        for i in [1u64, 2, 3, 4, 40] {
            p.record(ms(i));
        }
        p.summary().expect("samples")
    };
    let shuffled = {
        let mut p = FrameProbe::new(0);
        for i in [40u64, 2, 4, 1, 3] {
            p.record(ms(i));
        }
        p.summary().expect("samples")
    };

    assert_eq!(ascending, shuffled);
}

/// The mean is the arithmetic mean over counted samples, and a long tail must actually move it —
/// a summary that silently clamped or dropped outliers would hide the regression it exists to
/// surface.
#[test]
fn the_mean_includes_the_tail() {
    let mut probe = FrameProbe::new(0);
    for _ in 0..9 {
        probe.record(ms(2));
    }
    probe.record(ms(20));

    let s = probe.summary().expect("ten samples");
    assert_eq!(
        s.mean,
        Duration::from_micros(3_800),
        "(9*2 + 20) / 10 = 3.8ms"
    );
}

/// Sub-millisecond precision survives. A fast scene renders in hundreds of microseconds, and a
/// summary rounded to whole milliseconds would report every such build as identical.
#[test]
fn sub_millisecond_samples_are_not_rounded_away() {
    let mut probe = FrameProbe::new(0);
    probe.record(Duration::from_micros(400));
    probe.record(Duration::from_micros(600));

    let s = probe.summary().expect("two samples");
    assert_eq!(s.mean, Duration::from_micros(500));
}

// ---------------------------------------------------------------------------------------------
// Enabling a run: what the environment says
// ---------------------------------------------------------------------------------------------
//
// The measurement mode drives the window at full rate and then exits the process. That is exactly
// what T000z needs and exactly what nobody wants to trigger by accident, so what does and does not
// enable it is decided here, under test, rather than by an `unwrap_or_default` at the call site.

/// The overwhelmingly common case: the variable is not set, so the application starts normally.
#[test]
fn an_unset_variable_does_not_enable_a_run() {
    assert_eq!(ProbeConfig::from_env_value(None), Ok(None));
}

/// `MICOLD_FRAME_PROBE=` — set but empty — is how a shell leaves a cleared variable, and reads as
/// "off" to anyone who writes it. It must not launch a run that quits the app a few seconds later.
#[test]
fn an_empty_variable_does_not_enable_a_run() {
    assert_eq!(ProbeConfig::from_env_value(Some("")), Ok(None));
    assert_eq!(ProbeConfig::from_env_value(Some("   ")), Ok(None));
}

/// A bare count is the ordinary invocation, and the warm-up defaults rather than being spelled out
/// at every call site.
#[test]
fn a_bare_count_is_that_many_counted_frames() {
    assert_eq!(
        ProbeConfig::from_env_value(Some("300")),
        Ok(Some(ProbeConfig {
            frames: 300,
            warm_up: DEFAULT_WARM_UP,
        }))
    );
}

/// `frames:warm_up` for when the default warm-up is wrong — a slower machine, or a scene that pays
/// a longer glyph-cache cost than the default allows for.
#[test]
fn a_count_and_a_warm_up_can_both_be_given() {
    assert_eq!(
        ProbeConfig::from_env_value(Some("600:120")),
        Ok(Some(ProbeConfig {
            frames: 600,
            warm_up: 120,
        }))
    );
}

/// Surrounding whitespace is forgiven — it survives shell quoting more often than it is meant.
#[test]
fn surrounding_whitespace_is_ignored() {
    assert_eq!(
        ProbeConfig::from_env_value(Some("  300 : 30  ")),
        Ok(Some(ProbeConfig {
            frames: 300,
            warm_up: 30,
        }))
    );
}

/// Zero warm-up is legitimate and must survive the parse — measuring an already-warm scene is a
/// real choice, and [`FrameProbe`] already supports it.
#[test]
fn an_explicit_zero_warm_up_is_accepted() {
    assert_eq!(
        ProbeConfig::from_env_value(Some("10:0")),
        Ok(Some(ProbeConfig {
            frames: 10,
            warm_up: 0,
        }))
    );
}

/// A malformed value is an error, never a silent "off".
///
/// This is the case that matters most. Someone typing `MICOLD_FRAME_PROBE=yes` and getting a normal
/// launch would conclude the probe does not work; worse, someone typing `30O` (letter O) during a
/// T000z capture would record an ordinary session as a measurement run. Loud beats convenient.
#[test]
fn a_malformed_value_is_an_error_not_a_silent_off() {
    for bad in ["yes", "on", "30O", "300:", ":30", "300:30:30", "-5", "3.5"] {
        assert!(
            ProbeConfig::from_env_value(Some(bad)).is_err(),
            "`{bad}` must be rejected, not quietly ignored"
        );
    }
}

/// Counting zero frames is a run that can only ever report nothing, so it is refused at the point
/// where a person can still fix the command rather than after the app has exited with no figure.
#[test]
fn a_zero_frame_count_is_refused() {
    assert!(ProbeConfig::from_env_value(Some("0")).is_err());
    assert!(ProbeConfig::from_env_value(Some("0:30")).is_err());
}

/// The rejection says what to type instead. A parse error that only says "invalid" makes the
/// grammar something you have to read the source to discover.
#[test]
fn the_rejection_names_the_expected_grammar() {
    let err = ProbeConfig::from_env_value(Some("yes")).expect_err("must be rejected");
    assert!(
        err.contains("yes"),
        "the message must quote what was actually given: {err}"
    );
    assert!(
        err.contains("MICOLD_FRAME_PROBE"),
        "the message must name the variable: {err}"
    );
}

// ---------------------------------------------------------------------------------------------
// Ending a run
// ---------------------------------------------------------------------------------------------

/// The run ends on **counted** frames, not on frames observed. Warm-up is not part of the quota —
/// otherwise the warm-up setting would silently shorten the measurement it exists to protect.
#[test]
fn a_run_completes_on_counted_frames_not_observed_ones() {
    let config = ProbeConfig {
        frames: 3,
        warm_up: 2,
    };
    let mut probe = config.probe();

    for _ in 0..2 {
        probe.record(ms(90)); // warm-up
        assert!(
            !config.is_complete(&probe),
            "warm-up must not count toward the quota"
        );
    }
    for _ in 0..2 {
        probe.record(ms(4));
        assert!(!config.is_complete(&probe));
    }

    probe.record(ms(4)); // the third counted frame
    assert!(config.is_complete(&probe));
}

/// The probe a config builds carries that config's warm-up. Trivial, and the one wiring mistake
/// that would silently fold warm-up frames into the figure.
#[test]
fn the_configured_probe_discards_the_configured_warm_up() {
    let config = ProbeConfig {
        frames: 10,
        warm_up: 2,
    };
    let mut probe = config.probe();

    probe.record(ms(90));
    probe.record(ms(80));
    probe.record(ms(4));

    assert_eq!(probe.counted(), 1);
}

// ---------------------------------------------------------------------------------------------
// Reporting the run
// ---------------------------------------------------------------------------------------------

/// The summary formats itself into the line that gets pasted into `quickstart.md` §B8. Formatting
/// lives here rather than at the print site so the recorded figure has one shape across all three
/// slots — three figures written to different precisions are not comparable at a glance, which is
/// the entire point of recording them together.
#[test]
fn the_report_line_carries_all_four_figures_in_milliseconds() {
    let mut probe = FrameProbe::new(0);
    probe.record(Duration::from_micros(3_420));
    probe.record(Duration::from_micros(5_100));

    let line = probe.summary().expect("samples").report_line();

    assert!(line.contains("2 frames"), "frame count missing: {line}");
    assert!(line.contains("4.26 ms"), "mean missing: {line}");
    assert!(line.contains("5.10 ms"), "p95 missing: {line}");
    assert!(line.contains("max"), "max not labelled: {line}");
}

/// Two decimal places, always — including on a whole number, where a bare `4 ms` would read as a
/// coarser measurement than it is, and on a fast scene, where truncating to whole milliseconds
/// would report every sub-millisecond build as `0 ms`.
#[test]
fn the_report_line_keeps_sub_millisecond_resolution() {
    let mut probe = FrameProbe::new(0);
    probe.record(Duration::from_micros(400));

    let line = probe.summary().expect("one sample").report_line();
    assert!(
        line.contains("0.40 ms"),
        "a fast frame must not round away to `0 ms`: {line}"
    );
}

// ---------------------------------------------------------------------------------------------
// The scene: which one, and whether it is actually the one on screen
// ---------------------------------------------------------------------------------------------
//
// SC-018 compares three figures across a change that alters what the sidebar draws, so the scenes
// have to be the same scene. "A context menu open over a dialog" is not reproducible by hand —
// opened where, over which dialog? — and the difference between two hand-composed attempts lands in
// the figure without appearing in it. So the scene is composed by the application and, more
// importantly, *checked* before any figure is reported.

/// The facts a correctly composed baseline scene satisfies.
fn baseline_facts() -> SceneFacts {
    SceneFacts {
        worktrees: REFERENCE_WORKTREES,
        running_sessions: 1,
        dialog_open: true,
        context_menu_open: true,
        ripple_animating: false,
    }
}

#[test]
fn an_unset_scene_variable_selects_no_scene() {
    assert_eq!(Scene::from_env_value(None), Ok(None));
    assert_eq!(Scene::from_env_value(Some("")), Ok(None));
}

#[test]
fn the_two_scenes_are_named() {
    assert_eq!(
        Scene::from_env_value(Some("baseline")),
        Ok(Some(Scene::Baseline))
    );
    assert_eq!(Scene::from_env_value(Some("full")), Ok(Some(Scene::Full)));
}

/// Case and surrounding whitespace are forgiven; the name is a label, not a password.
#[test]
fn the_scene_name_is_case_and_whitespace_insensitive() {
    assert_eq!(
        Scene::from_env_value(Some("  BaseLine ")),
        Ok(Some(Scene::Baseline))
    );
    assert_eq!(Scene::from_env_value(Some("FULL")), Ok(Some(Scene::Full)));
}

/// An unrecognised name is refused, and the refusal lists what is valid. Silently falling back to
/// "no scene" would record an uncomposed window as the reference scene.
#[test]
fn an_unknown_scene_name_is_refused_and_lists_the_valid_ones() {
    let err = Scene::from_env_value(Some("basline")).expect_err("typo must be refused");
    assert!(err.contains("basline"), "must quote what was given: {err}");
    assert!(err.contains("baseline"), "must list `baseline`: {err}");
    assert!(err.contains("full"), "must list `full`: {err}");
}

/// The happy path: a correctly composed baseline scene passes.
#[test]
fn a_correctly_composed_baseline_scene_is_accepted() {
    assert_eq!(Scene::Baseline.check(&baseline_facts()), Ok(()));
}

/// The whole point of the check. FR-039b names **20** worktrees, and a run against 19 produces a
/// figure that looks exactly like a good one — there is nothing in `300 frames — mean 0.30 ms` that
/// says which sidebar it was measured against.
#[test]
fn a_scene_with_the_wrong_worktree_count_is_refused() {
    let facts = SceneFacts {
        worktrees: 19,
        ..baseline_facts()
    };
    let err = Scene::Baseline
        .check(&facts)
        .expect_err("19 worktrees is not the scene");
    assert!(err.contains("19"), "must say what was found: {err}");
    assert!(err.contains("20"), "must say what was expected: {err}");
}

/// Each remaining element of the scene is individually load-bearing: the session brings the
/// terminal grid, the dialog brings the scrim and shadow, the menu brings overlay stacking. A
/// figure missing any of them is measuring a different scene.
#[test]
fn each_missing_scene_element_is_refused_by_name() {
    let cases: [(SceneFacts, &str); 3] = [
        (
            SceneFacts {
                running_sessions: 0,
                ..baseline_facts()
            },
            "session",
        ),
        (
            SceneFacts {
                dialog_open: false,
                ..baseline_facts()
            },
            "dialog",
        ),
        (
            SceneFacts {
                context_menu_open: false,
                ..baseline_facts()
            },
            "menu",
        ),
    ];
    for (facts, expected) in cases {
        let err = Scene::Baseline
            .check(&facts)
            .expect_err("an incomplete scene must be refused");
        assert!(
            err.to_lowercase().contains(expected),
            "the refusal must name the missing element `{expected}`: {err}"
        );
    }
}

/// The baseline scene is defined by having **no** ripple — it is the scene capturable on the
/// pre-change build, where the ripple does not exist. One mid-animation means the operator selected
/// the wrong scene, and the figure would land in the wrong §B8 slot.
#[test]
fn a_baseline_scene_showing_a_ripple_is_refused() {
    let facts = SceneFacts {
        ripple_animating: true,
        ..baseline_facts()
    };
    assert!(Scene::Baseline.check(&facts).is_err());
}

/// The full scene is the baseline plus a ripple mid-animation.
#[test]
fn the_full_scene_requires_a_ripple() {
    let without = baseline_facts();
    let err = Scene::Full
        .check(&without)
        .expect_err("the full scene without a ripple is the baseline scene");
    assert!(
        err.to_lowercase().contains("ripple"),
        "the refusal must name the ripple: {err}"
    );

    let with = SceneFacts {
        ripple_animating: true,
        ..baseline_facts()
    };
    assert_eq!(Scene::Full.check(&with), Ok(()));
}

/// The full scene inherits every baseline requirement rather than checking the ripple alone — a
/// ripple over an empty window is not the scene either.
#[test]
fn the_full_scene_still_requires_the_rest_of_the_baseline() {
    let facts = SceneFacts {
        worktrees: 3,
        ripple_animating: true,
        ..baseline_facts()
    };
    assert!(Scene::Full.check(&facts).is_err());
}

// ---------------------------------------------------------------------------------------------
// The scene has to still be the scene when the frames are counted (T083 — FR-039b, SC-018)
// ---------------------------------------------------------------------------------------------

/// A scene that drifts *after* it was composed is refused.
///
/// The original check ran until it passed and then never again, which left the 300 counted frames
/// measured against whatever the window drifted into. That is the same error the check exists to
/// prevent — "there is nothing in `300 frames — mean 0.84 ms` that says what it was measured
/// against" — reappearing one step later in the run, and it showed up exactly as the theory
/// predicts: six `full` runs in two clusters 60% apart, with baseline runs interleaved between them
/// holding steady.
#[test]
fn a_scene_that_drifts_mid_run_is_refused() {
    let mut facts = baseline_facts();
    facts.context_menu_open = false;

    let err = Scene::Baseline
        .check_still_composed(&facts)
        .expect_err("a dismissed context menu must not go unnoticed mid-run");
    assert!(
        err.contains("context menu"),
        "the refusal must name what drifted: {err}"
    );
}

/// The mid-run check covers every element that has to hold continuously.
#[test]
fn every_continuous_element_is_checked_mid_run() {
    for (name, break_it) in [
        (
            "worktrees",
            (|f: &mut SceneFacts| f.worktrees = 19) as fn(&mut SceneFacts),
        ),
        ("session", |f: &mut SceneFacts| f.running_sessions = 0),
        ("dialog", |f: &mut SceneFacts| f.dialog_open = false),
        ("context menu", |f: &mut SceneFacts| {
            f.context_menu_open = false
        }),
    ] {
        let mut facts = baseline_facts();
        break_it(&mut facts);
        assert!(
            Scene::Baseline.check_still_composed(&facts).is_err(),
            "{name} drifted mid-run and the check passed anyway"
        );
    }
}

/// The mid-run check does **not** fail on a momentarily absent ripple.
///
/// The ripple is the one element that legitimately blinks: it runs its cycle, settles, and is
/// re-pressed on the frame after. A per-frame check that demanded one would refuse nearly every
/// honest run, so the ripple is held to a coverage fraction over the whole run instead.
#[test]
fn a_ripple_between_cycles_does_not_fail_the_mid_run_check() {
    let mut facts = baseline_facts();
    facts.ripple_animating = false;
    assert_eq!(Scene::Full.check_still_composed(&facts), Ok(()));
}

/// The full scene must have had a ripple for most of the run, not for one frame of it.
#[test]
fn the_full_scene_requires_the_ripple_to_cover_most_of_the_run() {
    // A run that showed a ripple almost throughout is what the scene describes.
    assert_eq!(Scene::Full.check_ripple_coverage(291, 300), Ok(()));
    // One that showed it for a handful of frames measured the baseline under the full scene's name.
    let err = Scene::Full
        .check_ripple_coverage(12, 300)
        .expect_err("a ripple present for 4% of the run is not `a ripple mid-animation`");
    assert!(
        err.contains("12") && err.contains("300"),
        "the refusal must state what it saw: {err}"
    );
}

/// The baseline scene must have had **no** ripple at any point.
#[test]
fn the_baseline_scene_refuses_any_ripple_over_the_run() {
    assert_eq!(Scene::Baseline.check_ripple_coverage(0, 300), Ok(()));
    assert!(
        Scene::Baseline.check_ripple_coverage(1, 300).is_err(),
        "a ripple on even one counted frame means the baseline slot would hold the wrong scene"
    );
}
