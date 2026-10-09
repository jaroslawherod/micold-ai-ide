//! A clipboard image saved as a PNG file (feature 487, FR-006 to FR-008, FR-012).
//!
//! The impure half of [`micold_core::path_insert::PastedLayout`]: read the image off the system
//! clipboard, encode it, write it into the session's worktree (kept out of `git status`) or, when
//! that cannot be written, under the app's data directory. Every failure is an `Err` that names the
//! cause, so the user is told why nothing was inserted.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use micold_core::path_insert::PastedLayout;

/// An image as the clipboard hands it over: straight RGBA, 8 bits per channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    /// Pixels across.
    pub width: usize,
    /// Pixels down.
    pub height: usize,
    /// `width * height * 4` bytes.
    pub bytes: Vec<u8>,
}

/// The image on the system clipboard. `Ok(None)` when it holds no image (the caller then pastes as
/// before); `Err` when it holds one that could not be read.
pub fn read_clipboard() -> Result<Option<Rgba>, String> {
    // A clipboard that cannot be opened cannot be known to hold an image: an empty paste stays
    // the silent no-op it always was.
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return Ok(None);
    };
    match clipboard.get_image() {
        Ok(img) => Ok(Some(Rgba {
            width: img.width,
            height: img.height,
            bytes: img.bytes.into_owned(),
        })),
        Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(e) => Err(format!(
            "Nothing was inserted: the image on the clipboard could not be read ({e})."
        )),
    }
}

/// `image` as PNG bytes.
pub fn encode_png(image: &Rgba) -> Result<Vec<u8>, String> {
    let (w, h) = (
        u32::try_from(image.width).map_err(|e| e.to_string())?,
        u32::try_from(image.height).map_err(|e| e.to_string())?,
    );
    if image.bytes.len() != image.width * image.height * 4 {
        return Err("the image data does not match its size".to_string());
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(&image.bytes)
        .map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok(out)
}

/// Save `image` for a session and return the file's path.
///
/// Tries `worktree` first (when the session has one), then `fallback`. A worktree directory is
/// kept out of version control two ways: a `*` `.gitignore` inside it, and a line in the
/// repository's `info/exclude`. The second may fail (it is only a convenience); the first alone
/// keeps `git status` clean.
pub fn save(
    image: &Rgba,
    worktree: Option<(&Path, &PastedLayout)>,
    fallback: &PastedLayout,
    now_nanos: u128,
    seq: u64,
) -> Result<PathBuf, String> {
    let png = encode_png(image).map_err(|e| format!("Nothing was inserted: {e}."))?;
    if let Some((root, layout)) = worktree {
        if let Ok(path) = write_into_worktree(root, layout, &png, now_nanos, seq) {
            return Ok(path);
        }
    }
    write_file(fallback, &png, now_nanos, seq).map_err(|e| {
        format!(
            "Nothing was inserted: the image could not be saved in {} ({e}).",
            fallback.dir().display()
        )
    })
}

fn write_into_worktree(
    root: &Path,
    layout: &PastedLayout,
    png: &[u8],
    now_nanos: u128,
    seq: u64,
) -> std::io::Result<PathBuf> {
    fs::create_dir_all(layout.dir())?;
    // `*` ignores the `.gitignore` itself too, so the directory is invisible to status.
    let gitignore = layout.dir().join(".gitignore");
    if !gitignore.exists() {
        fs::write(&gitignore, "*\n")?;
    }
    // Best effort: the `.gitignore` above already keeps status clean if this fails.
    let _ = append_exclude(root);
    write_file(layout, png, now_nanos, seq)
}

fn write_file(
    layout: &PastedLayout,
    png: &[u8],
    now_nanos: u128,
    seq: u64,
) -> std::io::Result<PathBuf> {
    fs::create_dir_all(layout.dir())?;
    let path = layout.next_file(now_nanos, seq);
    // `create_new`: a name that exists is never overwritten (US2.5).
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(png)?;
    Ok(path)
}

/// Add [`PastedLayout::exclude_line`] to the repository's `info/exclude`, once. The file is the
/// one `git rev-parse --git-path info/exclude` names from `worktree`, which linked worktrees share.
fn append_exclude(worktree: &Path) -> std::io::Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "--git-path", "info/exclude"])
        .current_dir(worktree);
    let out = micold_core::process::no_window(&mut cmd).output()?;
    if !out.status.success() {
        return Err(std::io::Error::other("not a git repository"));
    }
    let printed = String::from_utf8_lossy(&out.stdout);
    let exclude = worktree.join(printed.trim());
    let line = PastedLayout::exclude_line();
    let existing = fs::read_to_string(&exclude).unwrap_or_default();
    if existing.lines().any(|l| l == line) {
        return Ok(());
    }
    if let Some(parent) = exclude.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut text = existing;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(line);
    text.push('\n');
    fs::write(&exclude, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::session::SessionId;

    fn img() -> Rgba {
        Rgba {
            width: 2,
            height: 1,
            bytes: vec![255, 0, 0, 255, 0, 255, 0, 128],
        }
    }

    fn git(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn repo() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        git(t.path(), &["init", "-q"]);
        git(t.path(), &["config", "core.autocrlf", "false"]);
        t
    }

    #[test]
    fn the_pixels_survive_the_png_round_trip() {
        let bytes = encode_png(&img()).unwrap();
        let mut reader = png::Decoder::new(bytes.as_slice()).read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!((info.width, info.height), (2, 1));
        assert_eq!(&buf[..8], img().bytes.as_slice());
    }

    #[test]
    fn a_size_that_does_not_match_the_data_is_an_error() {
        let mut bad = img();
        bad.bytes.pop();
        assert!(encode_png(&bad).is_err());
    }

    #[test]
    fn a_saved_image_is_a_png_in_the_worktree_and_status_stays_clean() {
        let repo = repo();
        let id = SessionId::new();
        let layout = PastedLayout::in_worktree(repo.path(), id);
        let data = tempfile::tempdir().unwrap();
        let fallback = PastedLayout::in_data_dir(data.path(), id);
        let path = save(&img(), Some((repo.path(), &layout)), &fallback, 7, 1).unwrap();
        assert!(path.starts_with(layout.dir()), "{path:?}");
        assert!(fs::read(&path).unwrap().starts_with(b"\x89PNG"));
        assert_eq!(
            fs::read_to_string(layout.dir().join(".gitignore")).unwrap(),
            "*\n"
        );
        assert_eq!(git(repo.path(), &["status", "--porcelain"]), "");
    }

    #[test]
    fn the_exclude_line_is_appended_once() {
        let repo = repo();
        let id = SessionId::new();
        let layout = PastedLayout::in_worktree(repo.path(), id);
        let data = tempfile::tempdir().unwrap();
        let fallback = PastedLayout::in_data_dir(data.path(), id);
        for seq in 1..=3 {
            save(&img(), Some((repo.path(), &layout)), &fallback, 7, seq).unwrap();
        }
        let exclude = fs::read_to_string(repo.path().join(".git/info/exclude")).unwrap();
        assert_eq!(
            exclude
                .lines()
                .filter(|l| *l == PastedLayout::exclude_line())
                .count(),
            1,
            "{exclude}"
        );
    }

    #[test]
    fn two_saves_never_share_a_file() {
        let data = tempfile::tempdir().unwrap();
        let fallback = PastedLayout::in_data_dir(data.path(), SessionId::new());
        let a = save(&img(), None, &fallback, 7, 1).unwrap();
        let b = save(&img(), None, &fallback, 7, 2).unwrap();
        assert_ne!(a, b);
        assert!(a.exists() && b.exists());
    }

    #[test]
    fn a_worktree_that_cannot_be_written_falls_back_to_the_data_dir() {
        // A file where the worktree should be: nothing can be created inside it, as root or not.
        let blocker = tempfile::NamedTempFile::new().unwrap();
        let id = SessionId::new();
        let layout = PastedLayout::in_worktree(blocker.path(), id);
        let data = tempfile::tempdir().unwrap();
        let fallback = PastedLayout::in_data_dir(data.path(), id);
        let path = save(&img(), Some((blocker.path(), &layout)), &fallback, 7, 1).unwrap();
        assert!(path.starts_with(fallback.dir()), "{path:?}");
    }

    #[test]
    fn a_failed_write_names_where_it_failed() {
        let blocker = tempfile::NamedTempFile::new().unwrap();
        let fallback = PastedLayout::in_data_dir(blocker.path(), SessionId::new());
        let err = save(&img(), None, &fallback, 7, 1).unwrap_err();
        assert!(err.starts_with("Nothing was inserted"), "{err}");
        assert!(err.contains(&fallback.dir().display().to_string()), "{err}");
    }

    #[test]
    fn a_sandboxed_image_in_the_state_directory_is_inserted_by_its_container_path() {
        use micold_core::path_insert::{plan_insertion, InsertTarget, ShellKind};
        use micold_core::sandbox::{
            HomeMount, MountSet, ProjectMount, SecretMount, StateMount, STATE_CONTAINER_DIR,
        };
        // The worktree cannot take the image, so it goes to the state directory (US3.5).
        let blocker = tempfile::NamedTempFile::new().unwrap();
        let state = tempfile::tempdir().unwrap();
        let id = SessionId::new();
        let layout = PastedLayout::in_worktree(blocker.path(), id);
        let fallback = PastedLayout::in_data_dir(state.path(), id);
        let path = save(&img(), Some((blocker.path(), &layout)), &fallback, 7, 1).unwrap();
        let mounts = MountSet {
            projects: vec![ProjectMount::project_for(
                blocker.path().to_path_buf(),
                false,
            )],
            state: StateMount {
                host: state.path().to_path_buf(),
                container: PathBuf::from(STATE_CONTAINER_DIR),
            },
            home: HomeMount {
                host: state.path().join("home"),
                container: PathBuf::from("/home/u"),
            },
            secret: SecretMount {
                host: state.path().join("token"),
                container: PathBuf::from("/run/token"),
            },
            credentials: Vec::new(),
            history: None,
        };
        let plan = plan_insertion(
            &[path],
            ShellKind::Posix,
            InsertTarget::Sandbox(&mounts, &[]),
        );
        let shown = &plan.accepted[0].shown;
        assert!(
            shown.starts_with(format!("{STATE_CONTAINER_DIR}/pasted/{id}")),
            "{shown:?}"
        );
        assert!(shown.to_string_lossy().ends_with(".png"));
    }
}
