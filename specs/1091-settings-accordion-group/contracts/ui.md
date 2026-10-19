# UI contract: Settings accordions form a group

## Component (`ui::material`, Principle VIII)

```text
AccordionGroup::<K>::new()                       // K: Copy + Eq
group.is_open(key, default: Option<K>) -> bool
group.toggle(key, default: Option<K>)
group.reset()

AccordionSection::new(title, body, roles)
    .in_group(&group, key, default)              // = .open(group.is_open(key, default))
    .on_toggle(msg)                              // caller maps msg to group.toggle(key, default)
    .into()
```

- A section never passed to `in_group` is drawn and behaves exactly as before (FR-003); `open(bool)`
  stays.
- Header press, focus stop, ripple, summary, required marker and keyboard (Space, Enter) are the
  existing `Button`'s: grouped or not (FR-009, FR-005).
- Closing and opening animate together via each `Accordion`'s own reveal; no new motion.

## Daemon form

- All sections of a kind (`Section::of(runtime)`, container sections included) share the form's one
  group (FR-004).
- A form opens, or *Runs in* changes kind: only the kind's first section open (FR-007).
- Pressing the open section closes it, leaving none (FR-002). Pressing a closed section opens it
  and closes the other non-held one.
- A section holding a shown error (refused save or empty required field) is open, whatever the
  group says; its header press changes nothing; opening another section leaves it open (FR-006).

## Showcase

`AccordionSection` entry poses: "closed, with a summary", "open", "closed, a required field empty"
(unchanged), plus "a group: opening one closes the others" and "independent sections"; live
"press the header, or focus it and press Space or Enter" kept (FR-008).
