# T115 verification: §A is green on Linux, macOS and Windows

**Date**: 2026-08-26 · **Run**: [33003036028](https://github.com/jaroslawherod/micold-ai-ide/actions/runs/33003036028)
· **Branch**: `feat/run-daemon-inside-an-container-sandbox` (PR #235)
**Covers**: quickstart.md §A in full, on all three platforms, with no container runtime installed

## The run

```
assertion freeze                        pass    15s
build + test (macos-latest)             pass  1m52s
build + test (ubuntu-latest)            pass  3m27s
build + test (windows-latest)           pass  6m52s
ci complete                             pass     3s
classify change                         pass     7s
docs check                              pass     8s
fmt + clippy                            pass    48s
sandbox against a real runtime (linux)  pass  1m55s
```

The macOS and Windows legs install nothing: `cargo test -p micold-core --all-targets` plus the
enumerated render-free `micold-client` targets. `docker` is absent on both. That is Principle VI
checked rather than asserted — the fake `exec::CommandRunner` is what the whole §A table rests on,
and until this run it had never been compiled by a non-Linux toolchain.

## Why this took five runs, and what the first four found

T115's own entry recorded coverage as fixed and the run as still open, because the branch had never
had one: PR #235 was `CONFLICTING`, so GitHub could not compute a merge commit and no
`pull_request` workflow ever started. Merging `origin/main` is what produced the first execution.

Four of the five runs were red, and none of the four was a flake.

### 1. `sandbox-runtime` had no image to test against

Every target behind `sandbox-real-runtime` starts from `micold-daemon:dev`, the image
`mise run image` builds. A runner has no such image, so the first test to want it died on
`No such image: micold-daemon:dev` and cargo stopped — taking six untried targets with it and
reporting a single failure. The job now builds the image first (`mise run image` with the build
lock and the cross-compile dropped, both host-specific, and the glibc smoke test kept).

This is the one failure that was a gap in the harness rather than in the product. The three below
are defects in shipped code that only a non-Linux runner could see.

### 2. `quickstart_a_runs_everywhere.rs` could not read the file it checks

The gate splits `ci.yml` on `"\n  test:\n"`. A Windows runner checks out with `core.autocrlf=true`,
so the needle missed and the panic blamed the document: **"ci.yml has no `test:` job — that job *is*
the three-platform matrix"**, on a `ci.yml` that plainly has one.

### 3. `anatomy_call_sites.rs` mis-parsed ten files, and said so confidently

Same cause, different shape. That scan counts byte offsets line by line (`offset += line.len() + 1`)
while iterating with `lines()`, which strips the `\r`. Under CRLF the offset drifts by one byte per
line, the truncation point lands before the `#[cfg(test)]` it found, and the module is found a
second time in the tail. The report: **ten files "have more than one inline `#[cfg(test)]` module"**.
Each has one.

Both are fixed by one line in `.gitattributes` — `* text=auto eol=lf` — rather than by normalising
in each reader. Roughly thirty gates in this suite scan the repository's own text; a per-reader fix
is the enumerated list those gates exist to replace, and it would fall to whoever writes the
thirty-first. `git add --renormalize .` changes no file, so nothing in the tree was CRLF.

### 4. A Windows container path was half Linux and half not — FR-002, R2

The real one.

`pathmap::map_for` assembled `/mnt/host/c/Users/u/p` with `PathBuf::push`, and `PathBuf` is native
to whichever platform compiled it. On a Windows host that writes `\` separators, so the text handed
to `docker -v` was:

```
/mnt/host\c\Users/u/code/thing        (host "C:\Users\u\code\thing")
```

A Linux prefix with Windows separators. No Linux container accepts it, so **sandboxed mode was
broken on Windows outright** — not degraded, not mismapped, refused at `create`.

The mapping's own unit tests did not see it, and could not: they compare `PathBuf`s, and Windows
treats `/` and `\` as the same separator, so `PathBuf::from("/mnt/host/c/Users/u/p")` and the
pushed value are **equal** there. `a_windows_drive_letter_becomes_a_path_segment` passes on both
platforms whether the mapping is right or wrong.

What caught it is `sandbox_argv.rs::a_windows_host_mounts_every_path_under_the_container_root`,
whose assertions are on the *rendered argv strings*. T114 parameterised that suite over
`windows_host: bool` precisely so a Linux runner could exercise the Windows branch — and that was
the right move, but it is not sufficient: the branch it exercises is `map_for`'s, while the bug
lived in `PathBuf`'s platform-dependent rendering, which only a Windows toolchain produces. The
parameter carries the *logic* across platforms; it cannot carry the *standard library*.

The fix joins the container path as a `String`, and a new unit test asserts the rendered text
rather than the `PathBuf`, so the hiding place is closed rather than stepped around.

## What this does not cover

- **§B** is the manual pass and makes no platform claim; T120 owns it.
- **The Windows sandbox has still never been *run*.** This establishes that §A is green there and
  that the argv it produces is now well-formed. Whether a container started from a Windows host
  behaves is unverified, and there is no runner for it — the daemon-side-git argument in
  `pathmap.rs`'s module doc is the honest statement of what remains.
- The `micold-client` suite runs in full on Linux only. The render-free exceptions named in §A's
  table run everywhere, and `quickstart_a_runs_everywhere.rs` is what keeps that list honest.
