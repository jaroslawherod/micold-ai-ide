//! A Review tab's view as its first read answers it (#1274): changed files, their whole-change ids
//! and the open-then-list-read steps, shared by the review keys and the jump-to tests.

use std::path::PathBuf;

use micold_client::features::changes::{self, Effect, Msg, State};
use micold_core::review::base::{Base, ReviewScope};
use micold_core::review::changes::{
    ChangeIdentity, ChangeKind, ChangeList, ChangedFile, Content, Origin,
};
use micold_core::review::comment::{CommentId, CommentState, ReviewComment, Scope};
use micold_core::review::marks::{ChangeId, ReviewMark, WholeChange, WholeFile};
use micold_core::review::{LineRange, RelPath, Side};
use micold_core::session::SessionLocation;

pub fn path(p: &str) -> RelPath {
    RelPath::from_native(p).expect("relative path")
}

/// A modified text file at `p`, two lines added and two removed.
pub fn file(p: &str) -> ChangedFile {
    ChangedFile {
        path: path(p),
        kind: ChangeKind::Modified,
        content: Content::Text,
        added: 2,
        removed: 2,
        large: false,
        origin: Origin::Uncommitted,
        identity: ChangeIdentity::Unknown,
    }
}

/// A change id from one byte.
pub fn id(n: usize) -> ChangeId {
    ChangeId::parse(&format!("{:02x}", n % 256).repeat(20)).expect("an id")
}

/// The whole change of `files`, the n-th on [`id`]`(n + 1)`.
pub fn whole(files: &[ChangedFile]) -> WholeChange {
    WholeChange {
        files: files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let entry = WholeFile {
                    path: f.path.clone(),
                    kind: f.kind.clone(),
                    origin: f.origin,
                    id: Some(id(i + 1)),
                };
                (f.path.clone(), entry)
            })
            .collect(),
    }
}

/// Open `entry`'s Review tab of `project` in `state` and answer its first read with `files`.
pub fn open_into(
    state: &mut State,
    project: PathBuf,
    entry: SessionLocation,
    files: &[ChangedFile],
) {
    let seq = match changes::update(state, Msg::Opened { project, entry }) {
        Effect::ReadList { seq, .. } => seq,
        other => panic!("opening reads the list, got {other:?}"),
    };
    let scope = ReviewScope::Worktree {
        base: Base::MergeBase {
            branch: "main".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
        },
    };
    changes::update(
        state,
        Msg::ListRead {
            seq,
            whole: Ok(whole(files)),
            result: Ok(ChangeList {
                files: files.to_vec(),
                scope,
            }),
        },
    );
}

/// The service pushes `worktree_dir`'s review of `project`: `comments`, and `reviewed` marked on
/// their current ids, `all` being the listed files in [`whole`]'s order.
pub fn marked(
    state: &mut State,
    project: PathBuf,
    worktree_dir: &str,
    reviewed: &[&str],
    all: &[&str],
    comments: Vec<ReviewComment>,
) {
    let marks = reviewed
        .iter()
        .map(|p| {
            let at = all.iter().position(|q| q == p).expect("listed");
            ReviewMark {
                scope: Scope::WorkingTree,
                path: path(p),
                change: id(at + 1),
            }
        })
        .collect();
    changes::update(
        state,
        Msg::ReviewChanged {
            project,
            worktree_dir: worktree_dir.into(),
            comments,
            sending: false,
            marks,
        },
    );
}

/// A draft on `line` of `p`, made at `created`, saying `on <p>:<line>`.
pub fn draft(p: &str, line: u32, created: u64) -> ReviewComment {
    ReviewComment {
        id: CommentId::new(),
        path: path(p),
        side: Side::New,
        range: LineRange::new(line, line).expect("a range"),
        quote: vec![format!("l{line}")],
        text: format!("on {p}:{line}"),
        state: CommentState::Draft,
        created,
        pr: None,
    }
}
