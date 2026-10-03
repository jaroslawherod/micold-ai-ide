# Rubric: plan

- The Constitution Check table covers all eight principles. Each justified violation is in
  Complexity Tracking.
- Every functional requirement maps to a plan section, contract or data-model entity.
- Tech choices name real crates and modules in this workspace. The reviewer greps to confirm paths
  and types exist or are marked new.
- Local-first storage (Principle IV): nothing assumes a remote service.
- The test strategy says which layer tests each requirement (core unit, client, geometry gates,
  quickstart §B visual pass).
- research.md records each decision with its rejected alternatives.
