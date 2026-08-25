# Canned container-runtime output

Real output from Docker 29.5.1 (`overlayfs` driver, cgroup v2, x86_64 Linux) and the podman
equivalents, trimmed to the fields the parser reads. Tests replay these through
`sandbox::exec::RecordingRunner` so the parser is exercised against what the runtimes actually
print, not against what we assume they print.

The failure fixtures matter more than the success ones: each is a case that is awkward to arrange
against a live runtime, and each must map to a distinct `RuntimeError` variant (obligation C-6,
conformance check K-8).

## Naming, and what each half is worth

`err_*.txt` is Docker's wording; `podman_err_*.txt` is podman's. They are separate files rather
than one shared file per failure precisely because the wording is the input — a classifier that
only recognises `Cannot connect to the Docker daemon` reports podman's daemon being down as
`Unknown`, which is exactly the anonymous failure FR-034 exists to prevent.

**Provenance differs between the two halves, and the difference is worth stating.** The Docker
fixtures were captured on the machine described above. Podman is not installed there, so the
podman fixtures were transcribed from podman's own message strings rather than captured from a
run: they are what podman prints, to the best of what its sources and documentation say, and they
have not been confirmed against a live podman. T098 — the quickstart pass against a real podman on
Linux — is where that confirmation happens, and it is the task that may correct these files.

Treating a transcribed fixture as if it were captured is the failure mode to avoid here: it would
let the classifier pass a suite built from our own assumptions. The transcription is a starting
point that keeps the *shape* of the suite honest across both runtimes until a real podman is
available.
