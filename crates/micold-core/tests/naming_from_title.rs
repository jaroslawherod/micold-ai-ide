//! The worktree name a picked issue's title becomes (feature 034, FR-010,
//! contracts/issue-naming-and-typing.md §1, research R11).

use micold_core::naming::{name_from_title, slugify, ISSUE_NAME_SLUG_MAX};

/// Four ten-letter words and one of `last` letters: a slug of `44 + last` characters.
fn five_words(last: usize) -> String {
    let word = "abcdefghij";
    format!("{word} {word} {word} {word} {}", &"abcdefghij"[..last])
}

#[test]
fn fits_and_the_50_boundary() {
    assert_eq!(ISSUE_NAME_SLUG_MAX, 50);
    assert_eq!(
        name_from_title("Crash when opening empty project"),
        "Crash when opening empty project",
        "a title that fits is the name, as written"
    );
    assert_eq!(
        name_from_title("  Fix  the   thing "),
        "Fix the thing",
        "whitespace is normalised to single spaces"
    );

    let exactly_50 = five_words(6);
    assert_eq!(slugify(&exactly_50).len(), 50, "precondition");
    assert_eq!(
        name_from_title(&exactly_50),
        exactly_50,
        "a slug of exactly 50 is kept whole"
    );

    let is_51 = five_words(7);
    assert_eq!(slugify(&is_51).len(), 51, "precondition");
    assert_eq!(
        name_from_title(&is_51),
        "abcdefghij abcdefghij abcdefghij abcdefghij",
        "one character over drops the last word"
    );
}

#[test]
fn cut_at_word_boundary() {
    let title = "When the sidebar is collapsed the create worktree form loses its focus ring";
    assert!(slugify(title).len() > 50, "precondition");
    let name = name_from_title(title);
    assert_eq!(
        name, "When the sidebar is collapsed the create worktree",
        "the longest whole-word prefix whose slug fits"
    );
    assert!(slugify(&name).len() <= 50);
}

#[test]
fn a_long_first_word_is_cut_at_50() {
    let word = "a".repeat(70);
    assert_eq!(
        name_from_title(&format!("{word} more words")),
        "a".repeat(50),
        "no whole word fits, so the first 50 characters of the slug"
    );
}

#[test]
fn empty_slug_yields_empty_name() {
    assert_eq!(
        name_from_title("🔥🔥 !!!"),
        "",
        "nothing sluggable: the name is left empty and 'name required' applies"
    );
    assert_eq!(name_from_title(""), "");
}

#[test]
fn slug_never_exceeds_50() {
    let long_word = "x".repeat(49);
    let mut corpus: Vec<String> = vec![
        "Crash when opening empty project".into(),
        "Fix: the — thing (again) & again!".into(),
        "Ünïcödé títle with ascii bits 123".into(),
        format!("{long_word} y"),
        format!("{long_word}z"),
        format!("{long_word}zz tail"),
        "a b c d e f g h i j k l m n o p q r s t u v w x y z a b c d e f g h".into(),
        "con".into(),
        "🔥 fire in the hole".into(),
    ];
    for last in 0..=10 {
        corpus.push(five_words(last));
    }
    for n in 45..=56 {
        corpus.push("w".repeat(n));
    }
    for title in &corpus {
        let name = name_from_title(title);
        assert!(
            slugify(&name).len() <= ISSUE_NAME_SLUG_MAX,
            "{title:?} -> {name:?} slugs past 50"
        );
        let normalised = title.split_whitespace().collect::<Vec<_>>().join(" ");
        if slugify(&normalised).len() <= ISSUE_NAME_SLUG_MAX {
            assert_eq!(name, normalised, "{title:?} fits, so it is kept whole");
        }
    }
}
