# Changelog

## [0.6.0](https://github.com/jaroslawherod/micold-ai-ide/compare/micold-ai-ide-v0.5.0...micold-ai-ide-v0.6.0) (2026-08-04)


### Features

* **fixture:** script the 20-worktree reference scene ([61e0183](https://github.com/jaroslawherod/micold-ai-ide/commit/61e01833e32e99d898b60420e65f25308840f053))
* **material:** add Ellipsized::at_role, and open the library to the crate ([31bffc4](https://github.com/jaroslawherod/micold-ai-ide/commit/31bffc49fcc6873342c5ba269d671e5b97b96b10))
* **perf:** add the frame-time probe T000z measures with ([b4a31b9](https://github.com/jaroslawherod/micold-ai-ide/commit/b4a31b90a3408eb40d3a2f61d561817b0ac2833b))
* **perf:** compose and verify the reference scene, and capture T000z ([a66274e](https://github.com/jaroslawherod/micold-ai-ide/commit/a66274efc07bac397f754932efc506957621bf34))
* **showcase:** add the component showcase gallery ([be06341](https://github.com/jaroslawherod/micold-ai-ide/commit/be06341ca116279cf6051d1be07db4491709da3b))
* **tokens:** author the type, elevation, shape, state, motion and density scales ([ac8c5d9](https://github.com/jaroslawherod/micold-ai-ide/commit/ac8c5d9aae0cdfc9b465bf95d29bbbf028bd6562))
* **tokens:** re-author the palette on the Material 3 baseline ramps ([02c873d](https://github.com/jaroslawherod/micold-ai-ide/commit/02c873d4aa278557938ffb7e4ddedba9a79e03ed))
* **type:** ship Roboto and give type roles weight and line height ([471bda5](https://github.com/jaroslawherod/micold-ai-ide/commit/471bda58ab325d4e43bf1efac4550401acfcf49d))
* **ui:** components own the transitions they play (feature 017, phase 6) ([84eb133](https://github.com/jaroslawherod/micold-ai-ide/commit/84eb1330663f608fd9cba7b74f6bb248d516b54b))
* **ui:** give each row its own hover reveal ([a9e4de1](https://github.com/jaroslawherod/micold-ai-ide/commit/a9e4de17d5ab6974355dce4b264a809cd4f36a5f))
* **ui:** give surfaces real depth — tone and shadow instead of outlines ([709aaed](https://github.com/jaroslawherod/micold-ai-ide/commit/709aaed3c5db626b5cce69d606ea0186c4fe2059))
* **ui:** let components own the transitions they play ([f84c960](https://github.com/jaroslawherod/micold-ai-ide/commit/f84c960145632a2c9aecec8f191acbda40ab7ecd))
* **ui:** let the sidebar own its drawer and its edge ([4eb4ab5](https://github.com/jaroslawherod/micold-ai-ide/commit/4eb4ab51aa89294bbefbb24bccd7ec77b69ecff8))
* **ui:** let the sidebar own its drawer and its edge (feature 017, T039b/T041/T042) ([fc274be](https://github.com/jaroslawherod/micold-ai-ide/commit/fc274be43ad0b3206c0f1d1146821905c7218c2f))
* **ui:** make every control respond to the pointer and the keyboard ([82ebfca](https://github.com/jaroslawherod/micold-ai-ide/commit/82ebfca0b15cf786d4e2c6e02cdddd4e9e0d7e2f))


### Bug Fixes

* **ci:** pin the showcase-gate step to bash so Windows parses it ([f630cea](https://github.com/jaroslawherod/micold-ai-ide/commit/f630cea20db4deeecb083c91e59f6d4bf978c03b))
* **client:** end the read-only state when the daemon says we hold the project ([b34592d](https://github.com/jaroslawherod/micold-ai-ide/commit/b34592dcce4475fcbac5978b37de2b12904b8bda))
* **input:** let a restarted client drive the sessions it did not start ([3590594](https://github.com/jaroslawherod/micold-ai-ide/commit/3590594075636630a86aca6daa151ad7feb83019))
* **material:** lay out an animated size, so Expand's reveal clips (BUG-001) ([7401f5d](https://github.com/jaroslawherod/micold-ai-ide/commit/7401f5dba7d837794a07e5ac338c7a196421cc66))
* **test:** wait on the daemon instead of sleeping at it ([cfe2174](https://github.com/jaroslawherod/micold-ai-ide/commit/cfe2174354a78562de258fa3a42745c337ed3362))
* **ui:** end an over-long sidebar name in an ellipsis ([fbc0a06](https://github.com/jaroslawherod/micold-ai-ide/commit/fbc0a06e93a33f9c0b7229948a590489cd05471b))
* **ui:** end an over-long sidebar name in an ellipsis ([cb7f126](https://github.com/jaroslawherod/micold-ai-ide/commit/cb7f126eecc51079e1e30dab687e5e01f2307890))
* **ui:** remove the obsolete check_circle icon from session rows ([#32](https://github.com/jaroslawherod/micold-ai-ide/issues/32)) ([18f1603](https://github.com/jaroslawherod/micold-ai-ide/commit/18f160334c9f1ab01899d2b09dc694d28c39f3c8))
* worktree session stuck as a plain terminal, and deletes that fake success ([#57](https://github.com/jaroslawherod/micold-ai-ide/issues/57)) ([6fa392a](https://github.com/jaroslawherod/micold-ai-ide/commit/6fa392afcf93234328d2932b125c1ff97eec6111))
* **worktree:** a delete blocked by foreign-owned files is partial success, not failure ([#61](https://github.com/jaroslawherod/micold-ai-ide/issues/61)) ([eaaf216](https://github.com/jaroslawherod/micold-ai-ide/commit/eaaf21624b2339ecc01fb4b3f15140af16fa83a2))

## [0.5.0](https://github.com/jaroslawherod/micold-ai-ide/compare/micold-ai-ide-v0.4.0...micold-ai-ide-v0.5.0) (2026-07-27)


### Features

* **010:** daemon session persistence — US2 + US4–US7 (attach/activity, supervision, exclusivity, contract recovery, logout) ([34bd890](https://github.com/jaroslawherod/micold-ai-ide/commit/34bd890b0d9aab0228a7c5ecd6219a7f9432f02e))
* **client:** per-session activity badge + live titles in the sidebar (feat 010 US2 T048/T049) ([3ecfcd8](https://github.com/jaroslawherod/micold-ai-ide/commit/3ecfcd837fea294274a5115c81a71dad9e7ff9d5))
* **daemon:** loopback activity-hook receiver + per-session --settings wiring (feat 010 US2 T045/T046) ([b690ef5](https://github.com/jaroslawherod/micold-ai-ide/commit/b690ef5b47f59891174e43c1df611910b15766f3))
* **daemon:** process-tree teardown + US4 docs (feat 010 US4 T061/T063) ([a39db33](https://github.com/jaroslawherod/micold-ai-ide/commit/a39db3321fbd27b15715b79c35330da3129b275c))
* **daemon:** PtySession exit detection — reap + classify clean/crash (feat 010 US4 T060) ([416f9c3](https://github.com/jaroslawherod/micold-ai-ide/commit/416f9c31fdec7aa6f623450058b9181efa88c460))
* **daemon:** reset a surviving restart to Running — close the L5 gap (feat 010 US4) ([25e96c5](https://github.com/jaroslawherod/micold-ai-ide/commit/25e96c5accf19ff47fed957ede1458db2f0026ff))
* **daemon:** restart supervision policy — clean-exit vs crash-loop (feat 010 US4 T060 policy) ([450dac3](https://github.com/jaroslawherod/micold-ai-ide/commit/450dac30f319b88838183b27c61b1f64eb8b7cc8))
* **daemon:** unattended restart supervision loop + respawn (feat 010 US4 T060/T058/T059/T062) ([188f900](https://github.com/jaroslawherod/micold-ai-ide/commit/188f90060bbd7b79f45c56eefa72303e181be55a))
* **daemon:** wire the activity FSM + OSC title into the session projection (feat 010 US2 T046/T047) ([7b72bfe](https://github.com/jaroslawherod/micold-ai-ide/commit/7b72bfe24d7282e0d2239a12e339a05a5f0719ca))
* **diagnostics:** daemon diagnostics surface + log-event/redaction tests (feat 010 T080/T080a/T081) ([66c74ff](https://github.com/jaroslawherod/micold-ai-ide/commit/66c74ff687e9dcaad5c6c8fc15a30e10503c1fd2))
* **ui:** add the per-instance animation primitive ([1f898e6](https://github.com/jaroslawherod/micold-ai-ide/commit/1f898e6d6d33456fe83802534bfbef81a96e95de))
* **ui:** close the component boundary ([2193375](https://github.com/jaroslawherod/micold-ai-ide/commit/21933755874f8b2bbc0f9be5260018165da65626))
* **ui:** consolidate five floating surfaces onto one overlay primitive ([4b54f41](https://github.com/jaroslawherod/micold-ai-ide/commit/4b54f41b5c2d979a81bf1aeeb56ac4b4a00e369b))
* **ui:** Material component architecture (feature 017) ([629d135](https://github.com/jaroslawherod/micold-ai-ide/commit/629d135000f583e2b78f4825c59bac66b243ff55))
* **ui:** migrate the settings and folder-browser dialogs onto the library ([1c23acc](https://github.com/jaroslawherod/micold-ai-ide/commit/1c23accf6ebb4902a2ba18e6915d15e0a325552d))
* **ui:** migrate the shell and window chrome onto the library ([5791bc9](https://github.com/jaroslawherod/micold-ai-ide/commit/5791bc9416eca85c8b74cc06967bd42933d6cae9))
* **ui:** migrate the sidebar and terminal pane onto the library ([022a001](https://github.com/jaroslawherod/micold-ai-ide/commit/022a00140566c2eaf718cb01823858a192fa0354))
* **ui:** wrap the rendering stack behind the component library ([29987c6](https://github.com/jaroslawherod/micold-ai-ide/commit/29987c6846979545daa4fcf7441afc982dc41e7b))
* **us5:** exclusivity tests + client keepalive, banner & takeover (feat 010 US5) ([ce87145](https://github.com/jaroslawherod/micold-ai-ide/commit/ce8714521a20a06370fa73d42c64c9f762f95d74))
* **us6:** interrupted-resumable sessions + version-mismatch restart (feat 010 US6) ([11174e3](https://github.com/jaroslawherod/micold-ai-ide/commit/11174e3be0f9c4383f7dcc84d934d5d2da6556e0))
* **us7:** Linux logout survival via systemd user linger (feat 010 US7) ([0a3846c](https://github.com/jaroslawherod/micold-ai-ide/commit/0a3846cfe394dafd495754fe87a6a5f8f269903f))
* **worktree:** name the creation step in progress (FR-024) + review fixes ([a695f49](https://github.com/jaroslawherod/micold-ai-ide/commit/a695f49b010865a86f5f5874ddd0afefed69ceb1))
* **worktree:** name the creation step in progress (FR-024) + review fixes ([fede676](https://github.com/jaroslawherod/micold-ai-ide/commit/fede676e474b561c53c4c0eda26387e0e3037458))
* **worktree:** reuse or overwrite an existing branch when creating a worktree ([f0b5e54](https://github.com/jaroslawherod/micold-ai-ide/commit/f0b5e54e6e6f9d4b8e6fdf301464246540775339))
* **worktree:** reuse or overwrite an existing branch when creating a worktree ([4e06862](https://github.com/jaroslawherod/micold-ai-ide/commit/4e06862efdbd3ef8075e6d88bbd6a00b6667671b))


### Bug Fixes

* **001-app-shell-about:** close convergence gaps ([c5e565c](https://github.com/jaroslawherod/micold-ai-ide/commit/c5e565c9278232a508b2c04a160b978585a97baf))
* **002-project-workspace-management:** surface settings save failures ([93e6fd4](https://github.com/jaroslawherod/micold-ai-ide/commit/93e6fd4a75d06d7699270dd272cbc90f2a0a82a3))
* **005-worktree-session-terminal:** restore durable archive marker in the daemon ([06e9aa5](https://github.com/jaroslawherod/micold-ai-ide/commit/06e9aa5d36279db43ac180db66a62b081cd6684c))
* **006-real-terminal-emulator:** reconnect the focus-gate to its tested pure function ([d011db2](https://github.com/jaroslawherod/micold-ai-ide/commit/d011db2e266e5b09dbdaf0ccf829493bbdc3e941))
* **008-background-project-switching:** detect background restarts again ([f7c309d](https://github.com/jaroslawherod/micold-ai-ide/commit/f7c309d9cce4c3500a1762ac089718debd3a93ed))
* **010-submodule-worktree-support:** stop discarding the worktree-create error detail ([7a2c86a](https://github.com/jaroslawherod/micold-ai-ide/commit/7a2c86abd146db270c07551c88566896ea42a2c2))
* **013-create-worktree-refinement:** wire the keep/delete-branch choice to the daemon ([565dbe9](https://github.com/jaroslawherod/micold-ai-ide/commit/565dbe9f3f302bf24fa46699f64adfb7eaeb9c80))
* **client:** draw the session activity badge from the shared icon vocabulary ([bbbc68a](https://github.com/jaroslawherod/micold-ai-ide/commit/bbbc68a856feb75232c1e3fd099ff52bc2a03e7b))
* **client:** draw the session activity badge from the shared icon vocabulary ([380b1f8](https://github.com/jaroslawherod/micold-ai-ide/commit/380b1f88c48304d5356b5071e3a4ccb091fa71be))
* close implementation drifts found via /speckit-converge retrofit sweep ([ca5efe7](https://github.com/jaroslawherod/micold-ai-ide/commit/ca5efe7a36cc50039e7d78651235ea40119910ab))
* **daemon:** detect a stale daemon after a same-contract .deb upgrade ([#23](https://github.com/jaroslawherod/micold-ai-ide/issues/23)) ([bcd7d0a](https://github.com/jaroslawherod/micold-ai-ide/commit/bcd7d0a5baa9db722f2e20e37be6a4832b0abd1d))
* **daemon:** fold underline_color into the shadow-diff style key ([e44c1e1](https://github.com/jaroslawherod/micold-ai-ide/commit/e44c1e17fdb3aee0dea11e38daa04217a1cc61aa))
* **daemon:** resolve env-include for daemon-spawned sessions ([f3c63c8](https://github.com/jaroslawherod/micold-ai-ide/commit/f3c63c8367b9fba97a1e9ee64e9353520868cbf1))
* **daemon:** resolve env-include for daemon-spawned sessions (BUG-003) ([64e9d7f](https://github.com/jaroslawherod/micold-ai-ide/commit/64e9d7f6718f3218259cc721d440f5bd8e80330f))
* **daemon:** wrap Claude Code hook entries in the required matcher/hooks group ([42a9bfb](https://github.com/jaroslawherod/micold-ai-ide/commit/42a9bfb9f566dd3f8adcab382e29d22530bcc99f))
* **deb:** resolve packaging/asset paths relative to the crate dir ([9114284](https://github.com/jaroslawherod/micold-ai-ide/commit/91142842af2eefbf29c812d1982fbcf697be1fea))
* **deb:** resolve packaging/asset paths relative to the crate dir ([98f5be7](https://github.com/jaroslawherod/micold-ai-ide/commit/98f5be75624a19f39ba23cd10758161f52f0e46b))
* **protocol:** carry NamedColor discriminant as u16 so default cells aren't red ([7de5a11](https://github.com/jaroslawherod/micold-ai-ide/commit/7de5a11cbc55e1f5cdbbb56ab0658d0f7226ea8d))
* **release:** unbreak release-please after the workspace split ([5c2d6cd](https://github.com/jaroslawherod/micold-ai-ide/commit/5c2d6cd76cea1074348408f62642cb0cee356aeb))
* **release:** unbreak release-please after the workspace split ([34adc39](https://github.com/jaroslawherod/micold-ai-ide/commit/34adc39054a114bbdfbc1ff43a3cec23fe5a2273))


### Miscellaneous Chores

* release micold-ai-ide 0.5.0 ([d73372d](https://github.com/jaroslawherod/micold-ai-ide/commit/d73372dc729ae580f9ab723e8fe0cc857a184073))

## [0.4.0](https://github.com/jaroslawherod/micold-ai-ide/compare/micold-ai-ide-v0.3.0...micold-ai-ide-v0.4.0) (2026-07-25)


### Features

* add optional environment-include script for sessions ([9110e1e](https://github.com/jaroslawherod/micold-ai-ide/commit/9110e1e479ecc9149ff73b3ad1b9e44216689644))
* **project:** forget a project and remove it from the list ([45f4279](https://github.com/jaroslawherod/micold-ai-ide/commit/45f4279ba5fa6c181da9c8bc905a504435f0d90e))
* **project:** let the user forget a project and remove it from the list ([50ea068](https://github.com/jaroslawherod/micold-ai-ide/commit/50ea0689e5b1785774dbc7ef5dcead13f5f60882))
* **switcher:** forget a project from its right-click context menu ([5541a75](https://github.com/jaroslawherod/micold-ai-ide/commit/5541a758138a23b646b1d1fdbb105ae4cc1e9e5a))
* **terminal:** allow multiple Regular Terminal instances per session ([07988ec](https://github.com/jaroslawherod/micold-ai-ide/commit/07988ec1c764b10b09cb617a03b469912967d31c))
* **terminal:** redesign the instance switcher as tabs and fix session-id addressing ([2836151](https://github.com/jaroslawherod/micold-ai-ide/commit/2836151cfdbbb99a3cca988ab4cf7ad566fae0d2))
* **worktree:** hide agent-created worktrees from the sidebar ([9aad123](https://github.com/jaroslawherod/micold-ai-ide/commit/9aad1237c80ab15f645b13b5e8bc2d2ac391053c))
* **worktree:** hide agent-created worktrees from the sidebar ([8ddf591](https://github.com/jaroslawherod/micold-ai-ide/commit/8ddf591e496f0efb2dea57ec0eca4fbedd1002ce))
* **worktree:** Material select, branch-delete choice, and staged creation progress ([c3ebc6e](https://github.com/jaroslawherod/micold-ai-ide/commit/c3ebc6e6c5ae4fd74e3eb58f2d3e2b893d2374e6))


### Bug Fixes

* **env-include:** resolve the shell environment in the session's own directory ([2862bab](https://github.com/jaroslawherod/micold-ai-ide/commit/2862babb57628c891d5807142bbe2bad115e0b91))
* **env-include:** satisfy interactive-guard scripts when sourcing on Unix ([6150be3](https://github.com/jaroslawherod/micold-ai-ide/commit/6150be361bd3888a6d363fbe3657723ffd69440e))
* include env_include fields in test_app() helper ([91f88da](https://github.com/jaroslawherod/micold-ai-ide/commit/91f88da816f479da3d662b22cf17a5e3a4419400))
* **state:** close two data-loss gaps found by broader review of the diff ([d88c7a1](https://github.com/jaroslawherod/micold-ai-ide/commit/d88c7a1881c7c948c165530e723ddcc19d2ccbec))
* **state:** isolate storage faults per-project and stop reconciliation from resurrecting closed sessions ([93a0a08](https://github.com/jaroslawherod/micold-ai-ide/commit/93a0a08b17ebfec31fd283c3c43ef81eed9f3add))
* stop the app crashing on launch (theme subscription captured state) ([600c7d8](https://github.com/jaroslawherod/micold-ai-ide/commit/600c7d8dc11bab48acb742ba808be1bfeacadd07))
* **terminal:** accumulate sub-line wheel deltas so touchpads scroll ([0ce19ec](https://github.com/jaroslawherod/micold-ai-ide/commit/0ce19ecd81fc8907b10380365dac33b66e2070e8))
* **terminal:** stop ensure_attached_process from auto-respawning an exited instance ([3d6b564](https://github.com/jaroslawherod/micold-ai-ide/commit/3d6b56406b6adacd8c3bed48229eaefaadbf705f))
* **theme:** stop OS theme poll from panicking on startup ([6774993](https://github.com/jaroslawherod/micold-ai-ide/commit/67749937e176d514d5ffe9d9bb66b014e8559c1c))
* **theme:** stop transient dark_light::detect() timeouts from flashing the light theme ([650e701](https://github.com/jaroslawherod/micold-ai-ide/commit/650e701f668d7520b1d0f420e59bf1a0bebd1084))
* **ui:** use a close icon for the session close action ([05c8fe9](https://github.com/jaroslawherod/micold-ai-ide/commit/05c8fe9b4bc3e590a98564751902a7e9be7bfe07))
* **worktree:** compare worktree paths symlink-safely when removal fails ([c58da0a](https://github.com/jaroslawherod/micold-ai-ide/commit/c58da0aad7e0e59834e5940a1eef0563b2d8dfa8))
* **worktree:** mark deleted worktree's sessions archived so they can't resurrect ([7dc9c8a](https://github.com/jaroslawherod/micold-ai-ide/commit/7dc9c8a68b111230a4810a075840bfaf4d9cbfad))
* **worktree:** stop reporting a folder error on every successful delete ([86a4263](https://github.com/jaroslawherod/micold-ai-ide/commit/86a4263f38af22191993d58928d4767a2e0fc6f7))
* **worktree:** stop reporting a folder error on every successful delete ([7ed8e26](https://github.com/jaroslawherod/micold-ai-ide/commit/7ed8e26bc64d2c0ff1491c5f3ff57773e70b7996))

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
