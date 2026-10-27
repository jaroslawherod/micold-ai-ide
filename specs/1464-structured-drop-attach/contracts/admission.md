# Contract: composer file admission

Entry points (both end in `Msg::Dropped`):
1. Drop: `window::Event::FileDropped(path)` per file -> `SplitView` -> `PaneMsg::FileDropped(pane,path)` -> settle (40 ms) -> `target_of` = `DropTarget::Composer(session)` -> `read_for_composer`.
2. Picker: `Msg::AttachRequested(session)` -> `rfd` pick_files -> `Msg::AttachPicked{session,paths}` -> `read_for_composer`.

Rules (`apply_drop`): order preserved; images (PNG/JPEG/GIF/WebP by sniff) -> chips via `admit`, refusal text names file and reason; others -> `plan_insertion` quoted at caret; empty input -> no change, no notice; session not accepting input -> no change, one notice naming the count and the reason; cancel -> no change, no notice; open picker -> second request ignored.

Parity invariant (FR-008, SC-001): for the same ordered path list, draft text, images and notice are equal across both entry points.

Cue: `PaneMsg::FilesHovering(bool)`; no effect on admission.
