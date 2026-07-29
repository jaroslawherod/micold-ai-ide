//! Frame-time statistics for the reference-scene measurement (feature 018 — FR-039b, SC-018).
//!
//! SC-018 asks for three frame-time figures — the baseline scene before the visual change, the
//! baseline scene after it, and the full scene after it — recorded on one machine so a reviewer can
//! compare them. This module is the part of that which can be tested: it turns a stream of
//! per-frame durations into the summary that gets written down.
//!
//! **It does not decide when a frame happens.** Driving the render loop and timing each frame is
//! the client's job; everything about *what the numbers mean* is here, where a test can reach it.
//!
//! Two definitions are load-bearing and are pinned by tests rather than by convention, because two
//! builds measured under different definitions are not comparable and the difference would be
//! invisible in the recorded figure:
//!
//! - **Warm-up samples are discarded.** The first frames after a scene is composed pay for pipeline
//!   and glyph-cache warm-up that no later frame pays.
//! - **`p95` is nearest-rank**: the `ceil(0.95 × n)`-th smallest sample, 1-indexed. "p95" names a
//!   family of definitions that disagree with each other on small runs.
//!
//! The measurement is reported for trend and never gates a build (FR-039c): a wall-clock threshold
//! enforced on software-rendered CI runners would fail on runner variance rather than on the change
//! under review.

use std::time::Duration;

/// The outcome of one measurement run over one scene.
///
/// Deliberately has no `Default` and is only ever produced by [`FrameProbe::summary`], so a summary
/// full of zeros cannot be constructed and mistaken for a fast result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    /// Frames counted, excluding discarded warm-up. Never zero — a run with nothing to report
    /// yields `None` instead of a `Summary`.
    pub frames: usize,
    /// Arithmetic mean over the counted frames.
    pub mean: Duration,
    /// Nearest-rank 95th percentile: the `ceil(0.95 × frames)`-th smallest sample. On runs of ten
    /// or fewer this equals [`max`](Self::max) — such a run cannot distinguish a tail from a peak.
    pub p95: Duration,
    /// The slowest counted frame.
    pub max: Duration,
}

impl Summary {
    /// The one-line record for `quickstart.md` §B8's "Frame time:" slot.
    ///
    /// Formatting lives here rather than at the print site because §B8 holds *three* figures that
    /// only mean something compared with each other. Three lines written to different precisions
    /// are not comparable at a glance, which is the whole reason they are recorded together.
    ///
    /// Always two decimal places: a fast scene renders in hundreds of microseconds, and truncating
    /// to whole milliseconds would report every such build as `0 ms`.
    pub fn report_line(&self) -> String {
        fn millis(d: Duration) -> String {
            format!("{:.2} ms", d.as_secs_f64() * 1_000.0)
        }
        format!(
            "{} frames — mean {}, p95 {}, max {}",
            self.frames,
            millis(self.mean),
            millis(self.p95),
            millis(self.max),
        )
    }
}

/// Warm-up frames discarded when the environment does not say otherwise.
///
/// Generous on purpose. Overshooting costs a fraction of a second of an already-manual procedure;
/// undershooting folds pipeline and glyph-cache warm-up into the figure, which is invisible in the
/// result and makes the two builds it compares incomparable.
pub const DEFAULT_WARM_UP: usize = 30;

/// The environment variable that turns the measurement mode on.
pub const ENV_VAR: &str = "MICOLD_FRAME_PROBE";

/// One measurement run, as configured from the environment (FR-039b, T000z/T076a).
///
/// The mode this enables drives the window at full rate and then exits the process — exactly what
/// the reference-scene capture needs, and exactly what should never happen by accident. So the
/// decision of what does and does not enable it is made here, under test, rather than by an
/// `unwrap_or_default` in the binary's glue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeConfig {
    /// How many frames to **count**. Warm-up is not part of this quota — otherwise raising the
    /// warm-up would silently shorten the measurement the warm-up exists to protect.
    pub frames: usize,
    /// How many frames to discard before counting starts.
    pub warm_up: usize,
}

impl ProbeConfig {
    /// Read the configuration from [`ENV_VAR`]'s value.
    ///
    /// - `None`, empty, or all-whitespace → `Ok(None)`: the application starts normally.
    /// - `"<frames>"` → that many counted frames at [`DEFAULT_WARM_UP`].
    /// - `"<frames>:<warm_up>"` → both stated.
    /// - anything else → `Err` with a message naming the variable and the grammar.
    ///
    /// A malformed value is an error rather than a silent "off". Someone typing
    /// `MICOLD_FRAME_PROBE=yes` and getting an ordinary launch would conclude the probe is broken;
    /// worse, a typo during a T000z capture would record an ordinary session as a measurement run.
    pub fn from_env_value(raw: Option<&str>) -> Result<Option<Self>, String> {
        let Some(raw) = raw else {
            return Ok(None);
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        let reject = || {
            format!(
                "{ENV_VAR}=\"{raw}\" is not a measurement run. Expected a frame count, optionally \
                 with a warm-up: `{ENV_VAR}=300` or `{ENV_VAR}=300:60` (300 counted frames after \
                 60 discarded). Unset it to start normally."
            )
        };

        let mut parts = trimmed.split(':');
        let frames: usize = parts
            .next()
            .ok_or_else(reject)?
            .trim()
            .parse()
            .map_err(|_| reject())?;
        let warm_up = match parts.next() {
            Some(w) => w.trim().parse().map_err(|_| reject())?,
            None => DEFAULT_WARM_UP,
        };
        if parts.next().is_some() {
            return Err(reject());
        }
        if frames == 0 {
            return Err(format!(
                "{ENV_VAR}=\"{raw}\" would count no frames, so it could only ever report nothing. \
                 Give a frame count of at least 1."
            ));
        }

        Ok(Some(Self { frames, warm_up }))
    }

    /// A probe configured for this run.
    pub fn probe(&self) -> FrameProbe {
        FrameProbe::new(self.warm_up)
    }

    /// Whether `probe` has collected everything this run asked for.
    pub fn is_complete(&self, probe: &FrameProbe) -> bool {
        probe.counted() >= self.frames
    }
}

/// Worktrees the reference scene is defined over (FR-039b).
pub const REFERENCE_WORKTREES: usize = 20;

/// The environment variable naming which reference scene to compose and measure.
pub const SCENE_ENV_VAR: &str = "MICOLD_FRAME_PROBE_SCENE";

/// Which of FR-039b's two reference scenes a run is measuring.
///
/// The scenes exist as a pair because the ripple this feature introduces cannot exist on the build
/// measured first: [`Baseline`](Self::Baseline) is capturable on both builds and is the like-for-like
/// comparison SC-018 turns on; [`Full`](Self::Full) is the baseline plus a ripple and is post-change
/// only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    /// 20 worktrees, sidebar expanded, one running session, a context menu over a dialog. No ripple.
    Baseline,
    /// The baseline scene plus a ripple mid-animation.
    Full,
}

/// What the application observes about the scene actually on screen.
///
/// Gathered by the binary from live state and handed here to be judged, so the rule for "is this the
/// reference scene" is testable rather than an `if` buried in render glue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneFacts {
    /// Worktree rows the sidebar is showing.
    pub worktrees: usize,
    /// Sessions currently running.
    pub running_sessions: usize,
    /// Whether a modal dialog is open.
    pub dialog_open: bool,
    /// Whether a context menu is open.
    pub context_menu_open: bool,
    /// Whether a ripple is mid-animation.
    pub ripple_animating: bool,
}

impl Scene {
    /// Read the scene from [`SCENE_ENV_VAR`]'s value. `Ok(None)` means no scene was named.
    ///
    /// An unrecognised name is refused rather than falling back to "no scene", which would measure
    /// an uncomposed window and record it as the reference scene.
    pub fn from_env_value(raw: Option<&str>) -> Result<Option<Self>, String> {
        let Some(raw) = raw else {
            return Ok(None);
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        match trimmed.to_ascii_lowercase().as_str() {
            "baseline" => Ok(Some(Scene::Baseline)),
            "full" => Ok(Some(Scene::Full)),
            _ => Err(format!(
                "{SCENE_ENV_VAR}=\"{raw}\" is not a reference scene. Valid scenes are `baseline` \
                 (capturable on both builds — this is the comparison SC-018 turns on) and `full` \
                 (the baseline plus a ripple, post-change only)."
            )),
        }
    }

    /// Whether the scene on screen is the one this run claims to be measuring.
    ///
    /// This is the check that makes three figures comparable. Without it a run against 19 worktrees,
    /// or with the dialog already dismissed, produces a figure indistinguishable from a good one —
    /// there is nothing in `300 frames — mean 0.30 ms` that says what it was measured against, and
    /// the error would surface only as an unexplained gap between slots.
    pub fn check(&self, facts: &SceneFacts) -> Result<(), String> {
        let mut wrong = Vec::new();

        if facts.worktrees != REFERENCE_WORKTREES {
            wrong.push(format!(
                "the sidebar shows {} worktrees, but the reference scene is {REFERENCE_WORKTREES} \
                 (build it with `mise run fixture`)",
                facts.worktrees
            ));
        }
        if facts.running_sessions == 0 {
            wrong.push("no terminal session is running; the scene needs one".to_string());
        }
        if !facts.dialog_open {
            wrong.push("no dialog is open".to_string());
        }
        if !facts.context_menu_open {
            wrong.push("no context menu is open".to_string());
        }

        match self {
            // The baseline scene is *defined* by having no ripple — it is what the pre-change build
            // can produce. One here means the wrong scene was selected, and the figure would be
            // recorded in the wrong §B8 slot.
            Scene::Baseline if facts.ripple_animating => {
                wrong.push(
                    "a ripple is animating, but the baseline scene has none — did you mean \
                     `full`?"
                        .to_string(),
                );
            }
            Scene::Full if !facts.ripple_animating => {
                wrong.push(
                    "no ripple is animating; the full scene is the baseline plus a ripple \
                     mid-animation, and it exists only on the post-change build"
                        .to_string(),
                );
            }
            _ => {}
        }

        if wrong.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "the window is not showing the {self:?} reference scene:\n  - {}",
                wrong.join("\n  - ")
            ))
        }
    }
}

/// Accumulates per-frame durations and summarises them.
///
/// ```
/// use std::time::Duration;
/// use micold_core::frame_probe::FrameProbe;
///
/// let mut probe = FrameProbe::new(30); // discard 30 warm-up frames
/// probe.record(Duration::from_millis(4));
/// ```
#[derive(Debug, Clone)]
pub struct FrameProbe {
    warm_up: usize,
    discarded: usize,
    samples: Vec<Duration>,
}

impl FrameProbe {
    /// A probe that discards the first `warm_up` frames before counting anything.
    ///
    /// Pass `0` to count every frame — legitimate when the scene is already warm, and the caller
    /// should not be forced to throw away good samples to satisfy a convention.
    pub fn new(warm_up: usize) -> Self {
        Self {
            warm_up,
            discarded: 0,
            samples: Vec::new(),
        }
    }

    /// Offer one frame's duration. Discarded while warm-up is still being served.
    pub fn record(&mut self, frame: Duration) {
        if self.discarded < self.warm_up {
            self.discarded += 1;
            return;
        }
        self.samples.push(frame);
    }

    /// Frames counted so far, excluding warm-up.
    pub fn counted(&self) -> usize {
        self.samples.len()
    }

    /// The summary, or `None` when nothing was counted.
    ///
    /// `None` rather than a zeroed `Summary` on purpose: a zero frame time is a *result*, and a run
    /// that collected no data is not one. Writing "0.00 ms" into the record would read as a
    /// measurement instead of as an empty run.
    pub fn summary(&self) -> Option<Summary> {
        let frames = self.samples.len();
        if frames == 0 {
            return None;
        }

        let mut sorted = self.samples.clone();
        sorted.sort_unstable();

        let total: Duration = sorted.iter().sum();
        let mean = total / frames as u32;

        // Nearest rank, in integer arithmetic so no float rounding can shift the index:
        //   ceil(0.95 * n) == ceil(19n / 20)
        let rank = (19 * frames).div_ceil(20);
        let p95 = sorted[rank - 1];

        let max = *sorted.last().expect("frames > 0");

        Some(Summary {
            frames,
            mean,
            p95,
            max,
        })
    }
}
