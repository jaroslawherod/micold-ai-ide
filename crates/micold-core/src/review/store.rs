//! The stored shape of a project's review comments (feature 482, data-model § Core: persistence):
//! `reviews/<project id>.json` beside `projects/`, written and read by
//! [`crate::store::JsonFileStore::save_reviews`] and [`crate::store::JsonFileStore::load_reviews`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::comment::ReviewComment;

/// The only version this build writes and reads.
pub const REVIEW_FILE_VERSION: u32 = 1;

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
        let stored = StoredReviews {
            version: REVIEW_FILE_VERSION,
            entries,
        };
        // Every field is a string, a number or a list of them: serialising cannot fail.
        serde_json::to_string_pretty(&stored).unwrap_or_default()
    }

    /// Read the file's JSON text; an error for anything unparseable or of another version.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let stored: StoredReviews = serde_json::from_str(text).map_err(|err| err.to_string())?;
        if stored.version != REVIEW_FILE_VERSION {
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
                (dir, entry.comments)
            })
            .collect();
        Ok(Self { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::comment::{CommentId, CommentState};
    use crate::review::{LineRange, RelPath, Side};
    use crate::store::JsonFileStore;
    use serde_json::json;
    use std::path::Path;
    use uuid::Uuid;

    fn comment(n: u128, state: CommentState) -> ReviewComment {
        ReviewComment {
            id: CommentId(Uuid::from_u128(n)),
            path: RelPath::from_git("src/a.rs"),
            side: Side::New,
            range: LineRange::new(12, 13).expect("12..=13"),
            quote: vec!["a".into(), "b".into()],
            text: "Why?".into(),
            state,
            created: 1_790_000_000,
        }
    }

    fn sample() -> ReviewFile {
        let mut file = ReviewFile::default();
        file.entries.insert(
            "feature-x".into(),
            vec![
                comment(1, CommentState::Pending),
                comment(2, CommentState::Sent { at: 1_790_000_100 }),
            ],
        );
        file.entries
            .insert(String::new(), vec![comment(3, CommentState::Pending)]);
        file
    }

    #[test]
    fn the_file_has_the_data_model_shape() {
        let value: serde_json::Value =
            serde_json::from_str(&sample().to_json()).expect("the file is JSON");
        assert_eq!(value["version"], json!(1), "the file says its version");
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
            ReviewFile::from_json(r#"{ "version": 2, "entries": [] }"#).is_err(),
            "a version this build does not know is refused rather than misread"
        );
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
