//! T014 (feature 041): the bytes of a saved-history file (contracts/saved-history-file.md §2, §4,
//! §5) — what is encoded decodes to the same snapshot, the header is the documented one, each
//! damage is named by the first check it fails, no input makes a read panic, and the bytes of one
//! snapshot are pinned by a committed fixture.

use std::path::PathBuf;

use micold_core::protocol::hashing::sha256;
use micold_core::terminal_history::{
    decode, encode, DamageReason, HistoryColor, HistorySnapshot, HistoryStyle, LogicalLine,
    StyleFlags, StyleRun, BASIC_COLORS, DIM_COLORS, FORMAT_VERSION, MAX_FILE_BYTES,
};

const MAGIC: &[u8; 8] = b"MICOLDTH";
/// Magic, version and payload length.
const HEADER_BYTES: usize = 20;
const CHECKSUM_BYTES: usize = 32;
/// A header and a checksum with no payload between them.
const SMALLEST_FILE_BYTES: usize = HEADER_BYTES + CHECKSUM_BYTES;
const VERSION_AT: std::ops::Range<usize> = 8..12;
const LENGTH_AT: std::ops::Range<usize> = 12..HEADER_BYTES;

const ALL_FLAGS: [StyleFlags; 7] = [
    StyleFlags::BOLD,
    StyleFlags::DIM,
    StyleFlags::ITALIC,
    StyleFlags::UNDERLINE,
    StyleFlags::INVERSE,
    StyleFlags::STRIKETHROUGH,
    StyleFlags::HIDDEN,
];

/// The `postcard` payload of [`ab`]: one style (default foreground, default background, no flag),
/// then one line, its text `ab` and its one run of 2 characters in style 0.
const AB_PAYLOAD: [u8; 11] = [1, 0, 0, 0, 1, 2, b'a', b'b', 1, 2, 0];
/// Where [`AB_PAYLOAD`] holds the first character of the text, the run's length and its style.
const AB_FIRST_CHAR: usize = 6;
const AB_RUN_CHARS: usize = 9;
const AB_RUN_STYLE: usize = 10;
const ESC: u8 = 0x1b;

fn run(chars: u32, style: HistoryStyle) -> StyleRun {
    StyleRun { chars, style }
}

fn fg(color: HistoryColor) -> HistoryStyle {
    HistoryStyle {
        fg: color,
        ..HistoryStyle::default()
    }
}

/// A line of `text` in one `style`.
fn line(text: &str, style: HistoryStyle) -> LogicalLine {
    LogicalLine {
        text: text.to_string(),
        runs: vec![run(text.chars().count() as u32, style)],
    }
}

fn snapshot_of(lines: Vec<LogicalLine>) -> HistorySnapshot {
    HistorySnapshot { lines }
}

/// One line, `ab`, in the default style.
fn ab() -> HistorySnapshot {
    snapshot_of(vec![line("ab", HistoryStyle::default())])
}

/// A whole file around `payload`: the header of HF §2, the payload, the checksum of both.
fn file_with_payload(payload: &[u8]) -> Vec<u8> {
    let mut file = MAGIC.to_vec();
    file.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    file.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    file.extend_from_slice(payload);
    let checksum = sha256(&file);
    file.extend_from_slice(&checksum);
    file
}

/// [`AB_PAYLOAD`] with the byte at `index` replaced, in a file whose checksum matches.
fn ab_file_with(index: usize, byte: u8) -> Vec<u8> {
    let mut payload = AB_PAYLOAD;
    payload[index] = byte;
    file_with_payload(&payload)
}

/// A snapshot that uses every colour kind as a foreground and as a background, every flag alone
/// and all together, a line of several runs, an empty line and text outside ASCII.
fn every_colour_kind_and_flag() -> HistorySnapshot {
    let colors = [
        HistoryColor::Default,
        HistoryColor::Basic(0),
        HistoryColor::Basic(15),
        HistoryColor::Dim(7),
        HistoryColor::Indexed(255),
        HistoryColor::Rgb(1, 2, 3),
    ];
    let mut lines: Vec<LogicalLine> = colors
        .iter()
        .flat_map(|&color| {
            let as_bg = HistoryStyle {
                bg: color,
                ..HistoryStyle::default()
            };
            [line("fg", fg(color)), line("bg", as_bg)]
        })
        .collect();
    let flag = |flags| HistoryStyle {
        flags,
        ..HistoryStyle::default()
    };
    lines.extend(ALL_FLAGS.iter().map(|&one| line("flag", flag(one))));
    let all = ALL_FLAGS
        .iter()
        .fold(StyleFlags::default(), |set, &one| set.with(one));
    lines.push(line("all flags", flag(all)));
    lines.push(LogicalLine {
        text: "red plain żółć 日本".to_string(),
        runs: vec![
            run(3, fg(HistoryColor::Basic(1))),
            run(7, HistoryStyle::default()),
            run(4, fg(HistoryColor::Rgb(1, 2, 3))),
            run(3, fg(HistoryColor::Basic(1))),
        ],
    });
    lines.push(LogicalLine::default());
    snapshot_of(lines)
}

/// A deterministic source of bytes (xorshift64), so a failure names the same input every run.
struct Bytes(u64);

impl Bytes {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn string(&mut self, max_len: u64) -> Vec<u8> {
        let len = self.next() % (max_len + 1);
        (0..len).map(|_| self.next() as u8).collect()
    }
}

const RANDOM_INPUTS: usize = 1_000;
const RANDOM_MAX_LEN: u64 = 200;

// U39: what is saved is what is read back.
#[test]
fn encode_then_decode_gives_the_same_snapshot() {
    const MANY_LINES: usize = 10_000;
    let many = snapshot_of(
        (0..MANY_LINES)
            .map(|n| {
                line(
                    &format!("line {n}"),
                    fg(HistoryColor::Indexed((n % 256) as u8)),
                )
            })
            .collect(),
    );
    let cases = [
        ("empty", HistorySnapshot::default()),
        ("one line", ab()),
        ("10,000 lines", many),
        ("every colour kind and flag", every_colour_kind_and_flag()),
    ];

    for (name, snapshot) in cases {
        assert_eq!(decode(&encode(&snapshot)), Ok(snapshot), "{name}");
    }
}

// U40: the layout of HF §2, byte for byte.
#[test]
fn the_encoded_bytes_are_the_header_the_payload_and_the_checksum_of_both() {
    let file = encode(&ab());

    assert_eq!(file.len(), SMALLEST_FILE_BYTES + AB_PAYLOAD.len());
    let (body, checksum) = file.split_at(file.len() - CHECKSUM_BYTES);
    assert_eq!(&body[..MAGIC.len()], MAGIC, "magic");
    assert_eq!(body[VERSION_AT], 1u32.to_le_bytes(), "version");
    assert_eq!(
        body[LENGTH_AT],
        (AB_PAYLOAD.len() as u64).to_le_bytes(),
        "payload length"
    );
    assert_eq!(body[HEADER_BYTES..], AB_PAYLOAD, "payload");
    assert_eq!(checksum, sha256(body), "checksum");
}

// U40: a style used by several runs is written once.
#[test]
fn a_style_used_twice_is_stored_once() {
    let red = fg(HistoryColor::Basic(1));
    let twice = snapshot_of(vec![line("a", red), line("b", red)]);
    // One style (Basic = kind 1, colour 1; default background; no flag), two lines in style 0.
    const PAYLOAD: [u8; 16] = [1, 1, 1, 0, 0, 2, 1, b'a', 1, 1, 0, 1, b'b', 1, 1, 0];

    assert_eq!(encode(&twice), file_with_payload(&PAYLOAD));
}

// U41, check 2.
#[test]
fn a_file_over_the_size_cap_is_too_large() {
    let over = vec![0u8; MAX_FILE_BYTES as usize + 1];

    assert_eq!(decode(&over), Err(DamageReason::TooLarge));
}

// U41, check 3.
#[test]
fn another_magic_is_not_a_history() {
    let mut file = encode(&ab());
    file[0] ^= 1;

    assert_eq!(decode(&file), Err(DamageReason::NotAHistory));
}

// U41, check 3: 52 bytes, a header and a checksum around no payload, is the least a file can be.
#[test]
fn fewer_than_52_bytes_is_not_a_history() {
    let no_payload = file_with_payload(&[]);
    assert_eq!(no_payload.len(), SMALLEST_FILE_BYTES);

    assert_eq!(
        decode(&no_payload[..SMALLEST_FILE_BYTES - 1]),
        Err(DamageReason::NotAHistory)
    );
    assert_eq!(
        decode(&no_payload),
        Err(DamageReason::Malformed),
        "52 bytes pass the size check"
    );
}

// U41, check 4: a file of another version is damaged, never migrated (HF §5).
#[test]
fn version_2_is_another_version() {
    const OTHER: u32 = 2;
    let mut file = encode(&ab());
    file[VERSION_AT].copy_from_slice(&OTHER.to_le_bytes());

    assert_eq!(decode(&file), Err(DamageReason::OtherVersion(OTHER)));
}

// U41, check 5: the size is not the one the header gives.
#[test]
fn a_file_cut_short_or_grown_is_truncated() {
    let whole = encode(&ab());
    let cut = &whole[..whole.len() - 1];
    let mut grown = whole.clone();
    grown.push(0);
    let mut length_overflows = whole.clone();
    length_overflows[LENGTH_AT].copy_from_slice(&u64::MAX.to_le_bytes());

    assert_eq!(decode(cut), Err(DamageReason::Truncated), "cut");
    assert_eq!(decode(&grown), Err(DamageReason::Truncated), "grown");
    assert_eq!(
        decode(&length_overflows),
        Err(DamageReason::Truncated),
        "a length that overflows"
    );
}

// U41, check 6.
#[test]
fn one_flipped_payload_bit_fails_the_checksum() {
    let mut file = encode(&ab());
    file[HEADER_BYTES + AB_FIRST_CHAR] ^= 1;

    assert_eq!(decode(&file), Err(DamageReason::Checksum));
}

// U41, check 7: the payload must decode with nothing left over.
#[test]
fn bytes_after_the_payload_or_a_payload_cut_short_are_malformed() {
    let mut trailing = AB_PAYLOAD.to_vec();
    trailing.push(0);
    let cut = &AB_PAYLOAD[..AB_PAYLOAD.len() - 1];

    assert_eq!(
        decode(&file_with_payload(&trailing)),
        Err(DamageReason::Malformed),
        "trailing"
    );
    assert_eq!(
        decode(&file_with_payload(cut)),
        Err(DamageReason::Malformed),
        "cut"
    );
}

// U41, check 8.
#[test]
fn a_style_index_outside_the_styles_is_a_bad_style_index() {
    let file = ab_file_with(AB_RUN_STYLE, 1);

    assert_eq!(decode(&file), Err(DamageReason::BadStyleIndex));
}

// U41, check 9.
#[test]
fn runs_that_do_not_sum_to_the_text_are_a_bad_run_length() {
    let file = ab_file_with(AB_RUN_CHARS, 3);

    assert_eq!(decode(&file), Err(DamageReason::BadRunLength));
}

// U41, check 10.
#[test]
fn a_text_with_esc_is_a_control_character() {
    let file = ab_file_with(AB_FIRST_CHAR, ESC);

    assert_eq!(decode(&file), Err(DamageReason::ControlCharacter));
}

// FR-016: a file that passes the ten checks and still holds a snapshot `HistorySnapshot::validate`
// rejects is not shown either. The checksum only says the bytes are the ones that were written.
#[test]
fn a_colour_index_outside_its_palette_is_malformed() {
    /// The enum indexes of `HistoryColor::Basic` and `HistoryColor::Dim` in the payload.
    const BASIC: u8 = 1;
    const DIM: u8 = 2;
    // [`AB_PAYLOAD`] with its one style's foreground, then its background, replaced.
    let with_fg = |kind: u8, index: u8| {
        file_with_payload(&[1, kind, index, 0, 0, 1, 2, b'a', b'b', 1, 2, 0])
    };
    let with_bg = |kind: u8, index: u8| {
        file_with_payload(&[1, 0, kind, index, 0, 1, 2, b'a', b'b', 1, 2, 0])
    };

    assert_eq!(
        decode(&with_fg(BASIC, BASIC_COLORS - 1)),
        Ok(snapshot_of(vec![line(
            "ab",
            fg(HistoryColor::Basic(BASIC_COLORS - 1))
        )])),
        "the last basic colour is in the palette"
    );
    for file in [
        with_fg(BASIC, BASIC_COLORS),
        with_fg(DIM, DIM_COLORS),
        with_bg(BASIC, BASIC_COLORS),
        with_bg(DIM, DIM_COLORS),
    ] {
        assert_eq!(decode(&file), Err(DamageReason::Malformed));
    }
}

// U41: the checks run in HF §4's order over the whole file, so the first one that fails names the
// damage whichever line it is on.
#[test]
fn the_first_failing_check_names_the_damage() {
    // No style; line 0 is `ESC` with no run (checks 9 and 10), line 1 is `b` with a run of 1
    // character in style 0 (check 8).
    const ALL_THREE: [u8; 10] = [0, 2, 1, ESC, 0, 1, b'b', 1, 1, 0];
    // One style; line 0 is `ESC` with a run of 1 (check 10), line 1 is `b` with no run (check 9).
    const RUN_LENGTH_AND_ESC: [u8; 13] = [1, 0, 0, 0, 2, 1, ESC, 1, 1, 0, 1, b'b', 0];

    assert_eq!(
        decode(&file_with_payload(&ALL_THREE)),
        Err(DamageReason::BadStyleIndex)
    );
    assert_eq!(
        decode(&file_with_payload(&RUN_LENGTH_AND_ESC)),
        Err(DamageReason::BadRunLength)
    );
}

// T019 (DM §3): the reason is what the log shows (FR-017).
#[test]
fn each_reason_has_its_own_text_for_the_log() {
    let reasons = [
        DamageReason::Unreadable(std::io::ErrorKind::PermissionDenied),
        DamageReason::TooLarge,
        DamageReason::NotAHistory,
        DamageReason::OtherVersion(2),
        DamageReason::Truncated,
        DamageReason::Checksum,
        DamageReason::Malformed,
        DamageReason::BadStyleIndex,
        DamageReason::BadRunLength,
        DamageReason::ControlCharacter,
    ];

    let texts: Vec<String> = reasons.iter().map(ToString::to_string).collect();

    assert_eq!(texts[3], "written by another version");
    for (index, text) in texts.iter().enumerate() {
        assert!(!text.is_empty(), "{:?} has a text", reasons[index]);
        assert!(
            !texts[..index].contains(text),
            "{:?} has its own text",
            reasons[index]
        );
    }
}

// U42: a read never returns part of a file.
#[test]
fn every_prefix_of_a_valid_file_is_damaged() {
    let file = encode(&every_colour_kind_and_flag());
    assert!(decode(&file).is_ok(), "the whole file is valid");

    for len in 0..file.len() {
        let expected = if len < SMALLEST_FILE_BYTES {
            DamageReason::NotAHistory
        } else {
            DamageReason::Truncated
        };
        assert_eq!(decode(&file[..len]), Err(expected), "{len} bytes");
    }
}

// U42: no bytes make a read panic.
#[test]
fn random_byte_strings_are_damaged_without_a_panic() {
    let mut bytes = Bytes(0x41_5eed);

    for n in 0..RANDOM_INPUTS {
        let input = bytes.string(RANDOM_MAX_LEN);
        assert!(decode(&input).is_err(), "input {n}: {input:?}");

        // The same bytes behind a valid magic and version reach the length check.
        let mut headed = MAGIC.to_vec();
        headed.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        headed.extend_from_slice(&input);
        assert!(decode(&headed).is_err(), "headed input {n}: {input:?}");
    }
}

// U42: random payloads under a matching checksum reach the checks behind it. One may be a valid
// history by chance; then it is a whole one, which encodes and reads back the same.
#[test]
fn random_payloads_with_a_matching_checksum_do_not_panic() {
    let mut bytes = Bytes(0x41_cafe);
    let mut reasons = Vec::new();

    for n in 0..RANDOM_INPUTS {
        let payload = bytes.string(RANDOM_MAX_LEN);
        match decode(&file_with_payload(&payload)) {
            Ok(snapshot) => assert_eq!(
                decode(&encode(&snapshot)),
                Ok(snapshot),
                "payload {n}: {payload:?}"
            ),
            Err(reason) => reasons.push(reason),
        }
    }

    assert!(
        reasons.contains(&DamageReason::Malformed),
        "random payloads reached the payload check"
    );
    for reason in reasons {
        assert!(
            matches!(
                reason,
                DamageReason::Malformed
                    | DamageReason::BadStyleIndex
                    | DamageReason::BadRunLength
                    | DamageReason::ControlCharacter
            ),
            "{reason:?} is a reason of checks 7 to 10"
        );
    }
}

// U43: the bytes of one snapshot are pinned (HF §5). A change of the types that changes the
// encoding fails here until `FORMAT_VERSION` is bumped and a fixture of the new version is added.
#[test]
fn the_v1_fixture_is_the_encoding_of_its_snapshot() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/terminal_history/v1.history");
    let fixture =
        std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));

    assert_eq!(encode(&every_colour_kind_and_flag()), fixture);
    assert_eq!(decode(&fixture), Ok(every_colour_kind_and_flag()));
}
