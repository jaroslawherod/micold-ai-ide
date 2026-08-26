#!/usr/bin/env bash
# Drives `scripts/check-assertions-frozen.sh` over crafted histories (feature 021, FR-027).
#
# Every case builds its own throwaway repository in a temp dir and deletes it afterwards, so the
# suite touches nothing real and cases cannot leak into one another.
#
# **The script under test stays OUTSIDE the fixture.** An earlier draft committed it into the
# fixture repository, so checking out a case's history also checked out whatever version of the
# script that commit carried -- and the first two cases silently exercised the buggy version they
# were written to catch. A gate's own test is exactly where that mistake is most expensive.
#
# Three things are pinned here. The first two are the ones the check has already got wrong once:
#
#   1. **Identity** (issue #146): an assertion is what it says, not which of its lines a diff
#      happened to touch. Reversing a multi-line expectation in place must fail.
#   2. **Scope** (spec.md Q3, task T074): FR-027 binds changes made for feature 021. Every other
#      feature owns its own expectations, and must be reported without being blocked.
#   3. **Adjudication** (task T074): a removal that has been read and judged not to be a relaxation
#      can be written down, and the entry is re-verified against the tree on every run. The cases
#      that matter are the ones holding the second half: a stale entry fails, an unheaded entry
#      fails, and one adjudication does not cover a different removal.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
CHECK="$ROOT/scripts/check-assertions-frozen.sh"

failures=0
cases=0

# A repository with one assertion-bearing test file and one source file, on a branch whose name
# says nothing about any feature.
new_repo() {
  local dir
  dir="$(mktemp -d)"
  git -C "$dir" init -q -b work
  git -C "$dir" config user.email t@example.com
  git -C "$dir" config user.name t
  git -C "$dir" config commit.gpgsign false
  mkdir -p "$dir/crates/thing/tests" "$dir/crates/thing/src" \
    "$dir/specs/021-mvu-slice-architecture" "$dir/specs/028-feature-encapsulation"
  cat > "$dir/crates/thing/tests/a.rs" <<'RS'
#[test]
fn it_starts_off() {
    let state = State::new();
    assert!(
        !state.enabled,
        "starts off"
    );
    assert_eq!(state.count, 0);
}
RS
  cat > "$dir/crates/thing/src/lib.rs" <<'RS'
#[cfg(test)]
mod tests {
    #[test]
    fn inline() {
        assert!(s.ready, "the thing is ready");
    }
}
RS
  git -C "$dir" add -A
  git -C "$dir" commit -qm seed
  echo "$dir"
}

commit_in() {
  git -C "$1" add -A
  git -C "$1" commit -qm "$2"
}

# run <name> <want_exit> <want_output_substring|-> <dir> <base> [env assignments...]
run() {
  local name="$1" want_exit="$2" want_out="$3" dir="$4" base="$5"
  shift 5
  cases=$((cases + 1))

  local out got_exit
  out="$(cd "$dir" && env "$@" "$CHECK" "$base" 2>&1)"
  got_exit=$?

  if [ "$got_exit" != "$want_exit" ]; then
    printf 'FAIL  %-52s want exit=%s got=%s\n' "$name" "$want_exit" "$got_exit"
    printf '%s\n' "$out" | sed 's/^/        | /' | head -6
    failures=$((failures + 1))
    return
  fi
  if [ "$want_out" != "-" ] && ! printf '%s' "$out" | grep -qF -- "$want_out"; then
    printf 'FAIL  %-52s want output ~ %s\n' "$name" "$want_out"
    printf '%s\n' "$out" | sed 's/^/        | /' | head -6
    failures=$((failures + 1))
    return
  fi
  printf 'ok    %-52s exit=%s\n' "$name" "$got_exit"
}

# Make the change in-scope for FR-027 the way a real feature-021 change is: by ticking its tasks.
claim_021() {
  printf -- '- [X] T0xx done\n' >> "$1/specs/021-mvu-slice-architecture/tasks.md"
}

# The same, for feature 028 -- whose FR-021 restates the freeze for its own duration. Two features
# can be frozen at once, and the check has to pick the right one to read adjudications from.
claim_028() {
  printf -- '- [X] T0xx done\n' >> "$1/specs/028-feature-encapsulation/tasks.md"
}

# --- identity: what an assertion says, not which lines moved (issue #146) ------------------------

# The case the line-based check passed and should not have. `assert!(` never changes, so it is
# diff context on neither side; the changed lines carry no `assert` token at all.
d="$(new_repo)"
sed -i 's/!state.enabled,/state.enabled,/; s/"starts off"/"starts ON now"/' "$d/crates/thing/tests/a.rs"
claim_021 "$d"; commit_in "$d" reverse
run "multi-line expectation reversed in place" 1 "removed and not reinstated" "$d" HEAD~1
rm -rf "$d"

d="$(new_repo)"
sed -i '/assert_eq!(state.count, 0);/d' "$d/crates/thing/tests/a.rs"
claim_021 "$d"; commit_in "$d" delete
run "assertion deleted outright" 1 "no near match survives" "$d" HEAD~1
rm -rf "$d"

# FR-027 admits relocation, and only relocation: the same assertion, some other file, both sides.
d="$(new_repo)"
sed -i '/assert_eq!(state.count, 0);/d' "$d/crates/thing/tests/a.rs"
printf '#[test]\nfn moved() {\n    assert_eq!(state.count, 0);\n}\n' > "$d/crates/thing/tests/b.rs"
claim_021 "$d"; commit_in "$d" relocate
run "assertion relocated to another file" 0 "intact" "$d" HEAD~1
rm -rf "$d"

d="$(new_repo)"
printf '#[test]\nfn extra() {\n    assert!(state.other);\n}\n' > "$d/crates/thing/tests/b.rs"
claim_021 "$d"; commit_in "$d" add
run "assertions added, none lost" 0 "added" "$d" HEAD~1
rm -rf "$d"

# Q3: `x.field` -> `x.field()` is an expectation change, not a mechanism rename. If this case ever
# goes green, someone widened norm() and reopened #146 from the other end.
d="$(new_repo)"
sed -i 's/s\.ready,/s.ready(),/' "$d/crates/thing/src/lib.rs"
claim_021 "$d"; commit_in "$d" rename
run "field -> method rename is not waived (Q3)" 1 "closest surviving" "$d" HEAD~1
rm -rf "$d"

# --- scope: FR-027 binds feature 021, not the repository (Q3, T074) ------------------------------

# Same edit as the first case, minus the tasks.md tick and on a branch naming no feature: another
# feature changing an expectation it owns. Reported in full, exit 0.
d="$(new_repo)"
sed -i 's/!state.enabled,/state.enabled,/; s/"starts off"/"starts ON now"/' "$d/crates/thing/tests/a.rs"
commit_in "$d" reverse
run "out of scope: reported, not enforced" 0 "reported, not enforced" "$d" HEAD~1
rm -rf "$d"

d="$(new_repo)"
sed -i 's/!state.enabled,/state.enabled,/' "$d/crates/thing/tests/a.rs"
commit_in "$d" reverse
run "out of scope: the loss is still listed" 0 'assert!(!state.enabled,"startsoff")' "$d" HEAD~1
rm -rf "$d"

# The branch-name signal, independent of the spec directory. CI checks out a detached merge commit
# on a pull request, so the name arrives by environment rather than from the repository.
d="$(new_repo)"
sed -i 's/!state.enabled,/state.enabled,/' "$d/crates/thing/tests/a.rs"
commit_in "$d" reverse
run "in scope by branch name" 1 "names feature 021" "$d" HEAD~1 FREEZE_BRANCH=feat/021-us4-thing
rm -rf "$d"

d="$(new_repo)"
sed -i 's/!state.enabled,/state.enabled,/' "$d/crates/thing/tests/a.rs"
claim_021 "$d"; commit_in "$d" reverse
run "in scope by touching the feature directory" 1 "specs/021-mvu-slice-architecture/" "$d" HEAD~1
rm -rf "$d"

d="$(new_repo)"
sed -i 's/!state.enabled,/state.enabled,/' "$d/crates/thing/tests/a.rs"
commit_in "$d" reverse
run "ASSERTION_FREEZE=enforce overrides out of scope" 1 "FAILED" "$d" HEAD~1 ASSERTION_FREEZE=enforce
rm -rf "$d"

d="$(new_repo)"
sed -i 's/!state.enabled,/state.enabled,/' "$d/crates/thing/tests/a.rs"
claim_021 "$d"; commit_in "$d" reverse
run "ASSERTION_FREEZE=report overrides in scope" 0 "reported, not enforced" "$d" HEAD~1 ASSERTION_FREEZE=report
rm -rf "$d"

d="$(new_repo)"
run "ASSERTION_FREEZE rejects an unknown mode" 2 "must be auto, enforce or report" "$d" HEAD ASSERTION_FREEZE=bogus
rm -rf "$d"

# --- scope: feature 028 freezes too, and scope decides which file is read (028 FR-021) -----------

# 028's restructuring renames assertion spellings by the thousand without changing an expectation.
# Out of scope it would report and exit 0, which is the failure mode the feature exists to correct.
d="$(new_repo)"
sed -i 's/!state.enabled,/state.enabled,/' "$d/crates/thing/tests/a.rs"
claim_028 "$d"; commit_in "$d" reverse
run "in scope by touching feature 028's directory" 1 "specs/028-feature-encapsulation/" "$d" HEAD~1
rm -rf "$d"

d="$(new_repo)"
sed -i 's/!state.enabled,/state.enabled,/' "$d/crates/thing/tests/a.rs"
commit_in "$d" reverse
run "in scope by a branch naming 028" 1 "names feature 028" "$d" HEAD~1 FREEZE_BRANCH=feat/028-encapsulation
rm -rf "$d"

# --- adjudications: the third option, and the rule that keeps it honest (T074) -------------------

# An adjudication file lives in the feature directory. Written here rather than by the fixture's
# seed because most cases must run without one -- the default has to be "no adjudications".
adjudicate() {
  local dir="$1" heading="$2"
  shift 2
  {
    printf '# Adjudicated removals\n\n## %s\n\n' "$heading"
    printf 'was: %s\n' "$@"
  } > "$dir/specs/021-mvu-slice-architecture/assertion-adjudications.md"
}

adjudicate_028() {
  local dir="$1" heading="$2"
  shift 2
  {
    printf '# Adjudicated removals\n\n## %s\n\n' "$heading"
    printf 'was: %s\n' "$@"
  } > "$dir/specs/028-feature-encapsulation/assertion-adjudications.md"
}

# The case T074 exists for: a removal read, judged not to be a relaxation, and written down.
d="$(new_repo)"
sed -i '/assert_eq!(state.count, 0);/d' "$d/crates/thing/tests/a.rs"
adjudicate "$d" "T0xx — the counter moved into the daemon" 'assert_eq!(state.count,0)'
claim_021 "$d"; commit_in "$d" adjudicated
run "an adjudicated removal passes" 0 "1 removal(s) adjudicated" "$d" HEAD~1
rm -rf "$d"

# ...and is *named* in the output, with the task that removed it. A silent subtraction would be a
# waiver by another name.
d="$(new_repo)"
sed -i '/assert_eq!(state.count, 0);/d' "$d/crates/thing/tests/a.rs"
adjudicate "$d" "T0xx — the counter moved into the daemon" 'assert_eq!(state.count,0)'
claim_021 "$d"; commit_in "$d" adjudicated
run "an adjudicated removal is named, with its task" 0 "T0xx — the counter moved into the daemon" "$d" HEAD~1
rm -rf "$d"

# The reason detect_scope hands back the directory and not just the sentence: an adjudication filed
# under 028 must be the one consulted when 028 is what is in scope. Reading 021's would leave every
# entry invisible and the removal unadjudicated, which is exit 1.
d="$(new_repo)"
sed -i '/assert_eq!(state.count, 0);/d' "$d/crates/thing/tests/a.rs"
adjudicate_028 "$d" "T0xx — the path moved into the feature struct" 'assert_eq!(state.count,0)'
claim_028 "$d"; commit_in "$d" adjudicated
run "an adjudication in 028's directory is the one read" 0 "1 removal(s) adjudicated" "$d" HEAD~1
rm -rf "$d"

# The rule that makes the file safe: an entry naming an assertion that is still in the suite is a
# place to bury the next removal, so it fails.
d="$(new_repo)"
adjudicate "$d" "T0xx — stale" 'assert_eq!(state.count,0)'
claim_021 "$d"; commit_in "$d" stale
run "an adjudication that stopped being true fails" 1 "are not missing from the suite" "$d" HEAD~1
rm -rf "$d"

# ...including when the change touched no assertion at all, which is the run a fast path would have
# skipped and the one a stale entry would otherwise survive forever.
d="$(new_repo)"
adjudicate "$d" "T0xx — stale" 'assert_eq!(state.count,0)'
claim_021 "$d"; commit_in "$d" stale
run "a stale adjudication fails even with nothing lost" 1 "not missing from the suite" "$d" HEAD~1
rm -rf "$d"

# ...but "no longer missing" is a claim about the *tree*, not about the diff, and judging it by the
# diff is what took this job red on main the hour it landed. Once the branch merges, the removal is
# behind the base as well as in front of it: absent from both sides, so it can never appear in a
# diff again, and every entry in the file turns stale the moment it lands -- on main, and on every
# branch cut from main afterwards, in scope or out, since the staleness verdict precedes the scope
# gate. The adjudication file would be unlandable by construction, correct only in the one commit
# that cannot exist.
d="$(new_repo)"
sed -i '/assert_eq!(state.count, 0);/d' "$d/crates/thing/tests/a.rs"
adjudicate "$d" "T0xx — the counter moved into the daemon" 'assert_eq!(state.count,0)'
claim_021 "$d"; commit_in "$d" adjudicated
run "an adjudication survives its own merge" 0 "intact" "$d" HEAD
rm -rf "$d"

# The same thing one commit later: an unrelated change, cut from the merge, that touches no
# assertion the file names and has no business being told about them.
d="$(new_repo)"
sed -i '/assert_eq!(state.count, 0);/d' "$d/crates/thing/tests/a.rs"
adjudicate "$d" "T0xx — the counter moved into the daemon" 'assert_eq!(state.count,0)'
claim_021 "$d"; commit_in "$d" adjudicated
printf '#[test]\nfn later() {\n    assert!(state.other);\n}\n' > "$d/crates/thing/tests/b.rs"
commit_in "$d" unrelated
run "a merged adjudication does not fail later branches" 0 "added" "$d" HEAD~1
rm -rf "$d"

# An entry with no heading has nobody's name on it, which is the thing the file exists to prevent.
d="$(new_repo)"
sed -i '/assert_eq!(state.count, 0);/d' "$d/crates/thing/tests/a.rs"
printf '# Adjudicated removals\n\nwas: assert_eq!(state.count,0)\n' \
  > "$d/specs/021-mvu-slice-architecture/assertion-adjudications.md"
claim_021 "$d"; commit_in "$d" unheaded
run "an adjudication under no heading is refused" 1 "no \`## \` heading" "$d" HEAD~1
rm -rf "$d"

# One adjudication does not cover a different removal.
d="$(new_repo)"
sed -i '/assert_eq!(state.count, 0);/d' "$d/crates/thing/tests/a.rs"
sed -i 's/!state.enabled,/state.enabled,/' "$d/crates/thing/tests/a.rs"
adjudicate "$d" "T0xx — the counter moved into the daemon" 'assert_eq!(state.count,0)'
claim_021 "$d"; commit_in "$d" partial
run "an unadjudicated removal still fails beside one" 1 "1 assertion(s) removed" "$d" HEAD~1
rm -rf "$d"

# --- refusing to pass vacuously ------------------------------------------------------------------

# A base with nothing under the watched paths must be an error, not a pass. This is the guard the
# old script's comment promised and never implemented.
d="$(mktemp -d)"
git -C "$d" init -q -b work
git -C "$d" config user.email t@example.com
git -C "$d" config user.name t
git -C "$d" config commit.gpgsign false
printf 'x\n' > "$d/README.md"; commit_in "$d" seed
printf 'y\n' >> "$d/README.md"; commit_in "$d" more
run "no assertions at the base is an error" 2 "vacuously" "$d" HEAD~1
rm -rf "$d"

d="$(new_repo)"
run "unknown base ref" 2 "not found" "$d" no/such/ref
rm -rf "$d"

echo
if [ "$failures" -ne 0 ]; then
  echo "check-assertions-frozen: $failures of $cases case(s) failed"
  exit 1
fi
echo "check-assertions-frozen: all $cases cases passed"
