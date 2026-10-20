//! The stored shape of a project's review comments (feature 482, data-model § Core: persistence):
//! `reviews/<project id>.json` beside `projects/`, written and read by
//! [`crate::store::JsonFileStore::save_reviews`] and [`crate::store::JsonFileStore::load_reviews`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::comment::{fresh_id, CommentId, EntryReview, ReviewComment};

/// The newest version this build writes (feature 1149, S3): 3, and only while some comment has a
/// pull request; otherwise 2 (feature 946, S2), so a file without PR comments stays readable by an
/// older build. It also reads 482's version 1, whose `pending` state loads as Draft.
pub const REVIEW_FILE_VERSION: u32 = 3;

/// The version written while no comment has a pull request.
const PLAIN_FILE_VERSION: u32 = 2;

/// The versions this build reads.
const READABLE_VERSIONS: [u32; 3] = [1, PLAIN_FILE_VERSION, REVIEW_FILE_VERSION];

/// Every entry's comments of one project, keyed by worktree directory name; `""` is the Default
/// entry (the project root), as on the wire.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewFile {
    /// Comments per entry. An entry with none is not written.
    pub entries: BTreeMap<String, Vec<ReviewComment>>,
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
}

#[derive(Serialize, Deserialize)]
struct StoredReviews {
    version: u32,
    entries: Vec<StoredEntry>,
}

impl ReviewFile {
    /// The file's JSON text.
    pub fn to_json(&self) -> String {
        let entries = self
            .entries
            .iter()
            .filter(|(_, comments)| !comments.is_empty())
            .map(|(dir, comments)| StoredEntry {
                location: if dir.is_empty() {
                    StoredLocation::Default
                } else {
                    StoredLocation::Worktree(dir.clone())
                },
                comments: comments.clone(),
            })
            .collect();
        let with_pr = self
            .entries
            .values()
            .flatten()
            .any(|comment| comment.pr.is_some());
        let stored = StoredReviews {
            version: if with_pr {
                REVIEW_FILE_VERSION
            } else {
                PLAIN_FILE_VERSION
            },
            entries,
        };
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
        let entries = stored
            .entries
            .into_iter()
            .map(|entry| {
                let dir = match entry.location {
                    StoredLocation::Default => String::new(),
                    StoredLocation::Worktree(dir) => dir,
                };
                (dir, with_distinct_references(entry.comments))
            })
            .collect();
        Ok(Self { entries })
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
    use crate::review::comment::CommentState;
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
            ReviewFile::from_json(r#"{ "version": 4, "entries": [] }"#).is_err(),
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
        let text = file_of(4, Vec::new());
        std::fs::write(&path, &text).expect("written");
        assert_eq!(store.load_reviews(project), ReviewFile::default());
        let mut aside = path.as_os_str().to_os_string();
        aside.push(".corrupt");
        assert_eq!(
            std::fs::read_to_string(&aside).ok(),
            Some(text),
            "a version 4 file is quarantined as 482's"
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
}
