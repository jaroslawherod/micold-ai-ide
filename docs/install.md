# Installing Micold AI IDE

This page describes release **{{MICOLD_VERSION}}**.

Linux is the only supported platform. macOS and Windows support was dropped in #1897. The desktop
application and its `.deb` and `.rpm` packages were retired in #1959; the session service
(`micold-daemon`) and the orchestrator are built from source or run from the container images, and
the web client is described in [the web client page](user-guide/web-client.md).

## Build from source

You need [Rust](https://www.rust-lang.org/tools/install) (the version in `rust-toolchain.toml`,
which `rustup` installs for you) and `git`.

```console
$ git clone --branch {{MICOLD_TAG}} https://github.com/jaroslawherod/micold-ai-ide.git
$ cd micold-ai-ide
$ cargo build --release -p micold-daemon
```

The daemon binary lands in `target-shared/release/` (the workspace redirects cargo's output there).
