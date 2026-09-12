//! Two processes write `settings.json` — the file must survive it (SC-026 first clause, FR-010b;
//! bugfix BUG-025 of `010-daemon-session-persistence`).
//!
//! The client and the service both persist into this one file by design (FR-010, FR-012a,
//! FR-012b), and T100 wired the client to send `SettingsSet` *immediately* after its own write, so
//! the service answers with its own load-then-save microseconds later. Every Settings save is
//! therefore two writes to one path, from two writers that never coordinate.
//!
//! Before the fix both staged through `settings.json.tmp`, a name derived from the target and so
//! shared by every writer of it. Two saves that both truncate before either writes leave the
//! shorter document over the longer one's bytes; the result does not parse, and the next `load`
//! moves it aside and adopts defaults — every setting the user chose, gone.
//!
//! The reporter's own corrupt file is preserved at
//! `specs/010-daemon-session-persistence/bugs/BUG-025-evidence/settings.json.bak.2026-08-26`; the
//! shape asserted against here is the one it holds.

use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use std::sync::{Arc, Barrier};

/// Two documents whose serialised lengths differ a lot, so a torn write leaves a visible tail.
fn short() -> Settings {
    Settings {
        env_include_script_path: String::new(),
        ..Settings::default()
    }
}

fn long() -> Settings {
    Settings {
        env_include_script_path: "/home/u/dev/a-considerably-longer-env-include-script.sh".into(),
        ..Settings::default()
    }
}

#[test]
fn concurrent_saves_never_leave_a_file_that_cannot_be_parsed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");

    // Enough rounds to hit the interleaving reliably. Against the shared temp path this fails
    // within the first handful; the loop is for the machine that schedules kindly.
    for round in 0..200 {
        let a = JsonFileSettingsStore::at(path.clone());
        let b = JsonFileSettingsStore::at(path.clone());
        let gate = Arc::new(Barrier::new(2));

        let (g1, g2) = (Arc::clone(&gate), Arc::clone(&gate));
        let t1 = std::thread::spawn(move || {
            g1.wait();
            let _ = a.save(&long());
        });
        let t2 = std::thread::spawn(move || {
            g2.wait();
            let _ = b.save(&short());
        });
        t1.join().unwrap();
        t2.join().unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(
            serde_json::from_str::<serde_json::Value>(&raw).is_ok(),
            "round {round}: two concurrent saves left an unparseable settings file. \
             This is the BUG-025 shape — a short document with a longer one's tail hanging off \
             the end, which the next load moves aside and replaces with defaults:\n{raw}"
        );
    }
}

#[test]
fn a_concurrent_save_loses_no_writers_own_field() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");

    // Seed a stored document so both writers merge into something real.
    JsonFileSettingsStore::at(path.clone())
        .save(&Settings::default())
        .unwrap();

    let a = JsonFileSettingsStore::at(path.clone());
    let b = JsonFileSettingsStore::at(path.clone());
    let gate = Arc::new(Barrier::new(2));
    let (g1, g2) = (Arc::clone(&gate), Arc::clone(&gate));

    let t1 = std::thread::spawn(move || {
        g1.wait();
        a.save(&long())
    });
    let t2 = std::thread::spawn(move || {
        g2.wait();
        b.save(&short())
    });
    let (r1, r2) = (t1.join().unwrap(), t2.join().unwrap());

    // Neither save may report success while having produced a file that cannot be read back.
    let outcome = JsonFileSettingsStore::at(path).load();
    assert_eq!(
        outcome.status,
        micold_core::store::LoadStatus::Loaded,
        "a save reported {r1:?}/{r2:?} yet the file did not load cleanly afterwards"
    );
    // One of the two writers won; the value on disk must be one of theirs, not a blend.
    let landed = outcome.settings.env_include_script_path;
    assert!(
        landed == long().env_include_script_path || landed == short().env_include_script_path,
        "neither writer's value survived: {landed:?}"
    );
}

/// The second thing mutual exclusion buys, and the one rename-atomicity cannot (FR-010b, T154).
///
/// A torn write leaves a file nobody can parse; a *lost update* leaves a perfectly valid file
/// that is missing a change somebody was told had been saved. This is the shape the two real
/// writers make: the client owns `theme` and the service owns fields of its own, and each does
/// read → change its fields → write back. Interleave the two read-modify-write cycles and the
/// later writer's base is the document from *before* the earlier writer's change, so the earlier
/// change is written away. `merged_with_existing` does not save this: it preserves keys this
/// build does not know about, and both writers know about both keys.
#[test]
fn interleaved_updates_keep_both_writers_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");

    for round in 0..80 {
        JsonFileSettingsStore::at(path.clone())
            .save(&Settings::default())
            .unwrap();

        let a = JsonFileSettingsStore::at(path.clone());
        let b = JsonFileSettingsStore::at(path.clone());
        let gate = Arc::new(Barrier::new(2));
        let (g1, g2) = (Arc::clone(&gate), Arc::clone(&gate));

        let t1 = std::thread::spawn(move || {
            g1.wait();
            a.update(&mut |settings: &mut Settings| {
                settings.theme = micold_core::theme::ThemePreference::Dark;
            })
        });
        let t2 = std::thread::spawn(move || {
            g2.wait();
            b.update(&mut |settings: &mut Settings| {
                settings.scrollback_lines = 12_345;
            })
        });
        t1.join().unwrap().unwrap();
        t2.join().unwrap().unwrap();

        let settings = JsonFileSettingsStore::at(path.clone()).load().settings;
        assert_eq!(
            settings.theme,
            micold_core::theme::ThemePreference::Dark,
            "round {round}: the theme writer's change was written away by the other writer"
        );
        assert_eq!(
            settings.scrollback_lines, 12_345,
            "round {round}: the scrollback writer's change was written away by the other writer"
        );
    }
}
