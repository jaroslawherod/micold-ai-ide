# TDD cycle log — #488 M3

M3's tests and code were written in one pass, not red-first. Red evidence below is retrospective: each behaviour's implementation was broken on purpose, the test run, and the break reverted (2026-10-09).

| Behaviour (task) | Break applied | Failing test (right reason) |
|---|---|---|
| Bound session resumes its own conversation, unbound starts fresh (T018/T019) | `launch_args_in` never returns `resume <id>` | `codex_provider::resume::a_bound_session_resumes_its_conversation_and_an_unbound_one_starts_fresh`, `…a_binding_whose_conversation_is_gone_can_be_replaced` |
| Bind only when exactly one candidate (T018, FR-006) | `sole_candidate` takes the first of several | `codex_provider::resume::only_exactly_one_candidate_is_ever_bound` |
| A shell session in the same folder must not block binding (review A F1, T021) | peer filter without the AI-CLI mode check | `codex_resume::a_shell_session_beside_it_does_not_hold_the_binding_back` ("timed out waiting for the binding") |

Not broken retrospectively (covered by the same passes, green): seam additions for claude/copilot/pi/fake (T016), name from first turn, bounded prefix, archive marker, daemon bind + restart resume, two same-folder sessions never cross-resume (T020).
