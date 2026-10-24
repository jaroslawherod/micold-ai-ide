//! The stored shape of a project's review comments (feature 482, data-model § Core: persistence):
//! `reviews/<project id>.json` beside `projects/`, written and read by
//! [`crate::store::JsonFileStore::save_reviews`] and [`crate::store::JsonFileStore::load_reviews`].

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::comment::{fresh_id, CommentId, EntryReview, ReviewComment};
use super::marks::ReviewMark;

/// The newest version this build writes (feature 1271, S4): 4, and only while some entry has a
/// reviewed mark; otherwise 3 while some comment has a pull request (feature 1149, S3), else 2
/// (feature 946, S2), so a file without marks stays readable by an older build. It also reads
/// 482's version 1, whose `pending` state loads as Draft.
pub const REVIEW_FILE_VERSION: u32 = 4;

/// The version written while some comment has a pull request and no entry has a mark.
const PR_FILE_VERSION: u32 = 3;

/// The version written while no comment has a pull request and no entry has a mark.
const PLAIN_FILE_VERSION: u32 = 2;

/// The versions this build reads.
const READABLE_VERSIONS: [u32; 4] = [1, PLAIN_FILE_VERSION, PR_FILE_VERSION, REVIEW_FILE_VERSION];

/// Every entry's comments of one project, keyed by worktree directory name; `""` is the Default
/// entry (the project root), as on the wire.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewFile {
    /// Comments per entry.
    pub entries: BTreeMap<String, Vec<ReviewComment>>,
    /// Reviewed marks per entry, keyed like `entries` (feature 1271). An entry with neither
    /// comments nor marks is not written.
    pub marks: BTreeMap<String, Vec<ReviewMark>>,
}

/// An entry's location as stored: `"default"` or `{ "worktree": "<dir>" }`.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StoredLocation {
    Default,
    Worktree(String),
}

#[derive(Serialize, Deserialize)]
struct StoredEntry {
    location: StoredLocation,
    comments: Vec<ReviewComment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    marks: Vec<ReviewMark>,
}

#[derive(Serialize, Deserialize)]
struct StoredReviews {
    version: u32,
    entries: Vec<StoredEntry>,
}

impl ReviewFile {
    /// The file's JSON text.
    pub fn to_json(&self) -> String {
        let dirs: BTreeSet<&String> = self
            .entries
            .iter()
            .filter(|(_, comments)| !comments.is_empty())
            .map(|(dir, _)| dir)
            .chain(
                self.marks
                    .iter()
                    .filter(|(_, marks)| !marks.is_empty())
                    .map(|(dir, _)| dir),
            )
            .collect();
        let entries = dirs
            .into_iter()
            .map(|dir| StoredEntry {
                location: if dir.is_empty() {
                    StoredLocation::Default
                } else {
                    StoredLocation::Worktree(dir.clone())
                },
                comments: self.entries.get(dir).cloned().unwrap_or_default(),
                marks: self.marks.get(dir).cloned().unwrap_or_default(),
            })
            .collect();
        let with_marks = self.marks.values().any(|marks| !marks.is_empty());
        let with_pr = self
            .entries
            .values()
            .flatten()
            .any(|comment| comment.pr.is_some());
        let version = if with_marks {
            REVIEW_FILE_VERSION
        } else if with_pr {
            PR_FILE_VERSION
        } else {
            PLAIN_FILE_VERSION
        };
        let stored = StoredReviews { version, entries };
        // Every field is a string, a number or a list of them: serialising cannot fail.
        serde_json::to_string_pretty(&stored).unwrap_or_default()
    }

    /// Read the file's JSON text; an error for anything unparseable or of a version this build
    /// does not read. A comment whose reference collides with an earlier one of its entry loads
    /// under a fresh id (research R4).
    pub fn from_json(text: &str) -> Result<Self, String> {
        let stored: StoredReviews = serde_json::from_str(text).map_err(|err| err.to_string())?;
        if !READABLE_VERSIONS.contains(&stored.version) {
            return Err(format!("unknown review file version {}", stored.version));
        }
        let mut file = Self::default();
        for entry in stored.entries {
            let dir = match entry.location {
                StoredLocation::Default => String::new(),
                StoredLocation::Worktree(dir) => dir,
            };
            // A location listed twice: the last entry wins for its marks as for its comments.
            if entry.marks.is_empty() {
                file.marks.remove(&dir);
            } else {
                file.marks.insert(dir.clone(), entry.marks);
            }
            file.entries
                .insert(dir, with_distinct_references(entry.comments));
        }
        Ok(file)
    }
}

/// `comments` in order, each one whose reference an earlier one already has under a fresh id.
fn with_distinct_references(comments: Vec<ReviewComment>) -> Vec<ReviewComment> {
    let mut kept: Vec<ReviewComment> = Vec::with_capacity(comments.len());
    for mut comment in comments {
        if kept.iter().any(|c| c.reference() == comment.reference()) {
            let entry = EntryReview::from_comments(kept.clone());
            comment.id = fresh_id(&entry, CommentId::new);
        }
        kept.push(comment);
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::comment::{CommentState, Scope};
    use crate::review::marks::ChangeId;
    use crate::review::{LineRange, RelPath, Side};
    use crate::store::JsonFileStore;
    use serde_json::json;
    use std::path::Path;
    use uuid::Uuid;

    fn comment(n: u128, state: CommentState) -> ReviewComment {
        ReviewComment {
            id: CommentId(Uuid::from_u128(n << 96)),
            path: RelPath::from_git("src/a.rs"),
            side: Side::New,
            range: LineRange::new(12, 13).expect("12..=13"),
            quote: vec!["a".into(), "b".into()],
            text: "Why?".into(),
            state,
            created: 1_790_000_000,
            pr: None,
        }
    }

    fn sample() -> ReviewFile {
        let mut file = ReviewFile::default();
        file.entries.insert(
            "feature-x".into(),
            vec![
                comment(1, CommentState::Draft),
                comment(2, CommentState::Sent { at: 1_790_000_100 }),
            ],
        );
        file.entries
            .insert(String::new(), vec![comment(3, CommentState::Draft)]);
        file
    }

    #[test]
    fn the_file_has_the_data_model_shape() {
        let value: serde_json::Value =
            serde_json::from_str(&sample().to_json()).expect("the file is JSON");
        assert_eq!(value["version"], json!(2), "the file says its version");
        let entries = value["entries"].as_array().expect("entries is a list");
        let locations: Vec<_> = entries.iter().map(|e| e["location"].clone()).collect();
        assert_eq!(
            locations,
            vec![json!("default"), json!({ "worktree": "feature-x" })],
            "the Default entry is `\"default\"`, a worktree `{{\"worktree\": dir}}`"
        );
        assert_eq!(entries[1]["comments"][0]["start"], json!(12));
        assert_eq!(
            entries[1]["comments"][1]["state"],
            json!({ "sent": { "at": 1_790_000_100u64 } })
        );
    }

    #[test]
    fn the_file_round_trips() {
        let file = sample();
        let back = ReviewFile::from_json(&file.to_json()).expect("its own output parses");
        assert_eq!(
            back, file,
            "every entry and comment comes back with its state"
        );
    }

    #[test]
    fn entries_with_no_comments_are_not_written() {
        let mut file = sample();
        file.entries.insert("empty".into(), Vec::new());
        let value: serde_json::Value =
            serde_json::from_str(&file.to_json()).expect("the file is JSON");
        assert_eq!(
            value["entries"].as_array().map(Vec::len),
            Some(2),
            "an entry without comments leaves no trace in the file"
        );
    }

    #[test]
    fn another_version_or_garbage_is_refused() {
        assert!(
            ReviewFile::from_json("{ not json").is_err(),
            "garbage is refused"
        );
        assert!(
            ReviewFile::from_json(r#"{ "version": 5, "entries": [] }"#).is_err(),
            "a version this build does not know is refused rather than misread"
        );
    }

    /// A comment of `src/a.rs` as stored, with `id` and the stored `state`.
    fn stored_comment(id: &str, state: serde_json::Value) -> serde_json::Value {
        json!({
            "id": id, "path": "src/a.rs", "side": "new", "start": 12, "end": 13,
            "quote": ["a", "b"], "text": "Why?", "state": state, "created": 1_790_000_000u64,
            "pr": null
        })
    }

    fn file_of(version: u32, comments: Vec<serde_json::Value>) -> String {
        json!({
            "version": version,
            "entries": [{ "location": { "worktree": "feature-x" }, "comments": comments }]
        })
        .to_string()
    }

    const ID_1: &str = "00000001-0000-0000-0000-000000000000";
    const ID_2: &str = "00000002-0000-0000-0000-000000000000";
    const ID_3: &str = "00000003-0000-0000-0000-000000000000";

    #[test]
    fn a_version_1_file_loads_pending_as_draft_and_sent_as_sent() {
        let text = file_of(
            1,
            vec![
                stored_comment(ID_1, json!({ "pending": null })),
                stored_comment(ID_2, json!({ "sent": { "at": 1_790_000_100u64 } })),
            ],
        );
        let file = ReviewFile::from_json(&text).expect("482's files still load (S2)");
        let states: Vec<_> = file.entries["feature-x"]
            .iter()
            .map(|c| c.state.clone())
            .collect();
        assert_eq!(
            states,
            vec![
                CommentState::Draft,
                CommentState::Sent { at: 1_790_000_100 }
            ]
        );
        let sent = &file.entries["feature-x"][1];
        assert_eq!(
            sent.reference().to_string(),
            "C-00000002",
            "a v1 Sent comment has a reference after load"
        );
    }

    #[test]
    fn saving_after_loading_a_version_1_file_writes_version_2() {
        let text = file_of(1, vec![stored_comment(ID_1, json!({ "pending": null }))]);
        let file = ReviewFile::from_json(&text).expect("loads");
        let value: serde_json::Value = serde_json::from_str(&file.to_json()).expect("JSON");
        assert_eq!(value["version"], json!(2));
        assert_eq!(
            value["entries"][0]["comments"][0]["state"],
            json!({ "draft": null })
        );
    }

    #[test]
    fn a_version_2_file_with_all_three_states_round_trips() {
        let text = file_of(
            2,
            vec![
                stored_comment(ID_1, json!({ "draft": null })),
                stored_comment(ID_2, json!({ "sent": { "at": 1_790_000_100u64 } })),
                stored_comment(
                    ID_3,
                    json!({ "reported": {
                        "sent_at": 1_790_000_100u64, "outcome": "answered", "reply": "Kept: see R4",
                        "first_at": 1_790_000_200u64, "last_at": 1_790_000_300u64,
                        "by": { "agent": "00000000-0000-0000-0000-0000000000aa" }
                    } }),
                ),
                stored_comment(
                    "00000004-0000-0000-0000-000000000000",
                    json!({ "reported": {
                        "sent_at": 1_790_000_100u64, "outcome": "applied", "reply": "",
                        "first_at": 1_790_000_200u64, "last_at": 1_790_000_200u64, "by": "user"
                    } }),
                ),
            ],
        );
        let file = ReviewFile::from_json(&text).expect("a v2 file loads");
        let written = file.to_json();
        let input: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        let output: serde_json::Value = serde_json::from_str(&written).expect("JSON");
        assert_eq!(output, input, "every state is written as it was read");
        let again = ReviewFile::from_json(&written).expect("its own output loads");
        assert_eq!(again.to_json(), written, "and writes back byte-equal");
    }

    fn pr_comment(n: u128) -> ReviewComment {
        let mut c = comment(n, CommentState::Draft);
        c.pr = Some(crate::review::comment::PrAnchor {
            number: 7,
            head_branch: "feat/x".into(),
            head_commit: "a".repeat(40),
        });
        c
    }

    #[test]
    fn a_file_with_a_pull_request_comment_is_version_3_and_round_trips_it() {
        let mut file = sample();
        file.entries
            .get_mut("feature-x")
            .expect("entry")
            .push(pr_comment(5));
        let json = file.to_json();
        let value: serde_json::Value = serde_json::from_str(&json).expect("JSON");
        assert_eq!(value["version"], json!(3));
        assert_eq!(
            value["entries"][1]["comments"][2]["pr"],
            json!({ "number": 7, "head_branch": "feat/x", "head_commit": "a".repeat(40) })
        );
        assert_eq!(ReviewFile::from_json(&json).expect("loads"), file);
    }

    #[test]
    fn a_file_without_pull_request_comments_is_written_as_version_2() {
        let value: serde_json::Value = serde_json::from_str(&sample().to_json()).expect("JSON");
        assert_eq!(value["version"], json!(2));
    }

    #[test]
    fn versions_1_and_2_load_with_no_pull_request() {
        for version in [1, 2] {
            let mut comment = stored_comment(ID_1, json!({ "draft": null }));
            comment.as_object_mut().expect("object").remove("pr");
            let file = ReviewFile::from_json(&file_of(version, vec![comment])).expect("loads");
            assert_eq!(file.entries["feature-x"][0].pr, None, "version {version}");
        }
    }

    #[test]
    fn a_colliding_reference_gets_a_fresh_id_on_load() {
        let earlier = "1a2b3c4d-0000-4000-8000-000000000001";
        let later = "1a2b3c4d-0000-4000-8000-000000000002";
        let text = file_of(
            2,
            vec![
                stored_comment(earlier, json!({ "draft": null })),
                stored_comment(later, json!({ "draft": null })),
            ],
        );
        let file = ReviewFile::from_json(&text).expect("loads");
        let comments = &file.entries["feature-x"];
        assert_eq!(comments.len(), 2, "both comments load");
        assert_eq!(
            comments[0].id.0.to_string(),
            earlier,
            "the earlier one is unchanged"
        );
        assert_ne!(
            comments[1].id.0.to_string(),
            later,
            "the later one is re-id'd"
        );
        assert_ne!(
            comments[1].reference(),
            comments[0].reference(),
            "and its reference differs (R4)"
        );
        assert_eq!(comments[1].text, "Why?", "keeping everything but its id");
    }

    fn store_in(dir: &Path) -> JsonFileStore {
        JsonFileStore::at(dir.join("projects.json"))
    }

    #[test]
    fn a_missing_file_loads_an_empty_review() {
        let dir = tempfile::tempdir().expect("temp dir");
        let loaded = store_in(dir.path()).load_reviews(Path::new("/repo"));
        assert_eq!(loaded, ReviewFile::default(), "no file is no comments");
    }

    #[test]
    fn saved_reviews_load_back_from_the_reviews_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = store_in(dir.path());
        let project = Path::new("/repo");
        store.save_reviews(project, &sample()).expect("saved");
        let path = store.reviews_path(project);
        assert_eq!(
            path.parent()
                .and_then(Path::file_name)
                .and_then(|n| n.to_str()),
            Some("reviews"),
            "the file lives in `reviews/` beside `projects/`"
        );
        assert!(path.is_file(), "the file is written");
        assert_eq!(
            store.load_reviews(project),
            sample(),
            "what was saved loads back"
        );
        let others = store.load_reviews(Path::new("/other"));
        assert_eq!(
            others,
            ReviewFile::default(),
            "another project's file is its own"
        );
    }

    #[test]
    fn a_file_of_an_unknown_version_is_kept_aside() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = store_in(dir.path());
        let project = Path::new("/repo");
        let path = store.reviews_path(project);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        let text = file_of(5, Vec::new());
        std::fs::write(&path, &text).expect("written");
        assert_eq!(store.load_reviews(project), ReviewFile::default());
        let mut aside = path.as_os_str().to_os_string();
        aside.push(".corrupt");
        assert_eq!(
            std::fs::read_to_string(&aside).ok(),
            Some(text),
            "a version 5 file is quarantined as 482's"
        );
    }

    #[test]
    fn an_unparseable_file_is_kept_aside_and_an_empty_review_loads() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = store_in(dir.path());
        let project = Path::new("/repo");
        let path = store.reviews_path(project);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(&path, "{ broken").expect("written");
        assert_eq!(
            store.load_reviews(project),
            ReviewFile::default(),
            "recovers empty"
        );
        let mut aside = path.as_os_str().to_os_string();
        aside.push(".corrupt");
        assert_eq!(
            std::fs::read_to_string(&aside).ok().as_deref(),
            Some("{ broken"),
            "the unreadable bytes are kept as `<name>.corrupt`"
        );
    }

    // ---- version 4: reviewed marks (feature 1271, S4, T016) -------------------------------------

    fn mark(scope: Scope, path: &str, n: u8) -> ReviewMark {
        ReviewMark {
            scope,
            path: RelPath::from_git(path),
            change: ChangeId::parse(&format!("{n:02x}").repeat(20)).expect("40 hex"),
        }
    }

    fn with_marks() -> ReviewFile {
        let mut file = sample();
        file.marks.insert(
            "feature-x".into(),
            vec![
                mark(Scope::WorkingTree, "src/a.rs", 1),
                mark(Scope::PullRequest(12), "src/a.rs", 2),
            ],
        );
        file.marks.insert(
            "marks-only".into(),
            vec![mark(Scope::WorkingTree, "b.rs", 3)],
        );
        file
    }

    #[test]
    fn a_location_listed_twice_loads_its_last_entry_for_marks_and_comments() {
        // Review A M1 F3: the first entry's marks must not outlive its comments.
        let written: serde_json::Value =
            serde_json::from_str(&with_marks().to_json()).expect("JSON");
        let entry = written["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|e| e["location"] == json!({ "worktree": "feature-x" }))
            .expect("feature-x is written")
            .clone();
        let mut later = entry.clone();
        later["marks"] = json!([]);
        later["comments"] = json!([]);
        let text = json!({ "version": 4, "entries": [entry, later] }).to_string();
        let file = ReviewFile::from_json(&text).expect("loads");
        assert!(
            !file.marks.contains_key("feature-x"),
            "the last entry has no marks"
        );
        assert!(file.entries["feature-x"].is_empty(), "and no comments");
    }

    #[test]
    fn a_file_with_marks_is_version_4_and_round_trips_them() {
        let file = with_marks();
        let json = file.to_json();
        let value: serde_json::Value = serde_json::from_str(&json).expect("JSON");
        assert_eq!(value["version"], json!(4));
        assert_eq!(
            value["entries"][1]["marks"],
            json!([
                { "scope": "WorkingTree", "path": "src/a.rs", "change": "01".repeat(20) },
                { "scope": { "PullRequest": 12 }, "path": "src/a.rs", "change": "02".repeat(20) },
            ]),
            "the data-model shape"
        );
        assert_eq!(
            value["entries"][0].get("marks"),
            None,
            "an entry without marks writes no `marks`"
        );
        let back = ReviewFile::from_json(&json).expect("loads");
        assert_eq!(back.marks, file.marks);
        assert_eq!(back.entries["feature-x"], file.entries["feature-x"]);
    }

    #[test]
    fn an_entry_with_marks_and_no_comments_is_written_with_no_comments() {
        let value: serde_json::Value = serde_json::from_str(&with_marks().to_json()).expect("JSON");
        let entries = value["entries"].as_array().expect("a list");
        let only = entries
            .iter()
            .find(|e| e["location"] == json!({ "worktree": "marks-only" }))
            .expect("the entry with marks only is written");
        assert_eq!(only["comments"], json!([]));
        assert_eq!(only["marks"][0]["path"], json!("b.rs"));
    }

    #[test]
    fn without_marks_the_version_stays_3_or_2() {
        let mut file = sample();
        file.marks.insert("feature-x".into(), Vec::new());
        let version = |file: &ReviewFile| {
            serde_json::from_str::<serde_json::Value>(&file.to_json()).expect("JSON")["version"]
                .clone()
        };
        assert_eq!(version(&file), json!(2), "no marks, no PR comment");
        file.entries
            .get_mut("feature-x")
            .expect("entry")
            .push(pr_comment(5));
        assert_eq!(version(&file), json!(3), "no marks, a PR comment");
    }

    #[test]
    fn versions_1_to_3_load_with_no_marks_and_every_comment() {
        for version in [1, 2, 3] {
            let mut comment = stored_comment(ID_1, json!({ "draft": null }));
            if version < 3 {
                comment.as_object_mut().expect("object").remove("pr");
            }
            let file = ReviewFile::from_json(&file_of(version, vec![comment])).expect("loads");
            assert!(file.marks.is_empty(), "version {version} has no marks");
            assert_eq!(file.entries["feature-x"].len(), 1, "version {version}");
        }
    }

    fn file_with_mark(mark: serde_json::Value) -> String {
        json!({
            "version": 4,
            "entries": [{ "location": "default", "comments": [], "marks": [mark] }]
        })
        .to_string()
    }

    #[test]
    fn a_malformed_mark_makes_the_file_unreadable() {
        let good = json!({ "scope": "WorkingTree", "path": "a.rs", "change": "ab".repeat(20) });
        assert!(ReviewFile::from_json(&file_with_mark(good)).is_ok());
        for bad in [
            json!({ "scope": "WorkingTree", "path": "a.rs", "change": "ab".repeat(19) }),
            json!({ "scope": "WorkingTree", "path": "a.rs", "change": "zz".repeat(20) }),
            json!({ "scope": { "PullRequest": 0 }, "path": "a.rs", "change": "ab".repeat(20) }),
        ] {
            assert!(
                ReviewFile::from_json(&file_with_mark(bad.clone())).is_err(),
                "{bad} is refused"
            );
        }
    }

    #[test]
    fn a_file_with_a_malformed_mark_is_kept_aside() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = store_in(dir.path());
        let project = Path::new("/repo");
        let path = store.reviews_path(project);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        let text =
            file_with_mark(json!({ "scope": "WorkingTree", "path": "a.rs", "change": "not hex" }));
        std::fs::write(&path, &text).expect("written");
        assert_eq!(store.load_reviews(project), ReviewFile::default());
        let mut aside = path.as_os_str().to_os_string();
        aside.push(".corrupt");
        assert_eq!(std::fs::read_to_string(&aside).ok(), Some(text));
    }
}
