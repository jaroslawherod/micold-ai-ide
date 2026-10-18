# Research: Overview Card Ripple

## R1 Reuse `material::Ripple` for the pointer ripple
- **Decision**: wrap the card's pressable in `material::Ripple` (tint `palette.text`, radius `RADIUS` = 12).
- **Rationale**: it already gives press origin, clamping, band clipping, the shared EXPAND/FADE, and the single-state-layer rule (`ripple.rs`, `cdk/ripple.rs`).
- **Rejected**: a card-local ripple (forks the shared one, Principle VIII); a pressed tint only (the issue's rejected alternative).

## R2 Keyboard origin by observing the child's capture
- **Decision**: `Ripple::centred_on_keys()`; start a centred ripple when Enter/Space was captured by the child during this event.
- **Rationale**: `TakesTheKeyboard::update` publishes and captures only while it holds focus, so focus gating is inherited with no new coupling or shared state; the ripple starts on the key press, as the message is published (clarification 1).
- **Rejected**: (a) ripple reading `Focus` state from the child tree — couples two widgets' private state; (b) starting the ripple from the application on `CardActivated` — needs a per-card id registry, violating FR-024e's no-registry rule; (c) turning it on for every `Ripple` — changes buttons, rows and chips (Assumptions).

## R3 Delay lives in feature state plus a root timer
- **Decision**: `State.activating: Option<Activation>`; `Task::perform(tokio::time::sleep(300 ms))` → `Msg::ActivationDue`.
- **Rationale**: iced has no cancellable view-level delay; a Task is the pattern `pr_status.rs` uses; the first activation is stored so a second cannot replace it (FR-007). Using `duration::MEDIUM_2` ties the delay to the ripple's expand time.
- **Rejected**: an iced `Subscription` timer (needs lifecycle bookkeeping for a single shot); removing `on_press` from cards while pending (changes the widget tree and resets the ripple's state); delaying inside the card widget (the widget does not own navigation).

## R4 Input during the delay
- **Decision**: ignore in `features::overview::update` (`ActivationRequested`, `Closed`, `Toggled`); a `Closed` from another path inside the 300 ms is dropped too (accepted); cards keep `on_press`.
- **Rationale**: one place, testable without a GUI; keeping the tree stable keeps the ripple alive. Arrow focus moves stay harmless.
- **Rejected**: swallowing events in the view (many sources: chord, Escape, clicks).

## R5 Edge cases
- Project switch during the delay: `ProjectSwitched` clears `activating`; the late `ActivationDue` is then a no-op.
- Card removed mid-ripple: ripple state is the card's tree state, dropped with it; nothing is animated centrally.
- Location state change mid-ripple: tint is read each draw, but the card keeps its palette from its model; spec requires the starting colour only loosely, and Overview is frozen for 300 ms (the fade may outlive the close, which discards it).
