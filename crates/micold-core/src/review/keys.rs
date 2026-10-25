//! The Review tab's key map (#1274): the one table of review keys, [`REVIEW_KEYS`], and how a key
//! press is decided against it, [`resolve`]. The client builds a [`Press`] and says who holds the
//! keyboard ([`Holder`]); it never matches a key by hand, so the `?` sheet, tooltips, the user
//! guide (#1276) and the bindings all read this table. The single-key switch (FR-022) is a plain
//! `single_keys` argument: [`resolve`], [`sheet_rows`] and [`key_hint`] filter on it and the table
//! never changes. Contract: `specs/1274-review-keyboard-map/contracts/keys.md` (K1–K4a).

/// What a review key does (FR-001, FR-010): the closed set the Review tab's reducer acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewAction {
    /// `j`: select the next file in file order (FR-003).
    NextFile,
    /// `k`: select the previous file (FR-003).
    PrevFile,
    /// `n`: make the next hunk current, crossing into the next file with hunks (FR-004).
    NextHunk,
    /// `p`: make the previous hunk current (FR-004).
    PrevHunk,
    /// `]`: show the next comment in comment order (FR-006).
    NextComment,
    /// `[`: show the previous comment (FR-006).
    PrevComment,
    /// `v`: toggle the selected file's reviewed mark (FR-007).
    ToggleReviewed,
    /// `g`: select the next file not marked reviewed (FR-008).
    NextUnreviewed,
    /// `c`: open the comment composer (FR-009).
    Comment,
    /// `?`: open or close the shortcut sheet (FR-012).
    ToggleSheet,
    /// The send chord: send the review as **Send review (n)** does (FR-010).
    Send,
}

/// The key a table row is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyName {
    /// The character the key produces (`?` already includes its Shift).
    Char(char),
    /// The Enter key.
    Enter,
}

/// The modifiers a table row asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyMods {
    /// The platform's primary modifier: Cmd on macOS, Ctrl elsewhere.
    pub primary: bool,
    /// Shift; compared only for `primary` rows, since a char already includes it (research R2).
    pub shift: bool,
}

/// One row of [`REVIEW_KEYS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewKey {
    /// The key.
    pub key: KeyName,
    /// Its modifiers.
    pub mods: KeyMods,
    /// What it does; `None` only for the save-comment chord, which the composer owns and the
    /// sheet only lists.
    pub action: Option<ReviewAction>,
    /// Whether an auto-repeat acts again (`j k n p ] [`, FR-015).
    pub repeats: bool,
    /// The `?` sheet's text for the row.
    pub sheet: &'static str,
}

impl ReviewKey {
    /// Whether the row is a single key (no primary modifier), which the single-key switch turns off
    /// (FR-022). The send and save-comment chords are not.
    pub fn is_single(&self) -> bool {
        !self.mods.primary
    }

    /// The row's key as `(Linux/Windows form, macOS form)`, as the sheet, tooltips and user guide
    /// write it (contract K4).
    pub fn labels(&self) -> (String, String) {
        let key = match self.key {
            KeyName::Char(c) => c.to_string(),
            KeyName::Enter => "Enter".to_owned(),
        };
        if !self.mods.primary {
            return (key.clone(), key);
        }
        let shift = if self.mods.shift { "Shift+" } else { "" };
        (format!("Ctrl+{shift}{key}"), format!("Cmd+{shift}{key}"))
    }
}

const fn single(c: char, action: ReviewAction, repeats: bool, sheet: &'static str) -> ReviewKey {
    ReviewKey {
        key: KeyName::Char(c),
        mods: KeyMods {
            primary: false,
            shift: false,
        },
        action: Some(action),
        repeats,
        sheet,
    }
}

const fn chord(shift: bool, action: Option<ReviewAction>, sheet: &'static str) -> ReviewKey {
    ReviewKey {
        key: KeyName::Enter,
        mods: KeyMods {
            primary: true,
            shift,
        },
        action,
        repeats: false,
        sheet,
    }
}

/// Every review key, in the `?` sheet's order (contract K1). The one place review keys are
/// chosen: [`resolve`] matches only from it, the client's sheet and tooltips render it, and #1276's
/// user-guide test compares the guide against it. **Public and stable**: changing a row is a
/// user-facing change.
pub const REVIEW_KEYS: [ReviewKey; 12] = [
    single('j', ReviewAction::NextFile, true, "Next file"),
    single('k', ReviewAction::PrevFile, true, "Previous file"),
    single('n', ReviewAction::NextHunk, true, "Next change"),
    single('p', ReviewAction::PrevHunk, true, "Previous change"),
    single(']', ReviewAction::NextComment, true, "Next comment"),
    single('[', ReviewAction::PrevComment, true, "Previous comment"),
    single(
        'v',
        ReviewAction::ToggleReviewed,
        false,
        "Mark file reviewed / not reviewed",
    ),
    single(
        'g',
        ReviewAction::NextUnreviewed,
        false,
        "Next file not reviewed",
    ),
    single(
        'c',
        ReviewAction::Comment,
        false,
        "Comment on the picked lines or the current change",
    ),
    single(
        '?',
        ReviewAction::ToggleSheet,
        false,
        "Show or hide these shortcuts",
    ),
    chord(true, Some(ReviewAction::Send), "Send review"),
    chord(false, None, "Save comment"),
];

/// The key of a press, as the client reads it from the platform event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressKey {
    /// The one character the press produced.
    Char(char),
    /// The Enter key.
    Enter,
    /// Anything else, including a dead key or an input method composing (no text produced).
    Other,
}

/// A key press, built by the client (contract K5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Press {
    /// The key.
    pub key: PressKey,
    /// The platform's primary modifier is held (Cmd on macOS, Ctrl elsewhere).
    pub primary: bool,
    /// Ctrl is held.
    pub ctrl: bool,
    /// Alt (Option) is held.
    pub alt: bool,
    /// The logo key (Cmd, Windows, Super) is held.
    pub logo: bool,
    /// Shift is held.
    pub shift: bool,
    /// The press is an auto-repeat.
    pub repeat: bool,
}

/// Who has the keyboard, as the client decides it (research R7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holder {
    /// The Review tab is shown and nothing else has the keyboard.
    Review,
    /// The review's comment composer has the keyboard.
    Composer,
    /// The shortcut sheet is the topmost overlay.
    Sheet,
    /// Anything else: a field, a dialog, the terminal, another tab.
    Other,
}

/// What `press` does while `holder` has the keyboard (contract K2, K3); `None` leaves the press to
/// whatever has the keyboard. With `single_keys` false (the Settings switch, FR-022) no single key
/// acts under any holder; the send chord still does.
pub fn resolve(press: &Press, holder: Holder, single_keys: bool) -> Option<ReviewAction> {
    let action = REVIEW_KEYS
        .iter()
        .find(|row| matches(row, press))
        .filter(|row| row.repeats || !press.repeat)
        .filter(|row| single_keys || !row.is_single())?
        .action?;
    let heard = match holder {
        Holder::Review => true,
        Holder::Sheet => action == ReviewAction::ToggleSheet,
        Holder::Composer => action == ReviewAction::Send,
        Holder::Other => false,
    };
    heard.then_some(action)
}

/// The sheet's line while the single-key switch is off (FR-022, contract K4a).
pub const SINGLE_KEYS_OFF_NOTE: &str = "Single-key shortcuts are off (Settings → Appearance)";

/// The rows the `?` sheet lists, in table order (contract K4a): every row while `single_keys` is on,
/// only the chords while it is off.
pub fn sheet_rows(single_keys: bool) -> impl Iterator<Item = &'static ReviewKey> {
    REVIEW_KEYS
        .iter()
        .filter(move |row| single_keys || !row.is_single())
}

/// The row a tooltip names for `action` (FR-013, contract K4a); `None` for a single key while
/// `single_keys` is off, so a tooltip never names a key that does nothing.
pub fn key_hint(action: ReviewAction, single_keys: bool) -> Option<&'static ReviewKey> {
    sheet_rows(single_keys).find(|row| row.action == Some(action))
}

/// Whether `press` is `row`'s key (contract K3).
fn matches(row: &ReviewKey, press: &Press) -> bool {
    let key = match (row.key, press.key) {
        (KeyName::Char(want), PressKey::Char(got)) => want == got,
        (KeyName::Enter, PressKey::Enter) => true,
        _ => false,
    };
    if !key {
        return false;
    }
    if !row.mods.primary {
        // Ctrl and Alt without logo is AltGr: the character already carries it.
        let altgr = press.ctrl && press.alt && !press.logo;
        return altgr || !(press.ctrl || press.alt || press.logo);
    }
    // The non-primary of Ctrl / logo: with `primary` from one of them, both held means the other.
    let other_of_ctrl_logo = press.ctrl && press.logo;
    press.primary && press.shift == row.mods.shift && !press.alt && !other_of_ctrl_logo
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One K1 row: (key, primary, shift, action, repeats, sheet text).
    type Row = (
        KeyName,
        bool,
        bool,
        Option<ReviewAction>,
        bool,
        &'static str,
    );

    /// K1's rows, in sheet order.
    const K1: [Row; 12] = [
        (
            KeyName::Char('j'),
            false,
            false,
            Some(ReviewAction::NextFile),
            true,
            "Next file",
        ),
        (
            KeyName::Char('k'),
            false,
            false,
            Some(ReviewAction::PrevFile),
            true,
            "Previous file",
        ),
        (
            KeyName::Char('n'),
            false,
            false,
            Some(ReviewAction::NextHunk),
            true,
            "Next change",
        ),
        (
            KeyName::Char('p'),
            false,
            false,
            Some(ReviewAction::PrevHunk),
            true,
            "Previous change",
        ),
        (
            KeyName::Char(']'),
            false,
            false,
            Some(ReviewAction::NextComment),
            true,
            "Next comment",
        ),
        (
            KeyName::Char('['),
            false,
            false,
            Some(ReviewAction::PrevComment),
            true,
            "Previous comment",
        ),
        (
            KeyName::Char('v'),
            false,
            false,
            Some(ReviewAction::ToggleReviewed),
            false,
            "Mark file reviewed / not reviewed",
        ),
        (
            KeyName::Char('g'),
            false,
            false,
            Some(ReviewAction::NextUnreviewed),
            false,
            "Next file not reviewed",
        ),
        (
            KeyName::Char('c'),
            false,
            false,
            Some(ReviewAction::Comment),
            false,
            "Comment on the picked lines or the current change",
        ),
        (
            KeyName::Char('?'),
            false,
            false,
            Some(ReviewAction::ToggleSheet),
            false,
            "Show or hide these shortcuts",
        ),
        (
            KeyName::Enter,
            true,
            true,
            Some(ReviewAction::Send),
            false,
            "Send review",
        ),
        (KeyName::Enter, true, false, None, false, "Save comment"),
    ];

    #[test]
    fn the_table_has_k1s_twelve_rows_in_sheet_order() {
        assert_eq!(REVIEW_KEYS.len(), K1.len(), "K1 lists twelve rows");
        for (row, (key, primary, shift, action, repeats, sheet)) in REVIEW_KEYS.iter().zip(K1) {
            assert_eq!(row.key, key, "row `{sheet}` is on K1's key, in K1's order");
            assert_eq!(
                row.mods,
                KeyMods { primary, shift },
                "row `{sheet}` carries K1's modifiers"
            );
            assert_eq!(row.action, action, "row `{sheet}` acts as K1 says");
            assert_eq!(
                row.repeats, repeats,
                "row `{sheet}` repeats only as K1 says"
            );
            assert_eq!(row.sheet, sheet, "the sheet text is K1's, verbatim");
        }
    }

    #[test]
    fn no_two_rows_share_a_key_and_modifiers() {
        for (i, a) in REVIEW_KEYS.iter().enumerate() {
            for b in &REVIEW_KEYS[i + 1..] {
                assert!(
                    (a.key, a.mods) != (b.key, b.mods),
                    "`{}` and `{}` would both claim one press",
                    a.sheet,
                    b.sheet
                );
            }
        }
    }

    #[test]
    fn only_the_save_comment_row_has_no_action_and_only_the_steppers_repeat() {
        let actionless: Vec<_> = REVIEW_KEYS.iter().filter(|r| r.action.is_none()).collect();
        assert_eq!(actionless.len(), 1, "one row is listed on the sheet only");
        assert_eq!(
            (actionless[0].key, actionless[0].mods),
            (
                KeyName::Enter,
                KeyMods {
                    primary: true,
                    shift: false
                }
            ),
            "the sheet-only row is the composer's own save chord"
        );
        let repeating: String = REVIEW_KEYS
            .iter()
            .filter(|r| r.repeats)
            .map(|r| match r.key {
                KeyName::Char(c) => c,
                KeyName::Enter => '⏎',
            })
            .collect();
        assert_eq!(
            repeating, "jknp][",
            "holding a stepper steps; nothing else repeats"
        );
    }

    #[test]
    fn labels_give_the_linux_windows_and_the_macos_form() {
        let label = |sheet: &str| {
            REVIEW_KEYS
                .iter()
                .find(|r| r.sheet == sheet)
                .expect("the row is in the table")
                .labels()
        };
        assert_eq!(
            label("Next file"),
            ("j".into(), "j".into()),
            "a char is itself"
        );
        assert_eq!(
            label("Show or hide these shortcuts"),
            ("?".into(), "?".into())
        );
        assert_eq!(
            label("Send review"),
            ("Ctrl+Shift+Enter".into(), "Cmd+Shift+Enter".into()),
            "the send chord names the platform's primary modifier"
        );
        assert_eq!(
            label("Save comment"),
            ("Ctrl+Enter".into(), "Cmd+Enter".into()),
            "the save chord has no Shift"
        );
    }

    fn press(key: PressKey) -> Press {
        Press {
            key,
            primary: false,
            ctrl: false,
            alt: false,
            logo: false,
            shift: false,
            repeat: false,
        }
    }

    fn char_press(c: char) -> Press {
        press(PressKey::Char(c))
    }

    /// The send chord as the client builds it on Linux/Windows (`primary` is Ctrl).
    fn send_chord() -> Press {
        Press {
            primary: true,
            ctrl: true,
            shift: true,
            ..press(PressKey::Enter)
        }
    }

    /// The press a row is bound to, unmodified apart from what the row asks for.
    fn press_for(row: &ReviewKey) -> Press {
        match row.key {
            KeyName::Char(c) => char_press(c),
            KeyName::Enter => Press {
                primary: row.mods.primary,
                ctrl: row.mods.primary,
                shift: row.mods.shift,
                ..press(PressKey::Enter)
            },
        }
    }

    #[test]
    fn every_row_with_an_action_resolves_to_it_while_the_review_has_the_keyboard() {
        for row in REVIEW_KEYS.iter().filter(|r| r.action.is_some()) {
            assert_eq!(
                resolve(&press_for(row), Holder::Review, true),
                row.action,
                "`{}` acts while the review holds the keyboard",
                row.sheet
            );
        }
    }

    #[test]
    fn nothing_resolves_while_something_else_has_the_keyboard() {
        for row in &REVIEW_KEYS {
            assert_eq!(
                resolve(&press_for(row), Holder::Other, true),
                None,
                "`{}` reaches the focused field or dialog (FR-002)",
                row.sheet
            );
        }
    }

    #[test]
    fn the_sheet_hears_only_a_fresh_unmodified_question_mark() {
        assert_eq!(
            resolve(&char_press('?'), Holder::Sheet, true),
            Some(ReviewAction::ToggleSheet),
            "`?` closes the sheet"
        );
        let repeated = Press {
            repeat: true,
            ..char_press('?')
        };
        assert_eq!(
            resolve(&repeated, Holder::Sheet, true),
            None,
            "`?` does not repeat"
        );
        let with_ctrl = Press {
            ctrl: true,
            primary: true,
            ..char_press('?')
        };
        assert_eq!(
            resolve(&with_ctrl, Holder::Sheet, true),
            None,
            "Ctrl+? is not `?`"
        );
        for row in REVIEW_KEYS.iter().filter(|r| r.key != KeyName::Char('?')) {
            assert_eq!(
                resolve(&press_for(row), Holder::Sheet, true),
                None,
                "`{}` does nothing while the sheet is open (FR-012)",
                row.sheet
            );
        }
    }

    #[test]
    fn the_composer_hears_only_a_fresh_send_chord() {
        assert_eq!(
            resolve(&send_chord(), Holder::Composer, true),
            Some(ReviewAction::Send),
            "the chord sends from the composer (FR-010)"
        );
        let repeated = Press {
            repeat: true,
            ..send_chord()
        };
        assert_eq!(
            resolve(&repeated, Holder::Composer, true),
            None,
            "the chord does not repeat"
        );
        assert_eq!(
            resolve(&char_press('j'), Holder::Composer, true),
            None,
            "`j` is typed into the comment (US1 s6)"
        );
    }

    #[test]
    fn ctrl_alt_or_logo_alone_refuses_a_single_key_but_altgr_does_not() {
        for (what, held) in [
            (
                "Ctrl",
                Press {
                    ctrl: true,
                    primary: true,
                    ..char_press('j')
                },
            ),
            (
                "Alt",
                Press {
                    alt: true,
                    ..char_press('j')
                },
            ),
            (
                "Cmd/logo",
                Press {
                    logo: true,
                    ..char_press('j')
                },
            ),
            (
                "Ctrl+Alt+logo",
                Press {
                    ctrl: true,
                    alt: true,
                    logo: true,
                    ..char_press('j')
                },
            ),
        ] {
            assert_eq!(
                resolve(&held, Holder::Review, true),
                None,
                "{what}+j is not `j` (FR-015)"
            );
        }
        let altgr = Press {
            ctrl: true,
            alt: true,
            primary: true,
            ..char_press('[')
        };
        assert_eq!(
            resolve(&altgr, Holder::Review, true),
            Some(ReviewAction::PrevComment),
            "Ctrl+Alt without logo is AltGr, so `[` typed with AltGr acts"
        );
    }

    #[test]
    fn shift_is_part_of_the_character_not_a_modifier() {
        let question = Press {
            shift: true,
            ..char_press('?')
        };
        assert_eq!(
            resolve(&question, Holder::Review, true),
            Some(ReviewAction::ToggleSheet),
            "`?` needs Shift on most layouts and still acts"
        );
        let capital = Press {
            shift: true,
            ..char_press('J')
        };
        assert_eq!(
            resolve(&capital, Holder::Review, true),
            None,
            "`J` is not bound"
        );
    }

    #[test]
    fn a_press_with_no_text_resolves_to_nothing() {
        assert_eq!(
            resolve(&press(PressKey::Other), Holder::Review, true),
            None,
            "a dead key or a composing input method produces no text"
        );
    }

    #[test]
    fn an_auto_repeat_acts_only_for_the_steppers() {
        for row in REVIEW_KEYS.iter().filter(|r| r.action.is_some()) {
            let repeated = Press {
                repeat: true,
                ..press_for(row)
            };
            let expected = if row.repeats { row.action } else { None };
            assert_eq!(
                resolve(&repeated, Holder::Review, true),
                expected,
                "holding `{}` acts once per repeat only if it steps (FR-015)",
                row.sheet
            );
        }
    }

    #[test]
    fn the_send_chord_needs_primary_and_shift_and_nothing_else() {
        let no_primary = Press {
            primary: false,
            ctrl: false,
            ..send_chord()
        };
        let with_alt = Press {
            alt: true,
            ..send_chord()
        };
        // On macOS the client sets `primary` from logo; Ctrl is then the non-primary modifier.
        let mac_with_ctrl = Press {
            logo: true,
            ..send_chord()
        };
        let mac_chord = Press {
            ctrl: false,
            logo: true,
            ..send_chord()
        };
        let no_shift = Press {
            shift: false,
            ..send_chord()
        };
        for holder in [Holder::Review, Holder::Composer] {
            assert_eq!(
                resolve(&no_primary, holder, true),
                None,
                "Shift+Enter alone does not send"
            );
            assert_eq!(
                resolve(&with_alt, holder, true),
                None,
                "Alt refuses the chord"
            );
            assert_eq!(
                resolve(&mac_with_ctrl, holder, true),
                None,
                "Ctrl and Cmd together are not the chord"
            );
            assert_eq!(
                resolve(&mac_chord, holder, true),
                Some(ReviewAction::Send),
                "Cmd+Shift+Enter is the macOS form"
            );
            assert_eq!(
                resolve(&no_shift, holder, true),
                None,
                "primary+Enter is the composer's save, never resolved here"
            );
        }
    }

    const HOLDERS: [Holder; 4] = [
        Holder::Review,
        Holder::Sheet,
        Holder::Composer,
        Holder::Other,
    ];

    #[test]
    fn with_the_switch_off_no_single_key_resolves_under_any_holder() {
        for row in REVIEW_KEYS.iter().filter(|row| row.is_single()) {
            for holder in HOLDERS {
                assert_eq!(
                    resolve(&press_for(row), holder, false),
                    None,
                    "`{}` is off under {holder:?} (FR-022)",
                    row.sheet
                );
            }
        }
    }

    #[test]
    fn with_the_switch_off_the_send_chord_still_sends() {
        for holder in [Holder::Review, Holder::Composer] {
            assert_eq!(
                resolve(&send_chord(), holder, false),
                Some(ReviewAction::Send),
                "the chord is not a single key (FR-022) under {holder:?}"
            );
        }
    }

    #[test]
    fn only_the_ten_char_rows_are_single_keys() {
        let singles: Vec<_> = REVIEW_KEYS
            .iter()
            .filter(|row| row.is_single())
            .map(|row| row.key)
            .collect();
        let chars: Vec<_> = "jknp][vgc?".chars().map(KeyName::Char).collect();
        assert_eq!(singles, chars, "the single keys of FR-001, in table order");
    }

    #[test]
    fn the_sheet_lists_every_row_when_on_and_only_the_chords_when_off() {
        let on: Vec<_> = sheet_rows(true).collect();
        assert_eq!(on.len(), REVIEW_KEYS.len(), "on: every row (SC-004)");
        assert!(
            on.iter().zip(REVIEW_KEYS.iter()).all(|(a, b)| a == &b),
            "on: in table order"
        );
        let off: Vec<_> = sheet_rows(false).map(|row| row.sheet).collect();
        assert_eq!(
            off,
            ["Send review", "Save comment"],
            "off: the two chords, in table order (FR-022)"
        );
        assert_eq!(
            SINGLE_KEYS_OFF_NOTE,
            "Single-key shortcuts are off (Settings → Appearance)"
        );
    }

    #[test]
    fn a_tooltip_names_a_single_key_only_while_the_switch_is_on() {
        assert_eq!(
            key_hint(ReviewAction::NextHunk, true).map(|row| row.key),
            Some(KeyName::Char('n'))
        );
        assert_eq!(
            key_hint(ReviewAction::ToggleReviewed, true).map(|row| row.key),
            Some(KeyName::Char('v'))
        );
        assert_eq!(key_hint(ReviewAction::NextHunk, false), None);
        assert_eq!(key_hint(ReviewAction::ToggleReviewed, false), None);
        for on in [true, false] {
            assert_eq!(
                key_hint(ReviewAction::Send, on).map(|row| row.sheet),
                Some("Send review"),
                "the chord is named either way"
            );
        }
    }
}
