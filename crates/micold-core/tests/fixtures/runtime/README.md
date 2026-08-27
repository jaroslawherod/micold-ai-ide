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

**Both halves are captured now.** The Docker fixtures come from the machine described above. The
podman fixtures were transcribed from podman's own message strings
until T098, and are now captured from podman 5.8.4 (Fedora build, rootless, `crun`) running one
level in on that same machine — see `specs/027-sandboxed-daemon-runtime/evidence/us5-podman.md`
for the rig and for what its nesting does and does not establish.

The transcription was wrong in ways worth recording, because they are the ways a transcribed
fixture is always wrong:

- `podman_err_port_unavailable.txt` was written with `rootlessport` and Docker's `host:port`
  separator. Podman 5.8.4 rootless publishes through **pasta**, and writes `127.0.0.1/7727`. The
  classifier read the variant off it correctly and the port as `0`.
- `podman_err_mount_rejected.txt` carried Docker's `mkdir ...: read-only file system` wording.
  Podman reports the syscall it tried: `statfs <path>: no such file or directory`.
- `podman_err_no_subuid.txt` had the right sentence and the wrong status. Podman 5.8.4 emits it as
  a `level=error` log line and then **continues** with single mapping; the fatal failure arrives
  later, in the words now in `podman_err_subuid_range_too_small.txt`, which share nothing with it.
  That second file is the one the classifier could not read at all.
- `podman_err_service_down.txt` and `podman_err_permission_denied.txt` were close but incomplete:
  both real messages open with a client info block and the `Cannot connect to Podman` paragraph,
  and both end by repeating the socket URL. The permission one contains the not-running wording
  *as well as* `permission denied`, which is why `classify` tests the permission phrases first —
  with the transcribed fixtures that ordering was a precaution, and with these it is load-bearing.

Treating a transcribed fixture as if it were captured is the failure mode this section exists to
prevent: it lets the classifier pass a suite built from our own assumptions. Anything added here
for a runtime nobody has run should say so in this file until someone runs it.
