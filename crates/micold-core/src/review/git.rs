//! The Changes view's git reads (research R1): the commands, run through the helpers of
//! [`crate::git`], their output handed to the pure parsers of this module.

use std::io;
use std::path::Path;

use super::base::{Base, BaseUnavailable, ReviewScope, Toggles};
use super::changes::ChangeList;
use crate::git::GitCli;

impl GitCli {
    /// The base of the worktree at `dir` (R7).
    pub fn review_base(&self, _dir: &Path) -> Base {
        Base::Unavailable(BaseUnavailable::NoDefaultBranch)
    }

    /// The files changed in `dir` under `toggles` (R1, FR-002, FR-004, FR-005).
    pub fn change_list(
        &self,
        _dir: &Path,
        scope: ReviewScope,
        _toggles: Toggles,
    ) -> io::Result<ChangeList> {
        Ok(ChangeList {
            files: Vec::new(),
            scope,
        })
    }
}
