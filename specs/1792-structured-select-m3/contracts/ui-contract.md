# UI contract: 1792

## §1 Select trigger (shared)
- One trigger definition used by `Select` and `MenuSelect`; builder API, ends in `.into()`; `Roles` passed in.
- Container shape, fill or outline, label, value, trailing arrow, state layers from `tokens::{shape,state,anatomy}` and `Roles`; no literals.
- States rest, hover, focus, pressed, disabled (no reaction), open (indicator emphasised as `Select` does today).
- The options row's Model, Effort and Permission controls are all `MenuSelect` with this trigger; same width rules (`MIN_TRIGGER_WIDTH`, model `MODEL_PICKER_WIDTH`).
- Long value: ellipsized inside the container, arrow stays visible. Value absent from the options: trigger renders its text. Empty option list: opening shows no menu.
- Menu: `material::picker` surface, per-option state layers, selected emphasis; unchanged.
- Existing messages (`ModelPicked`, effort and permission picks, `*Closed`, `*Moved`) are unchanged.

## §2 Options row expand
- `expand(options_row, options_open, over)`; `over` is `tokens::motion::duration::MEDIUM_2`; easing the wrapper's standard pair.
- `over == Duration::ZERO` when `settings.reduce_motion`.
- Toggle mid-animation reverses from current height; end height is the row's wrapped height.
- Controls of a collapsed row are not reachable by keyboard or pointer, and an open picker is closed when the row collapses.
- `composer_options_open` persistence unchanged.
