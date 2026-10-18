# Data model: Overview Card Ripple

## Card ripple (per card, widget tree state)
`cdk::ripple::Ripple` (existing): `origin: Option<Point>`, `extent`, expand and fade `Progress`. Created by `press(at, extent)` (pointer: `Some(position_in)`; keyboard: `None` → centre), idle once faded. Lives in the `material::Ripple` wrapper's tree state, one per card instance; nothing shared.

## `features::overview::State` (changed)
| Field | Type | Meaning |
|---|---|---|
| `activating` | `Option<micold_core::overview::Activation>` | the activation accepted and waiting out the 300 ms; `None` otherwise |

Transitions: `None --ActivationRequested(a) [open]--> Some(a)`; `Some --ActivationDue (root takes it)--> None`, then the root handles `CardActivated(a)` (closes, outcome as today); `Some --ProjectSwitched--> None`; `Some` ignores `ActivationRequested`, `Closed`, `Toggled`. `ActivationDue` while `None` is a no-op.

## `Msg` (changed)
Adds `ActivationRequested(Activation)` (emitted by cards) and `ActivationDue`; `CardActivated` is unchanged. Not persisted, not on the wire.
