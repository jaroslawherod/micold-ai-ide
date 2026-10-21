# Quickstart: Overview cards, a link to the worktree's issue

## A — Automated

```sh
mise run test-core     # issue_link (K1–K3), JiraBaseUrl (K4), Expansion additions (K5), settings and wire round trips
scripts/gate.sh        # whole gate: fmt, clippy, workspace tests (client U1–U6, daemon jira_base_url_setting), layout snapshot
grep -nE 'tracing|log::|println' crates/micold-core/src/issue_link.rs   # FR-014: no output
```

Expected: all green; the grep prints nothing; `tests/layout_snapshot.rs` passes with the new
`overview-issue-link` state under its text-overflow and containment gates, and the re-blessed
`overview-open` anchors.

## B — Visual pass (showcase and client, both themes, scale 1.0 and 2.0)

Setup: a project whose `origin` is `github.com/o/r`, worktrees `feat-1122_overview-issue-link`,
`feat-mic-87_add-login`, `feat_no-ticket`, one with a 120-character branch, and the Jira integration
switch on; a second project, also with a `feat-mic-87_x` worktree, and no Jira connection on
either.

1. Showcase *Surfaces*: a card with "Issue #1122", collapsed and expanded, at 280/380/480 px: link
   in `primary`, whole, short branch ellipsized first; underline only under the pointer; pointing
   hand (FR-003, FR-015, SC-004).
2. Client Overview: "Issue #1122" on its card; none on Default and `feat_no-ticket`; none on
   `feat-mic-87…` until Settings → Jira → **Jira base URL** (the open project's, with the connection
   fields left blank) is set to `https://example.atlassian.net/`, then "Issue MIC-87" appears
   without a restart, also in a second window on the same project (Story 2, FR-010, FR-012). Open the
   second project: its `feat-mic-87_x` card has no link and its Settings field is empty; set it to
   `https://b.atlassian.net`: its link opens `https://b.atlassian.net/browse/MIC-87` while the first
   project's still opens `https://example.atlassian.net/browse/MIC-87` (Story 2 sc. 8).
3. Click each link: the browser opens `https://github.com/o/r/issues/1122` and
   `https://example.atlassian.net/browse/MIC-87`; the Overview stays, no ripple, nothing selected
   (FR-004, FR-005).
4. Rest the pointer on a collapsed card's link: it does not expand; move onto the title and rest: it
   expands; on the expanded card the link stays clickable (Story 4).
5. Press on the card, release on the link, and the reverse: nothing activates (Story 4 sc. 5).
   Right-click the link: the card menu opens.
6. Keyboard only: Tab to a card, Tab to its link (ring shown, card stays expanded), Enter opens it;
   arrows move card to card from the link; Escape collapses, focus stays; Menu key opens the card
   menu, Escape returns to the link (Story 3).
7. Settings: enter `http://x`, `https://u@x`, `https://x?q`: refused with the reason, nothing
   stored; clear the field: that project's Jira links go, the other project's stay (FR-011).
8. Turn the GitHub issues switch off: "Issue #1122" goes; on: back without a restart (FR-001, FR-012).
9. Break the opener (Linux: `PATH` without `xdg-open`): the click shows feature 031's notification
   naming the address and the reason (FR-013).
10. Full screen: links behave the same.
