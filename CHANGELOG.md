# Changelog

## [0.2.0](https://github.com/jaroslawherod/micold-ai-ide/compare/micold-ai-ide-v0.1.0...micold-ai-ide-v0.2.0) (2026-07-16)


### Features

* app icon (terminal-prompt mark) + iced window icon ([fe212a2](https://github.com/jaroslawherod/micold-ai-ide/commit/fe212a2f255efb0dd2578cfce8e41f5a41c9d779))
* application shell and project/workspace management ([1c4fa7f](https://github.com/jaroslawherod/micold-ai-ide/commit/1c4fa7f124acfe3f5a54308e901a8028debdbaed))
* Material Design icons across the application shell ([99cdd57](https://github.com/jaroslawherod/micold-ai-ide/commit/99cdd57e530bb8fb355f55453ea6788bc3d99cb8))
* Material Design layout and light/dark theming ([8c2c04d](https://github.com/jaroslawherod/micold-ai-ide/commit/8c2c04d6bc29a2887e09d18e268fe22b6e8b16b0))
* real terminal behavior for embedded session terminals (spec 006) ([4b22a3c](https://github.com/jaroslawherod/micold-ai-ide/commit/4b22a3ca377396caf7684101690e37cdefd3e883))
* **ui:** animated hover highlight on the sidebar resize handle ([e74bebf](https://github.com/jaroslawherod/micold-ai-ide/commit/e74bebfdb5aa8ef38b4142d0db729250155f548e))
* worktree & session navigation with embedded terminal (spec 005) ([6aff29b](https://github.com/jaroslawherod/micold-ai-ide/commit/6aff29b2c670a75c553c294e61cfa30a48334bfd))


### Bug Fixes

* **ci:** satisfy fmt + clippy and fix Windows rename path match ([95baecc](https://github.com/jaroslawherod/micold-ai-ide/commit/95baecccb2c62b69a37d3b026262a66b78bf0221))
* detect OS dark theme on GNOME/Ubuntu via dark-light 2.0 ([c94fb28](https://github.com/jaroslawherod/micold-ai-ide/commit/c94fb28b59f515a91a46d89c5c7b092abe711a4f))
* deterministic lexical path normalization for project identity ([ace3939](https://github.com/jaroslawherod/micold-ai-ide/commit/ace393946ca8885b0ca545e8100a92518b759645))
* **ui:** compact toolbar, tighter collapsed sidebar, remove terminal/separator gap ([da9b653](https://github.com/jaroslawherod/micold-ai-ide/commit/da9b6530d91313c0f7d9d52bb1361cedd072a1a6))

## Changelog

All notable changes to **Micold AI IDE** are recorded here.

This file is maintained automatically by [release-please](https://github.com/googleapis/release-please)
from [Conventional Commits](https://www.conventionalcommits.org/), and it is **embedded into the
application at build time** (`micold_ai_ide::metadata::CHANGELOG`) so the running app can show a
"What's new" view without any network access.
