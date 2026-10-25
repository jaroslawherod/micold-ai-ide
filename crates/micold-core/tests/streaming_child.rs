//! Feature 1289, A5: `StreamingChild` streams a command's stdout and stderr as it prints, gives it
//! no input, and ends its whole process tree when told to (FR-012, FR-023), on every OS
//! (Constitution VI).

use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use micold_core::process::{ExitKind, Stream, StreamingChild};

/// Generous on purpose: what these tests catch is a run that never returns.
const PROMPTLY: Duration = Duration::from_secs(20);

/// Run `cmd` to its end and return what each stream printed, joined, and how it ended.
fn run_to_end(cmd: Command) -> (Vec<u8>, Vec<u8>, ExitKind) {
    let child = StreamingChild::spawn(cmd).expect("spawn");
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let exit = child.run(|stream, bytes| match stream {
        Stream::Stdout => out.extend_from_slice(bytes),
        Stream::Stderr => err.extend_from_slice(bytes),
    });
    (out, err, exit)
}

/// Run `cmd` on another thread, hand back the first stdout chunk, then terminate it from this one.
/// Returns that chunk, how it ended and how long it took after the terminate.
fn terminate_after_first_output(cmd: Command) -> (String, ExitKind, Duration) {
    let child = StreamingChild::spawn(cmd).expect("spawn");
    let terminator = child.terminator();
    let (tx, rx) = mpsc::channel();
    let runner = std::thread::spawn(move || {
        child.run(move |stream, bytes| {
            if stream == Stream::Stdout {
                let _ = tx.send(String::from_utf8_lossy(bytes).into_owned());
            }
        })
    });
    let first = rx.recv_timeout(PROMPTLY).expect("the command printed");
    let asked = Instant::now();
    terminator.terminate();
    let exit = runner.join().expect("run returned");
    (first, exit, asked.elapsed())
}

#[test]
fn a_program_that_does_not_exist_fails_to_spawn() {
    assert!(StreamingChild::spawn(Command::new("micold-no-such-program-1289")).is_err());
}

#[cfg(unix)]
mod unix {
    use super::*;

    fn sh(script: &str) -> Command {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", script]);
        cmd
    }

    /// Whether `pid` is still a live process (a zombie waiting to be reaped counts as gone).
    fn alive(pid: i32) -> bool {
        // SAFETY: signal 0 only checks that the process exists.
        if unsafe { libc::kill(pid, 0) } != 0 {
            return false;
        }
        match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            // The state follows the parenthesised command name.
            Ok(stat) => stat
                .rsplit_once(')')
                .is_none_or(|(_, rest)| !rest.trim_start().starts_with('Z')),
            Err(_) => !cfg!(target_os = "linux"),
        }
    }

    #[test]
    fn each_stream_arrives_in_the_order_it_was_printed() {
        let (out, err, exit) = run_to_end(sh("printf a; printf b >&2; printf c"));
        assert_eq!(out, b"ac");
        assert_eq!(err, b"b");
        assert_eq!(exit, ExitKind::Code(0));
    }

    #[test]
    fn more_than_a_pipe_buffer_of_output_arrives_whole() {
        let (out, _, exit) = run_to_end(sh("head -c 300000 /dev/zero"));
        assert_eq!(out.len(), 300_000);
        assert_eq!(exit, ExitKind::Code(0));
    }

    #[test]
    fn stdin_is_at_end_of_file_from_the_start() {
        let started = Instant::now();
        let (out, _, exit) = run_to_end(sh(r#"read x; echo "got:$?""#));
        assert!(started.elapsed() < PROMPTLY, "read waited for input");
        let out = String::from_utf8(out).unwrap();
        assert!(out.starts_with("got:"), "{out:?}");
        assert_ne!(out.trim(), "got:0", "read must hit end of file");
        assert_eq!(exit, ExitKind::Code(0));
    }

    #[test]
    fn an_exit_code_and_a_signal_are_told_apart() {
        assert_eq!(run_to_end(sh("exit 3")).2, ExitKind::Code(3));
        assert_eq!(run_to_end(sh("kill -TERM $$")).2, ExitKind::Signal(15));
    }

    #[test]
    fn terminate_ends_the_command_and_every_process_it_started() {
        let (first, exit, took) = terminate_after_first_output(sh("sleep 300 & echo $!; wait"));
        assert!(took < PROMPTLY, "run returned {took:?} after terminate");
        assert_eq!(exit, ExitKind::Signal(9));
        let grandchild: i32 = first.trim().parse().expect("the grandchild's pid");
        let deadline = Instant::now() + Duration::from_secs(5);
        while alive(grandchild) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            !alive(grandchild),
            "the grandchild {grandchild} outlived the terminate"
        );
    }

    /// Whether `pid` names any process at all, a zombie included: false once it is reaped.
    fn exists(pid: i32) -> bool {
        // SAFETY: signal 0 only checks that the process exists.
        unsafe { libc::kill(pid, 0) == 0 }
    }

    /// Wait up to five seconds for `gone(pid)`.
    fn soon(pid: i32, gone: impl Fn(i32) -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !gone(pid) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        gone(pid)
    }

    #[test]
    fn a_natural_exit_still_ends_what_it_left_running() {
        let (out, _, exit) = run_to_end(sh("sleep 300 >/dev/null 2>&1 & echo $!"));
        assert_eq!(exit, ExitKind::Code(0));
        let grandchild: i32 = String::from_utf8(out).unwrap().trim().parse().expect("pid");
        assert!(
            soon(grandchild, |pid| !alive(pid)),
            "the grandchild {grandchild} outlived the run"
        );
    }

    #[test]
    fn dropping_it_without_run_ends_and_reaps_its_tree() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pids = dir.path().join("pids");
        let script = format!("sleep 300 & echo $$ $! > '{}'; wait", pids.display());
        let child = StreamingChild::spawn(sh(&script)).expect("spawn");
        let deadline = Instant::now() + PROMPTLY;
        let written = loop {
            let text = std::fs::read_to_string(&pids).unwrap_or_default();
            if text.ends_with('\n') || Instant::now() > deadline {
                break text;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let [leader, grandchild]: [i32; 2] = written
            .split_whitespace()
            .map(|pid| pid.parse().expect("pid"))
            .collect::<Vec<_>>()
            .try_into()
            .expect("two pids");
        drop(child);
        assert!(!exists(leader), "the leader {leader} was not reaped");
        assert!(
            soon(grandchild, |pid| !alive(pid)),
            "the grandchild {grandchild} outlived the drop"
        );
    }

    #[test]
    fn terminate_before_run_still_ends_it() {
        let child = StreamingChild::spawn(sh("sleep 300")).expect("spawn");
        child.terminator().terminate();
        let started = Instant::now();
        assert_eq!(child.run(|_, _| {}), ExitKind::Signal(9));
        assert!(started.elapsed() < PROMPTLY);
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    /// PowerShell's cold start is slow on a busy runner; one at a time keeps the bounds honest.
    static SPAWNING: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn powershell(script: &str) -> Command {
        let mut cmd = Command::new("powershell.exe");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        cmd
    }

    #[test]
    fn each_stream_arrives_in_the_order_it_was_printed() {
        let _turn = SPAWNING.lock().unwrap_or_else(|e| e.into_inner());
        let (out, err, exit) = run_to_end(powershell(
            "[Console]::Out.Write('a'); [Console]::Error.Write('b'); [Console]::Out.Write('c')",
        ));
        assert_eq!(out, b"ac");
        assert_eq!(err, b"b");
        assert_eq!(exit, ExitKind::Code(0));
    }

    #[test]
    fn stdin_is_at_end_of_file_from_the_start() {
        let _turn = SPAWNING.lock().unwrap_or_else(|e| e.into_inner());
        let started = Instant::now();
        let (out, _, _) = run_to_end(powershell(
            "if ($null -eq [Console]::In.ReadLine()) { [Console]::Out.Write('eof') }",
        ));
        assert!(started.elapsed() < PROMPTLY, "the read waited for input");
        assert_eq!(out, b"eof");
    }

    #[test]
    fn terminate_ends_the_command_and_every_process_it_started() {
        let _turn = SPAWNING.lock().unwrap_or_else(|e| e.into_inner());
        let (first, exit, took) = terminate_after_first_output(powershell(
            "$p = Start-Process -PassThru -NoNewWindow powershell.exe \
             -ArgumentList '-NoProfile','-Command','Start-Sleep 300'; \
             [Console]::Out.Write(\"$($p.Id)\"); [Console]::Out.Flush(); Start-Sleep 300",
        ));
        assert!(took < PROMPTLY, "run returned {took:?} after terminate");
        assert!(matches!(exit, ExitKind::Code(_)), "{exit:?}");
        let grandchild: u32 = first.trim().parse().expect("the grandchild's pid");
        let alive = || {
            let probe = Command::new("powershell.exe")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!("if (Get-Process -Id {grandchild} -ErrorAction SilentlyContinue) {{ 'alive' }}"),
                ])
                .output()
                .expect("probe");
            String::from_utf8_lossy(&probe.stdout).contains("alive")
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while alive() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(200));
        }
        assert!(
            !alive(),
            "the grandchild {grandchild} outlived the terminate"
        );
    }
}
