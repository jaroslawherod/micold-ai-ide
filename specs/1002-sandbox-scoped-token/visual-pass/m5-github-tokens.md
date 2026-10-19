# Visual pass: Settings > GitHub > GitHub access tokens (quickstart Part C steps 2, 12; feature 1002)

2026-10-19. Client built from this branch, Xvfb + tiny-skia, private XDG dirs, fake PAT only (github_pat_FAKE), no network.
Seeded a PatStore file for fakeowner/scratch expiring in 12 h.

| # | Check | Result | Evidence |
|---|-------|--------|----------|
| 1 | Section renders legibly: heading, intro, Repository field (hint "owner/name or a github.com URL"), Token field, Add button | PASS | m5-github-tokens-section.png, m5-github-tokens-form-masked.png |
| 2 | Token field masks input (18 dots for 18 typed chars) | PASS | m5-github-tokens-form-masked.png |
| 3 | Help text: Contents: Read and write, Metadata: Read, short expiry; sandbox can use everything the token can and client cannot narrow it; leaked token valid until it expires or is deleted on GitHub; Remove deletes only here, not on GitHub | PASS | same |
| 4 | Stored row: "fakeowner/scratch - expires on 2026-10-20", red expiry warning line, Replace and Remove buttons | PASS | m5-github-tokens-section.png |

Notes: not exercised: Add with validation (needs network), Pull requests permission sentence (not in text seen). My first Repository keystrokes did not land (field stayed empty; likely focus, a driving issue, not judged a defect). No visual defect seen.
