# TDD cycle log — 483 parallel agent runs

No `tdd/test-list.md` was derived at design time; as in 482, the test-first task pairs of tasks.md
are the test list. Reds run against the stubs of commit `9f82d331` (they compile and answer
"nothing": `can_become` false, `RunGroup::new` Err, `validate` Ok, `derive_group` empty,
`RunsFile::to_json` empty / `from_json` Err, `JsonFileStore` runs methods no-ops).

## M1 — start N runs, group row (T001–T027)

| Behaviour | Tasks | Red (stub) | Green |
|---|---|---|---|
| Limits, `RunGroup::new`/`validate`, `can_become`, `GroupId` serde | T002 → T003 | `runs::tests`: 3 of 6 FAILED (`a_new_group_takes_two_to_eight_runs_numbered_from_one`, `a_loaded_group_refuses_no_runs_…`, `only_the_data_model_transitions_are_allowed`); the limits, the refusal of bad numbering (stub always Err), the accepted loaded shapes (stub always Ok) and `GroupId` serde pass on the stub by construction | |
| `derive_group` | T004 → T005 | `runs::naming::tests`: 4 of 5 FAILED (stub `Ok(vec![])`); the case-only test passes on the stub by construction (empty equals empty) | |
| `RunsFile` JSON, `runs_path`, load/save/remove | T006 → T007 | `runs::store::tests`: 7 of 7 FAILED | |
| Wire delta v36 | T008 → T009 | `protocol_roundtrip` did not compile before the wire edit (no `micold_core::runs`, no `RunGroupCreate`/`RunGroupPick`/`RunGroupDismiss`/`RunGroupsChanged`/`RunGroupCreated`/`RunPicked`; warm build log) | |
