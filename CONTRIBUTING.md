# Contributing

Thanks for helping out. Bug reports, feature ideas and pull requests are all welcome.

## Set up

```sh
mise trust          # once per clone
mise run run        # run the GUI client
mise run test-core  # fast: render-free core only
mise run gate       # CI's merge gate locally — run before pushing
```

`mise tasks` lists every task. Linux needs the usual GUI dev libraries (X11/Wayland/xkbcommon);
see [`.github/workflows/ci.yml`](.github/workflows/ci.yml) for the package list.

## Pull requests

- Open an issue first for anything larger than a small fix, so the direction is agreed.
- Keep a PR to one change, with tests; `mise run gate` must pass.
- Search the crates for an existing helper before adding a new one — duplication is budgeted
  (`mise run duplication`).
- Use [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:` …);
  the changelog is generated from them.

## Where things live

- Architecture and build notes: [`docs/development/`](docs/development/)
- Per-feature specs: [`specs/`](specs/)
- User guide sources: [`docs/user-guide/`](docs/user-guide/)
