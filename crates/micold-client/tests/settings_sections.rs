//! Every setting lands in exactly one section (feature 027, T061/T063 — FR-027, FR-028, FR-026a).
//!
//! # The risk this exists for
//!
//! Turning a 420dp modal into a sectioned view is a *migration*, and the failure mode of a
//! migration is not a crash: it is a setting that quietly stops being reachable. The scrollback
//! limit rendered in no section still parses, still saves, still round-trips through
//! `settings.json` — every existing test stays green — and the user simply cannot change it any
//! more. Nothing else in this workspace would notice.
//!
//! So this holds the two halves of that property:
//!
//! - **Nothing is dropped.** Every field of the persisted settings shape is claimed by a section.
//! - **Nothing is claimed twice.** Two sections rendering the scrollback limit is the other half
//!   of FR-027's "in exactly one section" — two controls writing one draft field disagree the
//!   moment the user edits both.
//!
//! # Why the sections declare their own settings
//!
//! Each section module states what it renders as a `SETTINGS` constant, and this file checks that
//! declaration against two independent things: the persisted shape in `micold-core` (so a new
//! field cannot be forgotten) and the module's own source (so a declaration cannot be a fiction).
//! A scan that tried to *infer* which settings a module rendered would be guessing from field
//! accesses through a draft, and would go wrong in both directions — it cannot see a control that
//! reads its value indirectly, and it cannot tell a rendered setting from a mentioned one.
//!
//! # Not being rendered yet is allowed; being unrendered *and* unrecorded is not
//!
//! [`DEFERRED`] carries the settings a later task in this feature lands, each naming that task.
//! [`a_deferred_setting_that_arrived_is_stale`] fails once one is claimed, so the list cannot
//! outlive its reason. The rule is not "every field has a control today"; it is that no field sits
//! in the state where the requirement is neither met nor waived.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Settings this feature renders in a later phase, and the task that does it. Keyed by the same
/// dotted path the persisted shape produces.
///
/// **Empty, and kept.** Both entries — the resource limits and the network posture — landed with
/// T086 and T087, and [`a_deferred_setting_that_arrived_is_stale`] is what made deleting them a
/// step rather than an oversight. The list stays because the next field added to `SandboxProfile`
/// needs somewhere to be recorded on the day it has no control yet, and rebuilding this machinery
/// then is how it does not get built.
const DEFERRED: &[(&str, &str)] = &[];

fn client_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn core_dir() -> PathBuf {
    client_dir().join("../micold-core/src")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The `pub` field names of a struct, in declaration order.
///
/// Text, not syntax: this asks what a source file *says*, the same reason `material_boundary.rs`
/// and `composite_call_sites.rs` read text. A `syn` dependency would buy precision this does not
/// need — these three structs are plain records of named fields.
fn fields_of(src: &str, struct_name: &str) -> Vec<String> {
    let start = src
        .find(&format!("pub struct {struct_name} {{"))
        .unwrap_or_else(|| panic!("`pub struct {struct_name}` not found — has it been renamed?"));
    let body = &src[start..];
    let end = body
        .find("\n}")
        .unwrap_or_else(|| panic!("unterminated `{struct_name}`"));
    body[..end]
        .lines()
        .skip(1)
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("pub ")?;
            let name = rest.split(':').next()?.trim();
            // `pub fn`, `pub const` and the like never appear inside a struct body, but a field
            // named with a raw identifier would — and a name with a space in it is not a field.
            (!name.is_empty() && !name.contains(' ')).then(|| name.to_string())
        })
        .collect()
}

/// Every setting the application persists, as the dotted path its JSON uses.
///
/// Derived from the three structs rather than listed here, so that adding a field to
/// `SandboxProfile` and forgetting the control is this test's failure rather than a user's.
fn persisted_settings() -> Vec<String> {
    let settings = read(&core_dir().join("settings.rs"));
    let sandbox = read(&core_dir().join("sandbox/mod.rs"));

    let mut out = Vec::new();
    for field in fields_of(&settings, "Settings") {
        if field == "daemon" {
            for daemon_field in fields_of(&settings, "DaemonConfig") {
                if daemon_field == "sandbox" {
                    for profile_field in fields_of(&sandbox, "SandboxProfile") {
                        out.push(format!("daemon.sandbox.{profile_field}"));
                    }
                } else {
                    out.push(format!("daemon.{daemon_field}"));
                }
            }
        } else {
            out.push(field);
        }
    }
    out
}

fn settings_dir() -> PathBuf {
    client_dir().join("src/ui/settings")
}

/// Each section module's file name and source.
fn section_sources() -> BTreeMap<String, String> {
    let dir = settings_dir();
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("dir entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if name == "mod.rs" || !name.ends_with(".rs") {
            continue;
        }
        out.insert(name, read(&path));
    }
    out
}

/// Where a module's `SETTINGS` declaration starts and ends, so the rest of the file can be read
/// as evidence *for* it rather than as part of it.
fn settings_block(src: &str) -> Option<(usize, usize)> {
    let start = src.find("pub const SETTINGS:")?;
    let end = src[start..].find("];").map(|e| start + e)?;
    Some((start, end))
}

/// The `(setting, message)` pairs a section module declares: the persisted setting it renders, and
/// the `Message` variant the control emits.
///
/// The message is what makes the declaration checkable. A bare list of setting names is a claim
/// with no evidence anywhere in the file — a module could declare five settings and wire two
/// controls, and nothing would notice. Naming the message means the declaration points at a line
/// of the module that has to exist, and at a variant `app.rs` has to carry.
fn declared_by(src: &str) -> Vec<(String, String)> {
    let Some((start, end)) = settings_block(src) else {
        return Vec::new();
    };
    let quoted: Vec<String> = src[start..end]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect();
    assert_eq!(
        quoted.len() % 2,
        0,
        "SETTINGS is a list of (setting, message) pairs; found an odd number of strings: {quoted:?}"
    );
    quoted
        .chunks(2)
        .map(|p| (p[0].clone(), p[1].clone()))
        .collect()
}

/// The module's source with its own declaration removed.
fn body_of(src: &str) -> String {
    match settings_block(src) {
        Some((start, end)) => format!("{}{}", &src[..start], &src[end..]),
        None => src.to_string(),
    }
}

/// Which section claims each setting.
fn claims() -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (module, src) in section_sources() {
        for (setting, _) in declared_by(&src) {
            out.entry(setting).or_default().push(module.clone());
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// FR-027 / FR-028: exactly one section, and nothing dropped
// ---------------------------------------------------------------------------------------------

/// The scan itself has to be able to fail. A rename that emptied either side would otherwise turn
/// every assertion below into a comparison of two empty sets.
#[test]
fn the_scan_finds_both_sides() {
    assert!(
        persisted_settings().len() >= 5,
        "the persisted settings shape came back nearly empty — `Settings`, `DaemonConfig` or \
         `SandboxProfile` has been renamed and this gate is now comparing nothing to nothing"
    );
    let sections = section_sources();
    assert!(
        sections.len() >= 4,
        "expected a module per section under src/ui/settings, found {:?}",
        sections.keys().collect::<Vec<_>>()
    );
}

/// FR-027's load-bearing half: a setting that existed before feature 027 is still reachable.
///
/// These five are named literally, not derived, because "what existed before this feature" is a
/// historical fact about the previous release rather than a property of today's source — deriving
/// it from the current shape would let a deletion redefine the thing being checked.
#[test]
fn every_setting_that_predates_this_feature_is_still_rendered() {
    let before = [
        "theme",
        "scrollback_lines",
        "env_include_enabled",
        "env_include_script_path",
        "env_include_timeout_secs",
    ];
    let claims = claims();
    let missing: Vec<_> = before
        .iter()
        .filter(|s| !claims.contains_key(**s))
        .collect();
    assert!(
        missing.is_empty(),
        "settings that were editable before the sectioned view are now in no section: {missing:?}\n\
         A dropped setting still parses, still saves and still round-trips — the user simply \
         cannot change it any more, and nothing else here would notice (FR-027)."
    );
}

/// The other half of "exactly one": two controls writing one value disagree the moment both are
/// touched, and the user has no way to know which won.
#[test]
fn no_setting_is_claimed_by_two_sections() {
    let duplicated: Vec<_> = claims()
        .into_iter()
        .filter(|(_, modules)| modules.len() > 1)
        .collect();
    assert!(
        duplicated.is_empty(),
        "these settings are rendered in more than one section: {duplicated:?} (FR-027)"
    );
}

/// A field added to the persisted shape gets a control, or an entry in [`DEFERRED`] naming the
/// task that gives it one.
#[test]
fn every_persisted_setting_is_claimed_or_recorded_as_deferred() {
    let claims = claims();
    let deferred: BTreeSet<&str> = DEFERRED.iter().map(|(s, _)| *s).collect();
    let unaccounted: Vec<_> = persisted_settings()
        .into_iter()
        .filter(|s| !claims.contains_key(s) && !deferred.contains(s.as_str()))
        .collect();
    assert!(
        unaccounted.is_empty(),
        "these persisted settings are rendered by no section and recorded as deferred by \
         nothing: {unaccounted:?}\n\
         Add the control, or add the setting to DEFERRED with the task that lands it."
    );
}

/// A deferral outlives its reason the moment the control arrives.
#[test]
fn a_deferred_setting_that_arrived_is_stale() {
    let claims = claims();
    let arrived: Vec<_> = DEFERRED
        .iter()
        .filter(|(setting, _)| claims.contains_key(*setting))
        .map(|(setting, _)| *setting)
        .collect();
    assert!(
        arrived.is_empty(),
        "these settings are recorded in DEFERRED but are now rendered: {arrived:?} — delete the \
         entries, so the list keeps meaning what it says"
    );
}

/// A declaration is a claim that the module renders a control, so a control has to be there.
///
/// Read from the module with its own `SETTINGS` block removed, or the declaration would be its own
/// evidence and this would pass for a module that renders nothing at all.
#[test]
fn every_declared_setting_has_a_control_in_its_module() {
    let mut violations = Vec::new();
    for (module, src) in section_sources() {
        let body = body_of(&src);
        for (setting, message) in declared_by(&src) {
            if !body.contains(&format!("SettingsMsg::{message}")) {
                violations.push(format!(
                    "{module} declares `{setting}` as emitting \
                     `Message::Settings(SettingsMsg::{message})`, but the module never emits it \
                     — the declaration describes a control that is not there"
                ));
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// And the message it names has to be a real one, so a renamed variant cannot leave the
/// declaration pointing at nothing.
///
/// The declaration names a variant of `features::settings::Msg`, not of `app::Message`: since
/// feature 028 the root carries one `Settings(settings::Msg)` arm and the variants live in the
/// feature that owns them, so that module — not `app.rs` — is where "is this a real message?" is
/// answered.
#[test]
fn every_declared_message_exists_on_the_application_message() {
    let vocabulary = read(&client_dir().join("src/features/settings.rs"));
    let mut violations = Vec::new();
    for (module, src) in section_sources() {
        for (setting, message) in declared_by(&src) {
            if !vocabulary.contains(&format!("{message}("))
                && !vocabulary.contains(&format!("{message},"))
            {
                violations.push(format!(
                    "{module} declares `{setting}` as \
                     `Message::Settings(SettingsMsg::{message})`, which \
                     `features/settings.rs` does not carry"
                ));
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// FR-028: the service's settings are together, and only together.
#[test]
fn every_session_service_setting_is_in_the_daemon_section() {
    let stray: Vec<_> = claims()
        .into_iter()
        .filter(|(setting, _)| setting.starts_with("daemon."))
        .filter(|(_, modules)| modules.iter().any(|m| m != "daemon.rs"))
        .collect();
    assert!(
        stray.is_empty(),
        "session-service settings rendered outside the daemon section: {stray:?}\n\
         FR-028 asks for one place to reason about what the service does, which a setting \
         scattered into Terminal or Appearance defeats."
    );
}

// ---------------------------------------------------------------------------------------------
// FR-026a / Principle VIII: the rail is a component
// ---------------------------------------------------------------------------------------------

/// The navigation rail is a library component, not a list this view builds for itself.
///
/// `material_builder_api.rs` already holds every component in the library to Principle VIII's
/// shape, so what is left to check is the part it cannot see: that the settings surface *reaches*
/// one, rather than growing a private rail that no other view could use and no component gate
/// would ever inspect (FR-026a).
#[test]
fn the_section_rail_is_a_shared_component() {
    let component = client_dir().join("src/ui/material/section_list.rs");
    assert!(
        component.exists(),
        "expected the section rail at {} — a rail built privately inside the settings surface is \
         invisible to every component gate in this crate (FR-026a)",
        component.display()
    );

    let view = read(&client_dir().join("src/ui/settings_view.rs"));
    assert!(
        view.contains("SectionList"),
        "the settings view does not name `SectionList` — if the rail has been rebuilt locally, \
         FR-026a is not satisfied by the component merely existing"
    );
}

/// The component belongs to the library, so a settings-only copy cannot appear beside it.
#[test]
fn no_section_module_declares_its_own_rail() {
    let mut violations = Vec::new();
    for (module, src) in section_sources() {
        if src.contains("struct SectionList") || src.contains("fn section_list") {
            violations.push(module);
        }
    }
    assert!(
        violations.is_empty(),
        "these section modules declare a rail of their own: {violations:?} — there is one, in \
         `ui/material/section_list.rs`"
    );
}

// ---------------------------------------------------------------------------------------------
// What a picker shows for its current value (feature 027, T075)
// ---------------------------------------------------------------------------------------------

/// A `Select`'s trigger draws its selected value with `Display`, and [`Named`]'s `Display` is its
/// *name* — so a selected value built with an empty name renders a picker with a label, a chevron,
/// and nothing between them.
///
/// Every one of the four pickers in this view shipped that way, because [`Named`]'s `PartialEq`
/// compares the value and ignores the name: the open option list found and ticked the right row,
/// the unit tests passed, the layout gates measured a box of exactly the right size, and the
/// collapsed field was blank. A visual pass found it (T075); nothing automated could.
///
/// [`Named`]: micold_client::ui — `crate::ui::settings::Named`
#[test]
fn no_picker_is_given_a_selected_value_with_no_name() {
    for (module, src) in section_sources() {
        for (offset, _) in src.match_indices("Named(") {
            // The whole call, to its closing paren — nested parens included, since the value is
            // itself often a call.
            let rest = &src[offset..];
            let mut depth = 0usize;
            let mut end = rest.len();
            for (i, ch) in rest.char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let call = &rest[..end];
            assert!(
                !call.contains(", \"\")") && !call.contains(",\n            \"\",\n"),
                "in `ui/settings/{module}`, `{call}` names its value with the empty string. A \
                 picker draws its selected value by name, so this one renders blank — pass \
                 `name_of(OPTIONS, value)` instead"
            );
        }
    }
}

/// The gate above is worth nothing if it cannot see the call it is about, and the shape it matches
/// is fragile under rustfmt: the four call sites it guards are split across lines precisely
/// because they got longer when they were fixed.
#[test]
fn the_scan_reaches_every_pickers_selected_value() {
    let found: usize = section_sources()
        .values()
        .map(|src| src.matches("Some(Named(").count())
        .sum();
    assert!(
        found >= 4,
        "found {found} selected values written as `Some(Named(`; there are four pickers in this \
         view, so a lower count means the scan stopped seeing them"
    );
}

// ---------------------------------------------------------------------------------------------
// FR-026e: one setting, one control — and the app bar is not a second place to put one
// ---------------------------------------------------------------------------------------------

/// What the app bar's overflow menu may emit. **Actions, not settings.**
///
/// Named rather than derived, because "is this message a setting?" has no answer in the source: the
/// theme cycle emitted `ThemeModeCycled` and the survival opt-in emitted `LogoutSurvivalRequested`,
/// neither of which looks like the `Settings…` variant the form uses for the same value. What can
/// be checked is the converse — that the menu offers only these three, each of which *opens* or
/// *reports* something rather than writing a value the form owns.
///
/// Spelled `Feature::Variant` since feature 028 folded the flat variants behind a per-feature
/// `Msg`: the same three messages, named the way the root now names them.
const MENU_MESSAGES: &[&str] = &[
    "Settings::Opened",
    "Connection::DiagnosticsRequested",
    "Help::AboutOpened",
];

/// Every `Message::…` variant `overflow_items` constructs, as `Feature::Variant`.
///
/// The root's vocabulary is two levels deep (feature 028): the wrapper names the feature and the
/// variant inside names the message. Reading only the wrapper would collapse all three of these to
/// three different features and tell us nothing about what they do.
fn menu_messages() -> BTreeSet<String> {
    let src = read(&client_dir().join("src/ui/toolbar.rs"));
    let start = src
        .find("pub fn overflow_items")
        .expect("`overflow_items` not found — has the menu been renamed?");
    let end = src[start..]
        .find("\n}")
        .map(|e| start + e)
        .expect("unterminated `overflow_items`");
    src[start..end]
        .match_indices("Message::")
        .map(|(i, _)| {
            let tail = &src[start + i + "Message::".len()..];
            let feature: String = tail
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let inner = tail[feature.len()..]
                .strip_prefix('(')
                .and_then(|rest| rest.split_once("::"))
                .map(|(_, variant)| {
                    variant
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect::<String>()
                })
                .unwrap_or_default();
            if inner.is_empty() {
                feature
            } else {
                format!("{feature}::{inner}")
            }
        })
        .collect()
}

/// FR-026e. The menu offers nothing the form owns.
///
/// This is the rule BUG-001 was the cost of not having. While Settings was a 420dp modal the menu
/// was behind a scrim and unreachable, so two controls for the theme were harmless; the full-surface
/// view leaves the app bar on screen, and the two became live simultaneous writers of one value. The
/// menu wrote the theme immediately, the open draft still held what it was when the view opened, and
/// Save reverted a choice the user had made two seconds earlier and watched take effect.
#[test]
fn the_overflow_menu_offers_no_setting() {
    let allowed: BTreeSet<String> = MENU_MESSAGES.iter().map(|s| s.to_string()).collect();
    let offered = menu_messages();
    assert!(
        !offered.is_empty(),
        "the scan found no messages in `overflow_items` — it has been restructured and this gate \
         is now checking nothing"
    );
    let extra: Vec<_> = offered.difference(&allowed).collect();
    assert!(
        extra.is_empty(),
        "the app bar's overflow menu emits {extra:?}, which is not one of the actions a menu may \
         offer.\n\
         If it writes a setting, it belongs in the section that owns that setting and nowhere \
         else (FR-026e). If it is genuinely an action, add it to MENU_MESSAGES with the reason."
    );
}

/// And the capability a removed duplicate provided is still reachable (SC-014).
///
/// "Keep sessions after logout" was not merely duplicated: the menu ran the Linux service-manager
/// flow, which is the *host-process* half of FR-014a's single opt-in, while the form set the
/// container's restart policy, which is the sandboxed half. Deleting the menu item alone would have
/// deleted the host-process capability with it. `logout_survival::enable_for` dispatches on the
/// resolved placement and is what makes one control cover both — so it has to have a caller.
#[test]
fn session_survival_still_reaches_both_placements() {
    let client = client_dir().join("src");
    let mut callers = Vec::new();
    for dir in ["features", "shell"] {
        let root = client.join(dir);
        let mut stack = vec![root];
        while let Some(path) = stack.pop() {
            let Ok(entries) = fs::read_dir(&path) else {
                continue;
            };
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|e| e.to_str()) == Some("rs")
                    && read(&p).contains("enable_for(")
                {
                    callers.push(p.display().to_string());
                }
            }
        }
    }
    assert!(
        !callers.is_empty(),
        "nothing in the client calls `logout_survival::enable_for`, so the survival opt-in acts \
         on at most one placement — and the app-bar item that covered the other one is gone \
         (FR-014d, SC-014)"
    );
}
