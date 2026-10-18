//! Contract tests for the pure `keymap::encode` (feature 006).
//! Runs under `cargo test --no-default-features` — no GUI. See
//! `specs/006-real-terminal-emulator/contracts/key-encoding.md`.

use micold_client::keymap::{
    encode, is_switcher_chord, nav_chord_labels, paste_bytes, switcher_chord_labels, Key, KeyInput,
    KeyOutput, Mods, NamedKey, NavAction, NavLevel, PaneAction, PhysicalKey, TermMode, NAV_CHORDS,
};
use micold_core::navigation::Dir;
use micold_core::pane_layout::Direction;

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
        physical: PhysicalKey::Other,
    }
}

/// [`ki`] with the physical key the platform reported for the press.
fn kp(key: Key, physical: PhysicalKey, mods: Mods) -> KeyInput {
    KeyInput {
        physical,
        ..ki(key, mods, None)
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

// ---- Open-new-terminal-instance chord (feature 011, FR-019) ----

#[test]
fn new_terminal_instance_chord_is_detected() {
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
    let out = encode(&ki(Key::Char('t'), mods, Some("t")), TermMode::default());
    assert_eq!(out, KeyOutput::NewTerminalInstance);

    // Case-insensitive on the letter itself (matches the existing release-focus chord's rule).
    let out = encode(&ki(Key::Char('T'), mods, Some("T")), TermMode::default());
    assert_eq!(out, KeyOutput::NewTerminalInstance);
}

#[test]
fn new_terminal_instance_chord_does_not_fire_on_plain_t_or_shift_t() {
    let out = encode(
        &ki(Key::Char('t'), Mods::NONE, Some("t")),
        TermMode::default(),
    );
    assert_eq!(bytes(out), b"t");

    let shift_only = Mods {
        shift: true,
        ..Mods::NONE
    };
    let out = encode(
        &ki(Key::Char('t'), shift_only, Some("T")),
        TermMode::default(),
    );
    assert_eq!(bytes(out), b"T");
}

#[test]
fn new_terminal_instance_chord_takes_precedence_over_printable_handling() {
    // On non-macOS, Ctrl+Shift+T without the chord check would otherwise fall through to a
    // control-chord/printable path; confirm the chord wins outright.
    #[cfg(not(target_os = "macos"))]
    {
        let mods = Mods {
            ctrl: true,
            shift: true,
            ..Mods::NONE
        };
        let out = encode(&ki(Key::Char('t'), mods, None), TermMode::default());
        assert_eq!(out, KeyOutput::NewTerminalInstance);
    }
}

// ---- Overview chord (feature 813, FR-011, SC-006) ----

/// The platform's primary modifier plus Shift (`Cmd+Shift` on macOS, else `Ctrl+Shift`).
fn primary_shift() -> Mods {
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
    mods
}

#[test]
fn overview_chord_encodes_to_toggle_overview() {
    for (c, text) in [('o', "o"), ('O', "O")] {
        let out = encode(
            &ki(Key::Char(c), primary_shift(), Some(text)),
            TermMode::default(),
        );
        assert_eq!(out, KeyOutput::ToggleOverview);
        assert!(micold_client::keymap::is_overview_chord(
            &Key::Char(c),
            primary_shift()
        ));
    }
}

#[test]
fn overview_chord_needs_shift_and_no_alt() {
    let shift_only = Mods {
        shift: true,
        ..Mods::NONE
    };
    let with_alt = Mods {
        alt: true,
        ..primary_shift()
    };
    for mods in [Mods::NONE, shift_only, with_alt] {
        let out = encode(&ki(Key::Char('o'), mods, Some("o")), TermMode::default());
        assert_ne!(out, KeyOutput::ToggleOverview, "{mods:?}");
        assert!(!micold_client::keymap::is_overview_chord(
            &Key::Char('o'),
            mods
        ));
    }
    // Ctrl+O without Shift is still the control byte.
    let ctrl = Mods {
        ctrl: true,
        ..Mods::NONE
    };
    #[cfg(not(target_os = "macos"))]
    {
        let out = encode(&ki(Key::Char('o'), ctrl, None), TermMode::default());
        assert_eq!(bytes(out), vec![0x0f]);
    }
    assert!(!micold_client::keymap::is_overview_chord(
        &Key::Char('o'),
        ctrl
    ));
}

#[test]
fn route_key_sends_the_overview_chord_to_the_toggle_focused_or_not() {
    use micold_client::app::{route_key, KeyRouting};
    for focused in [true, false] {
        assert_eq!(
            route_key(focused, KeyOutput::ToggleOverview),
            KeyRouting::ToggleOverview
        );
    }
}

#[test]
fn overview_chord_label_names_the_platform_chord() {
    #[cfg(target_os = "macos")]
    assert_eq!(micold_client::keymap::overview_chord_label(), "Cmd+Shift+O");
    #[cfg(not(target_os = "macos"))]
    assert_eq!(
        micold_client::keymap::overview_chord_label(),
        "Ctrl+Shift+O"
    );
}

/// Feature 970 (FR-001, FR-014): the Overview's full-screen chord is F with the platform's primary
/// modifier and Shift, and nothing else.
#[test]
fn is_overview_full_screen_chord_is_primary_shift_f_only() {
    use micold_client::keymap::is_overview_full_screen_chord;
    for c in ['f', 'F'] {
        assert!(is_overview_full_screen_chord(&Key::Char(c), primary_shift()));
    }
    let shift_only = Mods {
        shift: true,
        ..Mods::NONE
    };
    let primary_only = Mods {
        shift: false,
        ..primary_shift()
    };
    let with_alt = Mods {
        alt: true,
        ..primary_shift()
    };
    for mods in [Mods::NONE, shift_only, primary_only, with_alt] {
        assert!(
            !is_overview_full_screen_chord(&Key::Char('f'), mods),
            "{mods:?}"
        );
    }
    assert!(!is_overview_full_screen_chord(
        &Key::Char('g'),
        primary_shift()
    ));
}

#[test]
fn overview_full_screen_chord_label_names_the_platform_chord() {
    #[cfg(target_os = "macos")]
    assert_eq!(
        micold_client::keymap::overview_full_screen_chord_label(),
        "Cmd+Shift+F"
    );
    #[cfg(not(target_os = "macos"))]
    assert_eq!(
        micold_client::keymap::overview_full_screen_chord_label(),
        "Ctrl+Shift+F"
    );
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

// --- Paste bytes and bracketed paste (FR-013d, BUG-006) ---

const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";

fn bracketed(body: &str) -> Vec<u8> {
    [PASTE_START, body.as_bytes(), PASTE_END].concat()
}

#[test]
fn a_paste_without_bracketed_paste_mode_is_the_text_unchanged() {
    // Not even an embedded marker is touched: a process that never asked for bracketing has no
    // block for it to close, and the user pasted exactly these bytes.
    for text in ["echo hi", "echo AAA\necho BBB\n", "a\x1b[201~b", ""] {
        assert_eq!(paste_bytes(text, false), text.as_bytes());
    }
}

#[test]
fn a_paste_in_bracketed_paste_mode_is_wrapped_in_start_and_end_markers() {
    assert_eq!(
        paste_bytes("echo AAA\necho BBB\n", true),
        bracketed("echo AAA\necho BBB\n"),
        "a multi-line paste must arrive as one block, not as a sequence of Enters (FR-013d)"
    );
}

#[test]
fn an_end_marker_inside_the_pasted_text_is_removed() {
    assert_eq!(
        paste_bytes("safe\x1b[201~rm -rf ~\n", true),
        bracketed("saferm -rf ~\n"),
        "an embedded end marker would close the block and run the rest as keystrokes (FR-013d)"
    );
}

#[test]
fn removing_an_end_marker_cannot_assemble_another_one() {
    // One pass over `ESC[20` `ESC[201~` `1~` leaves an `ESC[201~` behind; the removal has to run
    // until none is left, or the smuggled marker still closes the block early.
    assert_eq!(paste_bytes("x\x1b[20\x1b[201~1~y", true), bracketed("xy"));
}

// ---- Pane chords (feature 484, FR-009) ----

/// The platform modifier of the app's reserved chords, with Shift.
fn chord_mods() -> Mods {
    Mods {
        shift: true,
        ctrl: cfg!(not(target_os = "macos")),
        logo: cfg!(target_os = "macos"),
        ..Mods::NONE
    }
}

fn pane_chords() -> Vec<(Key, PaneAction)> {
    vec![
        (Key::Char('d'), PaneAction::SplitVertical),
        (Key::Char('D'), PaneAction::SplitVertical),
        (Key::Char('h'), PaneAction::SplitHorizontal),
        (Key::Char('w'), PaneAction::Close),
        (Key::Char('W'), PaneAction::Close),
        (
            Key::Named(NamedKey::ArrowLeft),
            PaneAction::Focus(Direction::Left),
        ),
        (
            Key::Named(NamedKey::ArrowRight),
            PaneAction::Focus(Direction::Right),
        ),
        (
            Key::Named(NamedKey::ArrowUp),
            PaneAction::Focus(Direction::Up),
        ),
        (
            Key::Named(NamedKey::ArrowDown),
            PaneAction::Focus(Direction::Down),
        ),
    ]
}

#[test]
fn pane_chords_are_app_actions_never_bytes() {
    for (key, action) in pane_chords() {
        let out = encode(&ki(key.clone(), chord_mods(), None), TermMode::default());
        assert_eq!(out, KeyOutput::Pane(action), "{key:?}");
        // Also with the text iced resolves for the press, and in application cursor mode.
        let out = encode(
            &ki(key, chord_mods(), Some("\u{4}")),
            TermMode {
                app_cursor: true,
                alt_screen: true,
            },
        );
        assert!(matches!(out, KeyOutput::Pane(_)));
    }
}

/// No chord the key map forwards to the process (every Ctrl/Alt/plain letter, digit and named
/// key, with or without Ctrl/Alt) equals a pane chord. All pane chords need Shift plus the
/// platform modifier, so the existing reserved chords and plain forwarded input are unchanged.
#[test]
fn no_forwarded_chord_is_a_pane_chord() {
    let mut keys: Vec<Key> = ('a'..='z').chain('0'..='9').map(Key::Char).collect();
    keys.extend(
        [
            NamedKey::Enter,
            NamedKey::Tab,
            NamedKey::Escape,
            NamedKey::Space,
            NamedKey::Home,
            NamedKey::End,
            NamedKey::PageUp,
            NamedKey::PageDown,
            NamedKey::ArrowUp,
            NamedKey::ArrowDown,
            NamedKey::ArrowLeft,
            NamedKey::ArrowRight,
        ]
        .map(Key::Named),
    );
    let mods = [
        Mods::NONE,
        ctrl(),
        Mods {
            alt: true,
            ..Mods::NONE
        },
        Mods {
            shift: true,
            ..Mods::NONE
        },
        Mods {
            ctrl: true,
            alt: true,
            ..Mods::NONE
        },
    ];
    for key in &keys {
        for m in mods {
            let out = encode(&ki(key.clone(), m, None), TermMode::default());
            assert!(!matches!(out, KeyOutput::Pane(_)), "{key:?} {m:?}");
        }
        // With the pane modifier, only the pane chords' own keys are pane actions.
        let out = encode(&ki(key.clone(), chord_mods(), None), TermMode::default());
        let is_pane_key = pane_chords().iter().any(|(k, _)| k == key);
        assert_eq!(matches!(out, KeyOutput::Pane(_)), is_pane_key, "{key:?}");
    }
}

#[test]
fn existing_reserved_chords_are_unchanged() {
    for (c, want) in [
        ('e', KeyOutput::ReleaseFocus),
        ('t', KeyOutput::NewTerminalInstance),
    ] {
        assert_eq!(
            encode(&ki(Key::Char(c), chord_mods(), None), TermMode::default()),
            want
        );
    }
    #[cfg(not(target_os = "macos"))]
    for (c, want) in [('c', KeyOutput::Copy), ('v', KeyOutput::Paste)] {
        assert_eq!(
            encode(&ki(Key::Char(c), chord_mods(), None), TermMode::default()),
            want
        );
    }
}

#[test]
fn the_wrong_platform_modifier_is_not_a_pane_chord() {
    let wrong = Mods {
        shift: true,
        ctrl: cfg!(target_os = "macos"),
        logo: cfg!(not(target_os = "macos")),
        ..Mods::NONE
    };
    let out = encode(&ki(Key::Char('d'), wrong, None), TermMode::default());
    assert!(!matches!(out, KeyOutput::Pane(_)));
}

// ---- Navigation chords (feature 803, contracts/keys.md K2, K5) ----

const LOCATION_PREVIOUS: NavAction = NavAction {
    level: NavLevel::Location,
    dir: Dir::Previous,
};
const LOCATION_NEXT: NavAction = NavAction {
    level: NavLevel::Location,
    dir: Dir::Next,
};

const SESSION_PREVIOUS: NavAction = NavAction {
    level: NavLevel::Session,
    dir: Dir::Previous,
};
const SESSION_NEXT: NavAction = NavAction {
    level: NavLevel::Session,
    dir: Dir::Next,
};

/// The location and session chords: their physical key, the logical keys a layout may give for it,
/// and the action.
fn location_chords() -> [(PhysicalKey, [char; 2], NavAction); 4] {
    [
        (PhysicalKey::Comma, [',', '<'], LOCATION_PREVIOUS),
        (PhysicalKey::Period, ['.', '>'], LOCATION_NEXT),
        (PhysicalKey::BracketLeft, ['[', '{'], SESSION_PREVIOUS),
        (PhysicalKey::BracketRight, [']', '}'], SESSION_NEXT),
    ]
}

#[test]
fn plain_ctrl_brackets_still_reach_the_terminal() {
    // FR-008: without Shift the readline bytes (ESC and GS) are typed, not taken by the app.
    let primary_only = Mods {
        shift: false,
        ..chord_mods()
    };
    for (c, byte) in [('[', 0x1b), (']', 0x1d)] {
        let physical = if c == '[' {
            PhysicalKey::BracketLeft
        } else {
            PhysicalKey::BracketRight
        };
        let out = encode(
            &kp(Key::Char(c), physical, primary_only),
            TermMode::default(),
        );
        assert_eq!(bytes(out), vec![byte], "Ctrl+{c} keeps its control byte");
    }
}

#[test]
fn location_chords_decode_by_physical_key_whatever_the_layout_gives() {
    for (physical, _, action) in location_chords() {
        // A layout that puts another character on the key (FR-016) still decodes by position.
        for logical in [Key::Char('x'), Key::Char(';'), Key::Char('ж')] {
            let out = encode(
                &kp(logical.clone(), physical, chord_mods()),
                TermMode::default(),
            );
            assert_eq!(
                out,
                KeyOutput::Navigate(action),
                "the {physical:?} key with primary+Shift is {action:?} whatever it types ({logical:?})"
            );
        }
    }
}

#[test]
fn location_chords_fall_back_to_the_logical_key_without_a_physical_one() {
    for (_, logical, action) in location_chords() {
        for c in logical {
            let out = encode(&ki(Key::Char(c), chord_mods(), None), TermMode::default());
            assert_eq!(
                out,
                KeyOutput::Navigate(action),
                "with no physical key, {c:?} with primary+Shift is {action:?}"
            );
        }
    }
}

#[test]
fn near_misses_of_a_location_chord_keep_their_outputs() {
    let shift_only = Mods {
        shift: true,
        ..Mods::NONE
    };
    let with_alt = Mods {
        alt: true,
        ..chord_mods()
    };
    let primary_only = Mods {
        shift: false,
        ..chord_mods()
    };
    for (physical, [plain, shifted], _) in location_chords() {
        for mods in [Mods::NONE, shift_only, with_alt, primary_only] {
            let c = if mods.shift { shifted } else { plain };
            let out = encode(&kp(Key::Char(c), physical, mods), TermMode::default());
            assert!(
                !matches!(out, KeyOutput::Navigate(_)),
                "{c:?} with {mods:?} is not a navigation chord, got {out:?}"
            );
        }
        // Typed without a modifier the key still writes its character.
        let out = encode(
            &kp(Key::Char(plain), physical, Mods::NONE),
            TermMode::default(),
        );
        assert_eq!(
            bytes(out),
            plain.to_string().into_bytes(),
            "plain {plain:?} is typed"
        );
    }
}

#[test]
fn the_other_primary_shift_chords_are_unchanged_by_the_navigation_chords() {
    for (c, want) in [
        ('e', KeyOutput::ReleaseFocus),
        ('t', KeyOutput::NewTerminalInstance),
        ('o', KeyOutput::ToggleOverview),
    ] {
        let out = encode(&ki(Key::Char(c), chord_mods(), None), TermMode::default());
        assert_eq!(out, want, "primary+Shift+{c} keeps its output");
    }
    for (key, action) in pane_chords() {
        let out = encode(&ki(key.clone(), chord_mods(), None), TermMode::default());
        assert_eq!(
            out,
            KeyOutput::Pane(action),
            "{key:?} keeps its pane action"
        );
    }
    #[cfg(not(target_os = "macos"))]
    for (c, want) in [('c', KeyOutput::Copy), ('v', KeyOutput::Paste)] {
        let out = encode(&ki(Key::Char(c), chord_mods(), None), TermMode::default());
        assert_eq!(out, want, "Ctrl+Shift+{c} keeps copy/paste");
    }
}

#[test]
fn no_two_navigation_chords_share_a_key() {
    for (i, (_, a)) in NAV_CHORDS.iter().enumerate() {
        for (_, b) in &NAV_CHORDS[i + 1..] {
            assert_ne!(a, b, "two navigation chords on the {a:?} key");
        }
    }
}

#[test]
fn no_navigation_chord_is_a_reserved_chord_of_the_app() {
    // R9: the navigation keys are not the letters, arrows or punctuation of an existing chord.
    for (action, physical) in NAV_CHORDS {
        let out = encode(
            &kp(Key::Char('x'), physical, chord_mods()),
            TermMode::default(),
        );
        assert_eq!(
            out,
            KeyOutput::Navigate(action),
            "{physical:?} decodes to its own action"
        );
        for (key, _) in pane_chords() {
            let out = encode(&ki(key.clone(), chord_mods(), None), TermMode::default());
            assert_ne!(out, KeyOutput::Navigate(action), "{key:?} is a pane chord");
        }
    }
}

#[test]
fn route_key_sends_a_navigation_chord_to_the_app_focused_or_not() {
    use micold_client::app::{route_key, KeyRouting};
    for (action, _) in NAV_CHORDS {
        for focused in [true, false] {
            assert_eq!(
                route_key(focused, KeyOutput::Navigate(action)),
                KeyRouting::Navigate(action),
                "{action:?} is the app's whether a terminal is focused ({focused}) or not"
            );
        }
    }
}

#[test]
fn navigation_chord_labels_name_both_platform_forms() {
    assert_eq!(
        nav_chord_labels(LOCATION_PREVIOUS),
        ("Ctrl+Shift+,".to_string(), "Cmd+Shift+,".to_string())
    );
    assert_eq!(
        nav_chord_labels(LOCATION_NEXT),
        ("Ctrl+Shift+.".to_string(), "Cmd+Shift+.".to_string())
    );
    assert_eq!(
        nav_chord_labels(SESSION_PREVIOUS),
        ("Ctrl+Shift+[".to_string(), "Cmd+Shift+[".to_string())
    );
    assert_eq!(
        nav_chord_labels(SESSION_NEXT),
        ("Ctrl+Shift+]".to_string(), "Cmd+Shift+]".to_string())
    );
}

#[test]
fn the_user_guide_lists_every_navigation_chord_in_both_forms() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/user-guide");
    for file in ["terminal-panes.md", "worktrees-and-sessions.md"] {
        let text = std::fs::read_to_string(root.join(file)).expect("the user guide page reads");
        for (action, _) in NAV_CHORDS {
            let (linux, mac) = nav_chord_labels(action);
            for label in [linux, mac] {
                assert!(
                    text.contains(&label),
                    "{file} lists {label} ({action:?}) — FR-013"
                );
            }
        }
    }
}

// ---- Project chords (feature 803, T030) ----

const PROJECT_PREVIOUS: NavAction = NavAction {
    level: NavLevel::Project,
    dir: Dir::Previous,
};
const PROJECT_NEXT: NavAction = NavAction {
    level: NavLevel::Project,
    dir: Dir::Next,
};

fn project_chords() -> [(PhysicalKey, NamedKey, NavAction); 2] {
    [
        (PhysicalKey::PageUp, NamedKey::PageUp, PROJECT_PREVIOUS),
        (PhysicalKey::PageDown, NamedKey::PageDown, PROJECT_NEXT),
    ]
}

#[test]
fn project_chords_decode_by_physical_key_and_by_the_named_key_fallback() {
    for (physical, named, action) in project_chords() {
        let out = encode(
            &kp(Key::Named(named), physical, chord_mods()),
            TermMode::default(),
        );
        assert_eq!(out, KeyOutput::Navigate(action), "{physical:?}");
        let out = encode(
            &kp(Key::Named(named), PhysicalKey::Other, chord_mods()),
            TermMode::default(),
        );
        assert_eq!(out, KeyOutput::Navigate(action), "fallback {named:?}");
    }
}

#[test]
fn page_keys_without_shift_or_the_primary_modifier_keep_their_outputs() {
    let shift_only = Mods {
        shift: true,
        ..Mods::NONE
    };
    let primary_only = Mods {
        shift: false,
        ..chord_mods()
    };
    for (physical, named, _) in project_chords() {
        for mods in [Mods::NONE, shift_only, primary_only] {
            let out = encode(&kp(Key::Named(named), physical, mods), TermMode::default());
            assert!(
                !matches!(out, KeyOutput::Navigate(_)),
                "{named:?} with {mods:?} is not a navigation chord, got {out:?}"
            );
        }
    }
}

// ---- The switcher chord (feature 968, T005; contracts/keys.md K5) ----

#[test]
fn the_switcher_chord_decodes_by_physical_key_whatever_the_layout_gives() {
    for (key, physical) in [
        (Key::Char('p'), PhysicalKey::KeyP),
        (Key::Char('P'), PhysicalKey::KeyP),
        // Dvorak: the key at P's position types `l`.
        (Key::Char('l'), PhysicalKey::KeyP),
        (Key::Char('L'), PhysicalKey::KeyP),
        // No physical code: the logical fallback.
        (Key::Char('p'), PhysicalKey::Other),
        (Key::Char('P'), PhysicalKey::Other),
    ] {
        let input = kp(key.clone(), physical, primary_shift());
        assert!(is_switcher_chord(&input), "{key:?} on {physical:?}");
        assert_eq!(
            encode(&input, TermMode::default()),
            KeyOutput::ToggleSwitcher,
            "{key:?} on {physical:?}"
        );
    }
}

#[test]
fn near_misses_of_the_switcher_chord_keep_their_outputs() {
    let mode = TermMode::default();
    let with_text = |key, mods, text| KeyInput {
        physical: PhysicalKey::KeyP,
        ..ki(key, mods, Some(text))
    };
    assert_eq!(
        encode(&kp(Key::Char('p'), PhysicalKey::KeyP, ctrl()), mode),
        KeyOutput::Bytes(vec![0x10])
    );
    let shift = Mods {
        shift: true,
        ..Mods::NONE
    };
    assert_eq!(
        encode(&with_text(Key::Char('P'), shift, "P"), mode),
        KeyOutput::Bytes(b"P".to_vec())
    );
    let alt_shift = Mods {
        alt: true,
        shift: true,
        ..Mods::NONE
    };
    assert_eq!(
        encode(&kp(Key::Char('P'), PhysicalKey::KeyP, alt_shift), mode),
        KeyOutput::Bytes(vec![0x1b, b'P'])
    );
    let primary_alt_shift = Mods {
        alt: true,
        ..primary_shift()
    };
    let input = kp(Key::Char('P'), PhysicalKey::KeyP, primary_alt_shift);
    assert!(!is_switcher_chord(&input));
    assert_ne!(encode(&input, mode), KeyOutput::ToggleSwitcher);
    // Without Shift it is not the chord either.
    let primary_only = Mods {
        shift: false,
        ..primary_shift()
    };
    assert!(!is_switcher_chord(&kp(
        Key::Char('p'),
        PhysicalKey::KeyP,
        primary_only
    )));
}

#[test]
fn no_other_reserved_or_navigation_chord_decodes_to_the_switcher() {
    let mode = TermMode::default();
    for c in ['e', 't', 'o', 'd', 'h', 'w', 'c', 'v', 'E', 'T', 'O'] {
        let out = encode(&ki(Key::Char(c), primary_shift(), None), mode);
        assert_ne!(out, KeyOutput::ToggleSwitcher, "{c:?}");
    }
    for named in [
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::PageUp,
        NamedKey::PageDown,
    ] {
        let out = encode(&ki(Key::Named(named), primary_shift(), None), mode);
        assert_ne!(out, KeyOutput::ToggleSwitcher, "{named:?}");
    }
    for (action, physical) in NAV_CHORDS {
        let input = kp(Key::Char('x'), physical, primary_shift());
        assert!(!is_switcher_chord(&input), "{action:?}");
        assert_ne!(
            encode(&input, mode),
            KeyOutput::ToggleSwitcher,
            "{action:?}"
        );
    }
}

#[test]
fn route_key_sends_the_switcher_chord_to_the_app_focused_or_not() {
    use micold_client::app::{route_key, KeyRouting};
    for focused in [true, false] {
        assert_eq!(
            route_key(focused, KeyOutput::ToggleSwitcher),
            KeyRouting::ToggleSwitcher
        );
    }
}

#[test]
fn the_switcher_chord_labels_name_both_platform_forms() {
    assert_eq!(
        switcher_chord_labels(),
        ("Ctrl+Shift+P".to_string(), "Cmd+Shift+P".to_string())
    );
}

#[test]
fn the_user_guide_lists_the_switcher_chord_in_both_forms() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/user-guide");
    let (linux, mac) = switcher_chord_labels();
    for file in ["terminal-panes.md", "worktrees-and-sessions.md"] {
        let text = std::fs::read_to_string(root.join(file)).expect("the user guide page reads");
        for label in [&linux, &mac] {
            assert!(
                text.contains(label),
                "{file} lists {label} (FR-017, SC-006)"
            );
        }
    }
}
