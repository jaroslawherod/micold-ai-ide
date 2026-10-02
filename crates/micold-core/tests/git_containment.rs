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
