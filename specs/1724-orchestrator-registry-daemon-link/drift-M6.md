# Plan drift, M6

| Path | Reason |
|---|---|
| `docs/user-guide/orchestrator-mode.md` | T043 asks the user guide to describe the whole feature: it gets the "never dials a daemon" paragraph. |
| `docs/SUMMARY.md` | Untouched: it already links the page (line 31, added in M1). |
| `quickstart.md` | At `specs/1724-orchestrator-registry-daemon-link/quickstart.md`, the feature directory, where T043 records the results. |
| `.github/workflows/ci.yml` | FR-008 netns proof (T042) needs strace and unprivileged user namespaces on ubuntu-24.04; it fails rather than skips in CI (Review A round 2 F1). |
