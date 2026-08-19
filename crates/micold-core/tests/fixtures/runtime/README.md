# Canned container-runtime output

Real output from Docker 29.5.1 (`overlayfs` driver, cgroup v2, x86_64 Linux) and the podman
equivalents, trimmed to the fields the parser reads. Tests replay these through
`sandbox::exec::RecordingRunner` so the parser is exercised against what the runtimes actually
print, not against what we assume they print.

The failure fixtures matter more than the success ones: each is a case that is awkward to arrange
against a live runtime, and each must map to a distinct `RuntimeError` variant (obligation C-6,
conformance check K-8).
