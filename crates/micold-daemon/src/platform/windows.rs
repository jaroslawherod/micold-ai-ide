//! Windows process-tree teardown.
//!
//! The robust mechanism is a per-session **job object** created at spawn with
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, so closing the job reaps the child and every descendant.
//! That lands with the Windows CI gate (T083/W5), where it can actually be exercised. Until then
//! this is a no-op: `PtySession::kill`'s `child.kill()` still terminates the direct child, so only
//! grandchild reaping waits for the job-object work — the same scope the Windows gate already tracks.
pub fn terminate_process_tree(_pid: u32) {}
