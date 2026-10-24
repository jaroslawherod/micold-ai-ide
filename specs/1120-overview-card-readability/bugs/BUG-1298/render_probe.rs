    // ---- BUG-1298 render probe (scratch; appended inside `mod tests` of overview_card.rs, never
    // committed). `PROBE_OUT=<dir> cargo test -p micold-client --lib render_design_cards -- --ignored`
    // writes one PNG per design card and scheme, at the design's card width.
    #[allow(clippy::too_many_arguments)]
    fn probe_model(
        state: CardState,
        kind: LocationType,
        reason: CardReason,
        title: &str,
        branch: Option<&str>,
        link: Option<&str>,
        sessions: (u32, u32),
        activity: Option<&str>,
        progress: Option<(Phase, u32, u32)>,
        prs: (u8, u8),
    ) -> CardModel {
        let mut m = model(state, "x");
        m.kind = kind;
        m.reason = reason;
        m.title = title.into();
        m.display_name = branch
            .map(|b| b.replace('/', "-"))
            .unwrap_or_else(|| title.to_string());
        m.branch = branch.map(str::to_string);
        m.issue_link = link.map(|l| micold_core::issue_link::IssueLink {
            label: l.into(),
            address: "https://example.invalid".into(),
        });
        m.sessions = sessions;
        m.activity = activity.map(|line| micold_core::hook_activity::CurrentActivity {
            line: line.into(),
            ratio: None,
        });
        if let Some((phase, done, total)) = progress {
            m.progress = Progress {
                phase: Some(phase),
                ratio: Some(Ratio { done, total }),
                source: micold_core::task_progress::Source::Ledger,
            };
        }
        m.prs = if branch.is_some() {
            PrDisplay::Counts(micold_core::overview::PrCounts {
                open: prs.0,
                merged: prs.1,
                open_capped: false,
                merged_capped: false,
            })
        } else {
            PrDisplay::Hidden
        };
        m
    }

    #[test]
    #[ignore]
    fn render_design_cards() {
        use CardState::*;
        use LocationType::*;
        let out = std::path::PathBuf::from(std::env::var("PROBE_OUT").expect("PROBE_OUT"));
        let width: f32 = std::env::var("PROBE_WIDTH")
            .ok()
            .and_then(|w| w.parse().ok())
            .unwrap_or(246.0);
        let ci = "CI red on main: daemon tests fail to compile on aarch64 musl";
        let ci_branch = "fix/1117_ci-red-on-main-daemon-tests-fail-to-compile-on";
        let cards: Vec<(&str, CardModel, Vec<CardPrRow>, usize, bool)> = vec![
            ("p1-root", probe_model(Working, ProjectRoot, CardReason::Working, "micold-ai-ide", Some("main"), None, (1, 1), Some("prepare a redesign of overview"), None, (0, 0)), vec![], 0, false),
            ("p2-ci", probe_model(Working, Bugfix, CardReason::Working, ci, Some(ci_branch), Some("Issue #1117"), (1, 1), Some("/speckit-autopilot #1117"), None, (0, 0)), vec![], 0, false),
            ("p3-regr", probe_model(Working, Bugfix, CardReason::Working, "Regression: sidebar worktree row shows visual artefacts", Some("fix/1119_regression-sidebar-worktree-row-shows-visual"), Some("Issue #1119"), (1, 1), Some("/speckit-autopilot #1119"), None, (0, 0)), vec![], 0, false),
            ("p4-idle-prog", probe_model(Idle, Feature, CardReason::Idle, "Allow the GitHub issues integration to be disabled", Some("feat/1102_allow-the-github-issues-integration-to-be-disabled"), Some("Issue #1102"), (1, 0), None, Some((Phase::Pr, 21, 25)), (1, 0)), vec![pr_row(1121, PrMark::Open, Some("checks pending"))], 0, false),
            ("p5-idle-prs", probe_model(Idle, Bugfix, CardReason::Idle, "Container daemon fails to start with podman: \"invalid argument\"", Some("fix/1115_container-daemon-fails-to-start-with-podman-invalid"), Some("Issue #1115"), (1, 0), None, None, (1, 1)), vec![pr_row(1118, PrMark::Open, Some("checks failing")), pr_row(1103, PrMark::Merged, None)], 1, false),
            ("o1-action", probe_model(NeedsAction, Feature, CardReason::NeedsPermission, "Add dark mode to the settings page", Some("feat/1130_dark-mode"), Some("Issue #1130"), (2, 1), Some("Waiting for permission to run cargo test"), Some((Phase::Implement, 4, 10)), (0, 0)), vec![], 0, false),
            ("o2-attn", probe_model(NeedsAttention, Bugfix, CardReason::MergeConflict, "Tab key skips the branch field", Some("fix/1098_tab-key"), Some("Issue #1098"), (1, 0), None, None, (2, 0)), vec![pr_row(1110, PrMark::Open, Some("merge conflict")), pr_row(1107, PrMark::Draft, Some("no checks"))], 0, false),
            ("o3-done", probe_model(Done, Chore, CardReason::Merged, "Bump iced to 0.14", Some("chore/1087_bump-iced"), Some("MIC-87"), (0, 0), None, None, (0, 1)), vec![pr_row(1089, PrMark::Merged, None)], 0, false),
            ("f1-expanded", probe_model(Working, Bugfix, CardReason::Working, ci, Some(ci_branch), Some("Issue #1117"), (1, 1), Some("Rewriting the cfg gate in crates/micold-daemon/src/sock.rs so the aarch64 musl build compiles"), None, (2, 1)), vec![pr_row(1124, PrMark::Open, Some("checks passed")), pr_row(1120, PrMark::Draft, Some("checks pending")), pr_row(1116, PrMark::Merged, None)], 0, true),
        ];
        for (scheme, r) in [("light", roles()), ("dark", super::super::test_support::dark_roles())] {
            for (name, m, rows, more, expanded) in &cards {
                let mut mounted = Mounted::new_in(
                    container(
                        OverviewCard::<String>::new(m, r)
                            .on_press("open".to_string())
                            .on_issue_link("issue".to_string())
                            .on_pr_link(|n| format!("pr{n}"))
                            .pull_requests(rows.clone(), *more)
                            .expanded(*expanded),
                    )
                    .width(Length::Fixed(width)),
                    Size::new(width, if *expanded { 420.0 } else { 1000.0 }),
                );
                let (px, w) = mounted.drawn(style::color(r.surface));
                let h = px.len() as u32 / 4 / w;
                image::save_buffer(
                    out.join(format!("{scheme}-{name}.png")),
                    &px,
                    w,
                    h,
                    image::ExtendedColorType::Rgba8,
                )
                .unwrap();
                let text: Vec<String> = mounted
                    .painted_text()
                    .into_iter()
                    .map(|p| format!("{:.0}@{:.0},{:.0} {:?}", p.size, p.bounds.x, p.bounds.y, p.content))
                    .collect();
                std::fs::write(out.join(format!("{scheme}-{name}.txt")), text.join("\n")).unwrap();
            }
        }
    }
