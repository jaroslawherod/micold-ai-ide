//! Turn usage and session totals (data-model.md *Turn usage*, research R3, FR-006–FR-009).

/// A turn's token figures as its result reports them (main conversation only, D5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tokens {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_creation: Option<u64>,
}

impl Tokens {
    /// No token figure at all.
    pub fn is_empty(&self) -> bool {
        self.input.is_none()
            && self.output.is_none()
            && self.cache_read.is_none()
            && self.cache_creation.is_none()
    }

    /// Each figure summed with `other`'s, `Some` when either reports it.
    fn add(self, other: Tokens) -> Tokens {
        let sum = |a: Option<u64>, b: Option<u64>| match (a, b) {
            (None, None) => None,
            (a, b) => Some(a.unwrap_or(0).saturating_add(b.unwrap_or(0))),
        };
        Tokens {
            input: sum(self.input, other.input),
            output: sum(self.output, other.output),
            cache_read: sum(self.cache_read, other.cache_read),
            cache_creation: sum(self.cache_creation, other.cache_creation),
        }
    }
}

/// One model's figures from a `modelUsage` entry, or the difference of two.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ModelFigures {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_creation: u64,
    pub cost_usd: f64,
    pub context_window: Option<u64>,
}

impl ModelFigures {
    /// Whether any figure is lower than `previous`'s: the CLI started counting again.
    fn fell_below(&self, previous: &ModelFigures) -> bool {
        self.input < previous.input
            || self.output < previous.output
            || self.cache_read < previous.cache_read
            || self.cache_creation < previous.cache_creation
            || self.cost_usd < previous.cost_usd
    }

    fn is_zero(&self) -> bool {
        self.input == 0
            && self.output == 0
            && self.cache_read == 0
            && self.cache_creation == 0
            && self.cost_usd == 0.0
    }

    fn minus(&self, previous: &ModelFigures) -> ModelFigures {
        ModelFigures {
            input: self.input - previous.input,
            output: self.output - previous.output,
            cache_read: self.cache_read - previous.cache_read,
            cache_creation: self.cache_creation - previous.cache_creation,
            cost_usd: self.cost_usd - previous.cost_usd,
            context_window: self.context_window,
        }
    }

    fn plus(&self, other: &ModelFigures) -> ModelFigures {
        ModelFigures {
            input: self.input.saturating_add(other.input),
            output: self.output.saturating_add(other.output),
            cache_read: self.cache_read.saturating_add(other.cache_read),
            cache_creation: self.cache_creation.saturating_add(other.cache_creation),
            cost_usd: self.cost_usd + other.cost_usd,
            context_window: other.context_window.or(self.context_window),
        }
    }
}

/// One turn's usage, keyed by its `Result` event's `seq` (#939's footer is at that `seq`).
#[derive(Debug, Clone, PartialEq)]
pub struct TurnUsage {
    pub result_seq: u64,
    /// As the result reports them (FR-006).
    pub tokens: Option<Tokens>,
    /// The turn's own cost (FR-009); `None` when it cannot be derived.
    pub cost_usd: Option<f64>,
    /// The turn's own per-model figures (FR-009), same rule as `cost_usd`.
    pub models: Option<Vec<(String, ModelFigures)>>,
    /// A running figure fell: the figures are the result's as they are.
    pub restarted: bool,
    /// In the session totals: it has token figures and a derivable cost.
    pub counted: bool,
}

/// The sums over the counted turns held (FR-007, FR-008).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionTotals {
    pub cost_usd: f64,
    pub tokens: Tokens,
    pub models: Vec<(String, ModelFigures)>,
    pub turns_counted: u64,
    /// Held results not counted, plus held turns that ended without a result.
    pub turns_left_out: u64,
    /// Earlier events are not held: the totals cover only the turns still held.
    pub partial: bool,
}

/// Figures per model, in the order the CLI lists them.
type PerModel = Vec<(String, ModelFigures)>;

/// The CLI's running figures of the last result folded.
#[derive(Debug, Clone, PartialEq)]
struct Running {
    cost: Option<f64>,
    models: Option<Vec<(String, ModelFigures)>>,
}

/// Turns and totals, folded digest by digest.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct UsageState {
    /// In `result_seq` order.
    pub(super) turns: Vec<TurnUsage>,
    pub(super) totals: SessionTotals,
    previous: Option<Running>,
}

impl UsageState {
    /// Folds one turn result at `seq`, given how many events before the first held one are not
    /// held.
    pub(super) fn result(
        &mut self,
        seq: u64,
        tokens: Option<Tokens>,
        cost_cum: Option<f64>,
        models_cum: Option<&Vec<(String, ModelFigures)>>,
        omitted: u64,
    ) {
        let (cost_usd, models, restarted) = match &self.previous {
            None if omitted == 0 => (cost_cum, models_cum.cloned(), false),
            None => (None, None, false),
            Some(previous) => turn_figures(cost_cum, models_cum, previous),
        };
        self.previous = Some(Running {
            cost: cost_cum,
            models: models_cum.cloned(),
        });
        let counted = tokens.is_some() && cost_usd.is_some();
        if counted {
            let totals = &mut self.totals;
            totals.turns_counted += 1;
            totals.cost_usd += cost_usd.unwrap_or(0.0);
            totals.tokens = totals.tokens.add(tokens.unwrap_or_default());
            for (model, figures) in models.iter().flatten() {
                match totals.models.iter_mut().find(|(m, _)| m == model) {
                    Some((_, sum)) => *sum = sum.plus(figures),
                    None => totals.models.push((model.clone(), *figures)),
                }
            }
        } else {
            self.totals.turns_left_out += 1;
        }
        self.turns.push(TurnUsage {
            result_seq: seq,
            tokens,
            cost_usd,
            models,
            restarted,
            counted,
        });
    }

    /// Folds a turn that ended without a result: left out of the totals.
    pub(super) fn ended_without_result(&mut self) {
        self.totals.turns_left_out += 1;
    }

    pub(super) fn turn(&self, result_seq: u64) -> Option<&TurnUsage> {
        self.turns
            .binary_search_by_key(&result_seq, |t| t.result_seq)
            .ok()
            .map(|i| &self.turns[i])
    }
}

/// FR-009: the running figures minus the previous result's, or as they are when one fell.
fn turn_figures(
    cost_cum: Option<f64>,
    models_cum: Option<&Vec<(String, ModelFigures)>>,
    previous: &Running,
) -> (Option<f64>, Option<PerModel>, bool) {
    let previous_model = |model: &str| {
        previous
            .models
            .iter()
            .flatten()
            .find(|(m, _)| m == model)
            .map(|(_, f)| f)
    };
    let restarted = matches!((cost_cum, previous.cost), (Some(now), Some(before)) if now < before)
        || models_cum.into_iter().flatten().any(|(model, figures)| {
            previous_model(model).is_some_and(|before| figures.fell_below(before))
        });
    if restarted {
        return (cost_cum, models_cum.cloned(), true);
    }
    let cost = cost_cum
        .zip(previous.cost)
        .map(|(now, before)| now - before);
    let models = models_cum.zip(previous.models.as_ref()).map(|(now, _)| {
        now.iter()
            .map(|(model, figures)| {
                let own = previous_model(model).map_or(*figures, |before| figures.minus(before));
                (model.clone(), own)
            })
            // A model the turn did not use.
            .filter(|(_, own)| !own.is_zero())
            .collect()
    });
    (cost, models, false)
}
