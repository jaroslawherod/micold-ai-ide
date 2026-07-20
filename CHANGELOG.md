# Changelog

## [0.3.0](https://github.com/jaroslawherod/micold-ai-ide/compare/micold-ai-ide-v0.2.0...micold-ai-ide-v0.3.0) (2026-07-20)


### Features

* **motion:** add generic framework-agnostic animation driver ([beda22e](https://github.com/jaroslawherod/micold-ai-ide/commit/beda22e3c2dfe630f5150daaa3cd185236b2ff6d))
* **packaging:** full desktop-launcher registration + mise deb task ([67c5b75](https://github.com/jaroslawherod/micold-ai-ide/commit/67c5b75a2d10ba191efea9c794d711eda1a7e627))
* **projects:** background project switching with top-bar switcher ([a8014c2](https://github.com/jaroslawherod/micold-ai-ide/commit/a8014c264d539cd2c0c3c31bd56f4761679aa29b))
* **session-location:** add ability to start sessions in project root (Feature 010) ([df6ab34](https://github.com/jaroslawherod/micold-ai-ide/commit/df6ab3466de29589139be57928fe277897fc776e))
* **session-location:** start a session in the project root without a worktree ([817af42](https://github.com/jaroslawherod/micold-ai-ide/commit/817af42b202f5f6f4d07af55336d1cfcac3268df))
* **sidebar:** copy worktree name to clipboard from the right-click menu ([fe71dcc](https://github.com/jaroslawherod/micold-ai-ide/commit/fe71dccf4d8bcb69eb2c875340c9f2a81ae2c50c))
* **sidebar:** move tag filtering behind a toolbar accordion, full-coverage icon font ([8387b24](https://github.com/jaroslawherod/micold-ai-ide/commit/8387b2475dd6e15934f84f62395c408b78a3e3fd))
* **sidebar:** worktree tags, filtering, rename/delete, hover actions ([c7f7581](https://github.com/jaroslawherod/micold-ai-ide/commit/c7f75815ba68efbee9cc700310ae5666aaca5e77))
* **terminal:** switch a session's terminal between claude and a regular shell ([da432ce](https://github.com/jaroslawherod/micold-ai-ide/commit/da432ce0c28a09a182f9a9da20dcf989baa626ab))
* **terminal:** switch a session's terminal between claude and a regular shell ([fc968c6](https://github.com/jaroslawherod/micold-ai-ide/commit/fc968c6c0fcfea84c440f479613b3df2d1f6ceb8))
* **terminal:** visible scrollbar + fix scrollback viewport mapping (FR-016) ([c6c7fd2](https://github.com/jaroslawherod/micold-ai-ide/commit/c6c7fd2231986dad069789dac68a2ff3d1152265))
* **ui:** display worktree creation progress in form log area ([bd7aed6](https://github.com/jaroslawherod/micold-ai-ide/commit/bd7aed64e0c80b84341770f87a54586f50141fe4))
* **ui:** overlay fade in/out and migrate animations onto the shared driver ([8b2385e](https://github.com/jaroslawherod/micold-ai-ide/commit/8b2385ec98d09f4470f7ac9ef6e0f9ab7fdf468d))
* **worktree:** auto-fetch git submodules on worktree create ([bcd4349](https://github.com/jaroslawherod/micold-ai-ide/commit/bcd43496eae1dc71c5cd08c807e1a6a9610df8cd))
* **worktree:** auto-fetch git submodules on worktree create ([d460ee4](https://github.com/jaroslawherod/micold-ai-ide/commit/d460ee4c85296fcca940ba1ae471a405b18bc8ca))


### Bug Fixes

* **ci:** remove stray blank line failing rustfmt check ([b34441b](https://github.com/jaroslawherod/micold-ai-ide/commit/b34441b08bad5bb05c0c736841304c83ba44b8d8))
* **ci:** repo-wide rustfmt drift and clippy -D warnings failures ([f0c3ec4](https://github.com/jaroslawherod/micold-ai-ide/commit/f0c3ec4d0acd0840bdfd92441ae7a398a225e6f2))
* **perf:** gate terminal/OS-theme polls on window focus to cut idle CPU ([182326c](https://github.com/jaroslawherod/micold-ai-ide/commit/182326c95a0484cfdb1e84cf98a1690eb7458276))
* **perf:** keep background terminal poll alive at a coarser cadence ([8c7ac4e](https://github.com/jaroslawherod/micold-ai-ide/commit/8c7ac4e350d1c9ce69219be55c370408de2f0b6a))
* **session-location:** address code-review findings from Feature 010 ([cc0aca9](https://github.com/jaroslawherod/micold-ai-ide/commit/cc0aca9e2a1dd8bf9dd29ee3b6d4e0a0bfea9823))
* **session:** sync session label with the AI CLI provider's session name ([4a66b5d](https://github.com/jaroslawherod/micold-ai-ide/commit/4a66b5dbcd5c6d5bc6822a53abb7c3cdbcc4d339))
* **sidebar:** code-review follow-ups from feature 008 ([e5638b2](https://github.com/jaroslawherod/micold-ai-ide/commit/e5638b29375e5d8c1be9a7101721c6c78d8dfad2))
* surface silent failures through a global notification surface ([ef39e2e](https://github.com/jaroslawherod/micold-ai-ide/commit/ef39e2ecee84624ea6e2384f5526ad869ec36b1c))
* terminal copy/mouse reporting + a global surface for silent failures ([ee08101](https://github.com/jaroslawherod/micold-ai-ide/commit/ee08101a5959eff5cd437662a54ece73c0ac91bf))
* **terminal:** anchor the mode toggle bottom-right, fix icon glyphs ([a5a131d](https://github.com/jaroslawherod/micold-ai-ide/commit/a5a131d4010e7bd826e62dfcc380658685afad85))
* **terminal:** auto-focus session terminal on select/start ([a46cdaa](https://github.com/jaroslawherod/micold-ai-ide/commit/a46cdaad8c3cad095ec381ac3779dbdb00f9679f))
* **terminal:** copy selections with line breaks and full scrollback ([94ea3e4](https://github.com/jaroslawherod/micold-ai-ide/commit/94ea3e4fae920d5000b10ac4b612816ac1b1da02))
* **terminal:** new sessions start filling the window, not just a fixed 30x100 area ([#3](https://github.com/jaroslawherod/micold-ai-ide/issues/3)) ([c4c6fb7](https://github.com/jaroslawherod/micold-ai-ide/commit/c4c6fb79ac1ea8707707c3a04d3e45bfd43e565e))
* **terminal:** render selection highlight and add right-click copy/paste menu (FR-013) ([7f075c4](https://github.com/jaroslawherod/micold-ai-ide/commit/7f075c47c9cdca0015dd61c83b6f920e0ee1add2))
* **terminal:** report mode-toggle spawn failures instead of dropping them ([ed3e436](https://github.com/jaroslawherod/micold-ai-ide/commit/ed3e436d56dad696e3eb64b26ced9a6160a1891d))
* **terminal:** report mouse release, motion, and middle/right buttons ([63e56ea](https://github.com/jaroslawherod/micold-ai-ide/commit/63e56ea1fb73dc12db5bf6be6dafbfeb222f12ee))
* **terminal:** stop scrollbar drag flicker by scrolling to an absolute offset ([4558055](https://github.com/jaroslawherod/micold-ai-ide/commit/455805544eac17832e16a79a2d6942f8317ae745))
* **theme:** keep tracking the OS theme while the window is unfocused ([39488fb](https://github.com/jaroslawherod/micold-ai-ide/commit/39488fbe829306c34b9c4f6e24013b9fe7af2042))
* **ui:** grey the glyph on a disabled IconButton ([d34e329](https://github.com/jaroslawherod/micold-ai-ide/commit/d34e329626aaa25f0d0bebd3de73892c02a29fa6))
* **worktree:** fix double-move of progress log in create() error path ([f108c17](https://github.com/jaroslawherod/micold-ai-ide/commit/f108c17e2c873e101f776b0648aba82e9275a90b))
* **worktree:** stream create progress instead of flushing it at the end ([97d5b3f](https://github.com/jaroslawherod/micold-ai-ide/commit/97d5b3f44ec0957b8ca641531f0be7c2273df78d))
* **worktree:** stream create progress instead of flushing it at the end ([3912eea](https://github.com/jaroslawherod/micold-ai-ide/commit/3912eea4d493bab92e3ed83da6290424db83315c))

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
