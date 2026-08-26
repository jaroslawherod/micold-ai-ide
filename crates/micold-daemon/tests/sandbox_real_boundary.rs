//! §B.2 of the quickstart — the boundary, tested adversarially, **from inside a real session**.
//!
//! This is the feature. `evidence/us1-isolation.md` already probed the same boundary, but it did so
//! with `docker exec` and the entrypoint replaced by a shell: it asked what *a* shell in *that*
//! container can reach. What a user experiences is a shell the daemon spawned, inside the container
//! the application created, reached over the control channel — and the difference is not academic,
//! because the daemon chooses the session's working directory, its environment, and its `HOME`.
//!
//! So every probe here is typed into a session's PTY and read back off the grid, the way the user
//! would type it. The failures this can catch and an `exec` cannot: a session started somewhere
//! other than the project, a `HOME` pointing at a path that happens to be mounted, an environment
//! the daemon passes through that carries a credential.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_boundary -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use micold_core::protocol::auth::Token;

use sandbox_real_support::{
    credentials, input_serial, open_session, seed, start_sandbox, wait_for_accept, SandboxSpec,
    Terminal,
};

const CONTAINER: &str = "micold-boundary-probe";
const NETWORK: &str = "micold-boundary-probe-net";
/// Not 7727: a developer's own sandbox must neither be disturbed nor accidentally probed.
const PORT: u16 = 17731;
/// Written into the project on the host, read back from inside. Distinctive enough that finding it
/// anywhere else would be a finding of its own.
const MARKER: &str = "micold-boundary-marker-8f3a";

// ---------------------------------------------------------------------------------------------
// §B.2
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn sandbox_real_boundary_holds_from_inside_a_session() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("marker.txt"), MARKER).unwrap();

    // A directory beside the project that was never registered, holding the same marker. Probing
    // the project's *parent* instead would prove nothing: a bind mount's parent has to exist inside
    // the container as a path prefix, so it lists — with the mount and nothing else, which is the
    // correct behaviour and reads like a leak.
    let unregistered = dir.path().join("unregistered");
    std::fs::create_dir_all(&unregistered).unwrap();
    std::fs::write(unregistered.join("secret.txt"), MARKER).unwrap();
    let session = seed(&data, &project, "boundary");

    // The daemon's own `HOME`, and so the session's. Passing the *host* home is the realistic
    // setting — it is what `argv::create` does — and it is precisely what makes "`ls ~` shows
    // nothing" worth asserting: the path exists on the host and resolves to nothing inside.
    let host_home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());

    let token = Token::generate();
    let token_path = data.join("micold-ai-ide").join("sandbox.token");
    token.write_to(&token_path).unwrap();

    let daemon_log = data.join("micold-ai-ide").join("micold-daemon.log");
    let _sandbox = start_sandbox(&SandboxSpec {
        container: CONTAINER,
        network: NETWORK,
        port: PORT,
        data_home: &data,
        project: &project,
        token_path: &token_path,
        home: &host_home,
        extra: &[],
    });
    let (mut conn, catalog) = wait_for_accept(PORT, &credentials(&token)).await;
    let serial = input_serial(&catalog, session);
    let screen = open_session(&mut conn, &project, session, &daemon_log).await;
    let mut term = Terminal::new(&mut conn, session, screen, CONTAINER, &daemon_log, serial);

    // The project is reachable at its own host absolute path (research R2) ---------------------
    let seen = term
        .run(&format!("cat {}/marker.txt", project.display()))
        .await;
    assert!(
        seen.contains(MARKER),
        "the registered project must be readable at its host path; got:\n{seen}"
    );

    let cwd = term.run("pwd").await;
    assert!(
        cwd.contains(&project.display().to_string()),
        "a session must start in its project, not wherever the daemon happens to be; got:\n{cwd}"
    );

    // The project's own parent lists the mount and nothing else --------------------------------
    let parent = term.run(&format!("ls {}", dir.path().display())).await;
    assert_eq!(
        parent.trim(),
        "project",
        "the mount's parent must expose the mount and nothing else; got:\n{parent}"
    );

    // The host root is not the container's root -------------------------------------------------
    let root = term.run("ls /").await;
    assert!(
        root.contains("usr") && root.contains("etc"),
        "expected a container root listing; got:\n{root}"
    );

    // Everything below here must be *absent*. Each is a separate probe, so a failure names itself.
    for (what, cmd) in [
        ("the host home directory", format!("ls {host_home}")),
        (
            "an unregistered directory beside the project",
            format!("ls {}", unregistered.display()),
        ),
        (
            "a file in an unregistered directory beside the project",
            format!("cat {}/secret.txt", unregistered.display()),
        ),
        (
            "the runtime's own control socket (C-3)",
            "ls -l /var/run/docker.sock".to_string(),
        ),
        (
            "a git identity, with no credential opt-in (FR-004a)",
            format!("cat {host_home}/.gitconfig"),
        ),
        (
            "the AI CLI's auth directory, with no credential opt-in (FR-004a)",
            format!("ls {host_home}/.claude"),
        ),
    ] {
        let out = term.run(&cmd).await;
        assert!(
            out.contains("No such file") || out.contains("not found") || out.trim().is_empty(),
            "{what} must be unreachable from a sandboxed session (`{cmd}`); got:\n{out}"
        );
        assert!(
            !out.contains(MARKER),
            "{what}: nothing outside the project may carry the project's marker; got:\n{out}"
        );
    }

    // No ssh agent was forwarded, so no key can be listed ---------------------------------------
    //
    // Asserted as "no fingerprint appears" rather than as any particular message: `ssh-add` may be
    // absent from the image, present and unable to reach an agent, or present with an empty agent,
    // and all three are the same answer to the question being asked. What would be a finding is a
    // key.
    let ssh = term.run("ssh-add -l").await;
    assert!(
        !ssh.contains("SHA256:"),
        "with no credential opt-in a session must not be able to list an ssh identity; got:\n{ssh}"
    );

    // What a session writes comes back owned by the user, not by root (R3, C-4) ------------------
    let probe = "written-from-inside.txt";
    term.run(&format!("touch {}/{probe}", project.display()))
        .await;
    let written = project.join(probe);
    assert!(
        written.exists(),
        "a file created inside the sandbox must appear in the host project directory"
    );
    let (uid, gid) = micold_core::sandbox::host_identity();
    let meta = std::fs::metadata(&written).unwrap();
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(
            (meta.uid(), meta.gid()),
            (uid, gid),
            "a file written inside the sandbox came back owned by {}:{} instead of the user who \
             ran the application — that is the failure that would make the sandbox worse than none",
            meta.uid(),
            meta.gid()
        );
    }
    // Editable without elevation, which is the point of the ownership check.
    std::fs::write(&written, b"host can still write this").expect("the host can edit it");
    std::fs::remove_file(&written).expect("and remove it");
}
