# UI contract: Overview Card Ripple

## `material::Ripple` (extended)
- `Ripple::new(content, tint, radius)` — unchanged.
- `Ripple::centred_on_keys(self) -> Self` — new, default off. When on, an Enter or Space `KeyPressed` that the wrapped content captured during this event starts a ripple at the element's centre unless one is already running. No effect on pointer behaviour.

## `OverviewCard`
- Activatable (`on_press` set): wrapped in `Ripple` with tint = the state's text/content colour (`palette(...).text`), radius 12, `centred_on_keys()`.
- Plain: unchanged, no ripple, no focus.

## `features::overview`
- `Msg::ActivationRequested(Activation)` (new, emitted by a card): stores the activation, Overview stays open. `Msg::ActivationDue` (new): the root takes the stored activation and handles `CardActivated`, which is unchanged.
- Delay = `Duration::from_millis(duration::MEDIUM_2)`.
- While `activating`, `ActivationRequested`/`Closed`/`Toggled` are ignored.
