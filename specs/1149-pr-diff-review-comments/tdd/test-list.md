# Test list: PR diff review comments (#1149)

Derived after the fact from the test tasks of `tasks.md`, each traced to spec.md scenarios and FRs. Verification: `verification.md`.

| Test task | Layer | Traces to | Status |
|---|---|---|---|
| T001 | core BranchPrState serde | FR-005 | written, passing |
| T002 | core comment model | FR-001,FR-005 | written, passing |
| T003 | core store v3 | FR-005 | written, passing |
| T004 | core protocol roundtrip | FR-005,FR-006 | written, passing |
| T006 | daemon review_edit | FR-001,FR-005,FR-015 | written, passing |
| T008 | client features_pr_changes | US1 s1-4, FR-001/003/004/013/014/015 | written, passing |
| T012 | core prompt build_pr | US2, FR-007/008/010 | written, passing |
| T014 | daemon review_send W20 | US2, SC-002, FR-006/010 | written, passing |
| T015 | daemon review_reports (characterization) | FR-011 | written, passing |
| T017 | client features_pr_changes | US2, FR-006/013 | written, passing |
| T019 | core git reads | FR-009 | written, passing |
| T020 | daemon review_send real git | US3, FR-009 | written, passing |
| T023 | core anchor/diff | FR-012, SC-004 | written, passing |
| T025 | client features_pr_changes | US4, FR-012/013 | written, passing |
