//! The `ui` suite (#1981, FR-013): a placeholder for the web UI, whose Playwright tests will live
//! in `e2e/ui/`. Blocked on #1954; it skips, naming the issue, until real tests replace it.

#[test]
#[ignore = "placeholder: blocked on #1954 (web image); replace with real tests when it lands"]
fn ui_blocked_on_1954() {
    println!("skipped ui: blocked on #1954 (web image); Playwright suite goes in e2e/ui/");
}
