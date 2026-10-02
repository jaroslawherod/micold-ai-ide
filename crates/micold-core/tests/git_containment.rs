//! Whether a branch holds anything its merged pull request does not (feature 040, data-model §6,
//! contracts/reading-and-wire.md §3): the pure decision, the two git questions behind it against
//! a real repository, and the fake that scripts them.

use micold_core::git::containment;
use micold_core::protocol::messages::BranchContainment;

/// The last commit of a merged pull request, as GitHub reports it.
const HEAD: &str = "1111111111111111111111111111111111111111";
/// Any other commit.
const OTHER: &str = "2222222222222222222222222222222222222222";

/// U50. A branch that stands at the pull request's last commit, or behind it, holds nothing the
/// merge did not take (FR-015).
#[test]
fn a_tip_equal_to_the_head_or_an_ancestor_of_it_is_contained() {
    assert_eq!(
        containment(Some(HEAD), HEAD, None),
        BranchContainment::Contained,
        "a tip equal to the head is contained without asking git about ancestry"
    );
    assert_eq!(
        containment(Some(OTHER), HEAD, Some(true)),
        BranchContainment::Contained,
        "a branch behind its merged pull request is contained"
    );
}

/// U51. Commits made on the branch after the merge are work the merge did not take, so no removal
/// is suggested (FR-017).
#[test]
fn a_tip_that_is_not_an_ancestor_of_the_head_is_beyond() {
    assert_eq!(
        containment(Some(OTHER), HEAD, Some(false)),
        BranchContainment::Beyond,
        "a branch with commits after its merged pull request is beyond it"
    );
}

/// U52. What the repository cannot show is never read as "nothing newer" (FR-017): a branch that
/// does not exist locally has no tip, whatever was said about ancestry, and a pull request whose
/// last commit was never fetched leaves the ancestry unknown.
#[test]
fn no_tip_or_unknown_ancestry_is_unknown() {
    assert_eq!(
        containment(None, HEAD, None),
        BranchContainment::Unknown,
        "a branch that does not exist locally"
    );
    assert_eq!(
        containment(None, HEAD, Some(true)),
        BranchContainment::Unknown,
        "without a tip there is nothing an ancestry answer could be about"
    );
    assert_eq!(
        containment(None, HEAD, Some(false)),
        BranchContainment::Unknown,
        "without a tip there is nothing an ancestry answer could be about"
    );
    assert_eq!(
        containment(Some(OTHER), HEAD, None),
        BranchContainment::Unknown,
        "git could not say whether the tip is an ancestor of the head"
    );
}
