//! Contract tests for the pure `keymap::encode` (feature 006).
//! Runs under `cargo test --no-default-features` — no GUI. See
//! `specs/006-real-terminal-emulator/contracts/key-encoding.md`.

use micold_ai_ide::keymap::{encode, Key, KeyInput, KeyOutput, Mods, NamedKey, TermMode};

fn ctrl() -> Mods {
    Mods {
        ctrl: true,
        ..Mods::NONE
    }
}

fn ki(key: Key, mods: Mods, text: Option<&str>) -> KeyInput {
    KeyInput {
        key,
        mods,
        text: text.map(str::to_string),
    }
}

fn bytes(o: KeyOutput) -> Vec<u8> {
    match o {
        KeyOutput::Bytes(b) => b,
        other => panic!("expected Bytes, got {other:?}"),
    }
}

// ---- Named-key base encodings (no modifiers) ----

#[test]
fn named_base_encodings() {
    let m = TermMode::default();
    let cases: &[(NamedKey, &[u8])] = &[
        (NamedKey::Enter, b"\r"),
        (NamedKey::Backspace, b"\x7f"),
        (NamedKey::Tab, b"\t"),
        (NamedKey::Escape, b"\x1b"),
        (NamedKey::Space, b" "),
        (NamedKey::Insert, b"\x1b[2~"),
        (NamedKey::Delete, b"\x1b[3~"),
        (NamedKey::PageUp, b"\x1b[5~"),
        (NamedKey::PageDown, b"\x1b[6~"),
        (NamedKey::Home, b"\x1b[H"),
        (NamedKey::End, b"\x1b[F"),
        (NamedKey::ArrowUp, b"\x1b[A"),
        (NamedKey::ArrowDown, b"\x1b[B"),
        (NamedKey::ArrowRight, b"\x1b[C"),
        (NamedKey::ArrowLeft, b"\x1b[D"),
        (NamedKey::F(1), b"\x1bOP"),
        (NamedKey::F(5), b"\x1b[15~"),
        (NamedKey::F(12), b"\x1b[24~"),
    ];
    for (named, expected) in cases {
        let out = encode(&ki(Key::Named(*named), Mods::NONE, None), m);
        assert_eq!(bytes(out), *expected, "named {named:?}");
    }
}

#[test]
fn arrows_and_home_end_in_app_cursor_mode() {
    let m = TermMode {
        app_cursor: true,
        alt_screen: false,
    };
    let cases: &[(NamedKey, &[u8])] = &[
        (NamedKey::ArrowUp, b"\x1bOA"),
        (NamedKey::ArrowDown, b"\x1bOB"),
        (NamedKey::ArrowRight, b"\x1bOC"),
        (NamedKey::ArrowLeft, b"\x1bOD"),
        (NamedKey::Home, b"\x1bOH"),
        (NamedKey::End, b"\x1bOF"),
    ];
    for (named, expected) in cases {
        let out = encode(&ki(Key::Named(*named), Mods::NONE, None), m);
        assert_eq!(bytes(out), *expected, "app-cursor {named:?}");
    }
}

#[test]
fn shift_tab_is_back_tab() {
    let out = encode(
        &ki(
            Key::Named(NamedKey::Tab),
            Mods {
                shift: true,
                ..Mods::NONE
            },
            None,
        ),
        TermMode::default(),
    );
    assert_eq!(bytes(out), b"\x1b[Z");
}

// ---- Control chords: Ctrl+a..z → 0x01..0x1a, incl. the Ctrl+U == 0x15 regression ----

#[test]
fn ctrl_letters_span_0x01_to_0x1a() {
    for (i, c) in ('a'..='z').enumerate() {
        let out = encode(&ki(Key::Char(c), ctrl(), None), TermMode::default());
        assert_eq!(bytes(out), vec![0x01 + i as u8], "Ctrl+{c}");
    }
}

#[test]
fn ctrl_c_and_ctrl_d() {
    assert_eq!(
        bytes(encode(
            &ki(Key::Char('c'), ctrl(), None),
            TermMode::default()
        )),
        vec![0x03]
    );
    assert_eq!(
        bytes(encode(
            &ki(Key::Char('d'), ctrl(), None),
            TermMode::default()
        )),
        vec![0x04]
    );
}

#[test]
fn ctrl_u_is_0x15_not_0x51() {
    // Regression against iced_term's bindings.rs bug (mapped Ctrl+U to 0x51).
    let out = encode(&ki(Key::Char('u'), ctrl(), None), TermMode::default());
    assert_eq!(bytes(out), vec![0x15]);
}

// ---- Reserved focus-out chord ----

#[test]
fn reserved_focus_out_chord_releases_focus() {
    #[cfg(target_os = "macos")]
    let mods = Mods {
        logo: true,
        shift: true,
        ..Mods::NONE
    };
    #[cfg(not(target_os = "macos"))]
    let mods = Mods {
        ctrl: true,
        shift: true,
        ..Mods::NONE
    };
    let out = encode(&ki(Key::Char('e'), mods, Some("e")), TermMode::default());
    assert_eq!(out, KeyOutput::ReleaseFocus);
}

// ---- Copy / paste chords ----

#[test]
fn copy_paste_chords() {
    #[cfg(target_os = "macos")]
    let mods = Mods {
        logo: true,
        ..Mods::NONE
    };
    #[cfg(not(target_os = "macos"))]
    let mods = Mods {
        ctrl: true,
        shift: true,
        ..Mods::NONE
    };
    assert_eq!(
        encode(&ki(Key::Char('c'), mods, None), TermMode::default()),
        KeyOutput::Copy
    );
    assert_eq!(
        encode(&ki(Key::Char('v'), mods, None), TermMode::default()),
        KeyOutput::Paste
    );
}

// ---- Printable & ignore ----

#[test]
fn printable_char_with_text_writes_bytes() {
    let out = encode(
        &ki(Key::Char('a'), Mods::NONE, Some("a")),
        TermMode::default(),
    );
    assert_eq!(bytes(out), b"a");
}

#[test]
fn plain_char_without_text_still_writes_itself() {
    let out = encode(&ki(Key::Char('x'), Mods::NONE, None), TermMode::default());
    assert_eq!(bytes(out), b"x");
}

#[test]
fn logo_only_char_is_ignored() {
    // A bare Super/Command+char with no mapping is not terminal input.
    let out = encode(
        &ki(
            Key::Char('q'),
            Mods {
                logo: true,
                ..Mods::NONE
            },
            Some("q"),
        ),
        TermMode::default(),
    );
    assert_eq!(out, KeyOutput::Ignore);
}

// ---- Totality: never panics across a broad input space ----

#[test]
fn encode_is_total() {
    let modsets = [
        Mods::NONE,
        Mods {
            shift: true,
            ..Mods::NONE
        },
        Mods {
            ctrl: true,
            ..Mods::NONE
        },
        Mods {
            alt: true,
            ..Mods::NONE
        },
        Mods {
            logo: true,
            ..Mods::NONE
        },
        Mods {
            ctrl: true,
            shift: true,
            ..Mods::NONE
        },
    ];
    let named = [
        NamedKey::Enter,
        NamedKey::Backspace,
        NamedKey::Tab,
        NamedKey::Escape,
        NamedKey::Insert,
        NamedKey::Delete,
        NamedKey::Home,
        NamedKey::End,
        NamedKey::PageUp,
        NamedKey::PageDown,
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::F(1),
        NamedKey::F(20),
    ];
    for mode in [
        TermMode::default(),
        TermMode {
            app_cursor: true,
            alt_screen: true,
        },
    ] {
        for m in modsets {
            for c in ['a', 'Z', '1', ' ', '-', '[', '\\'] {
                let _ = encode(&ki(Key::Char(c), m, Some(&c.to_string())), mode);
            }
            for n in named {
                let _ = encode(&ki(Key::Named(n), m, None), mode);
            }
        }
    }
}
