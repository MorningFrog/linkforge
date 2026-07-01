use std::env;
use std::fs;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

#[cfg(windows)]
use windows::Win32::Foundation::HWND;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    IDNO, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_YESNOCANCEL,
    MESSAGEBOX_RESULT, MESSAGEBOX_STYLE, MessageBoxW,
};
#[cfg(windows)]
use windows::core::PCWSTR;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchContext {
    pub action: Option<String>,
    pub paths: Vec<String>,
    pub platform: String,
    pub siblings_requires_root: bool,
    pub background_target: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SameFileResult {
    pub same: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkCountResult {
    pub count: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiblingsResult {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardLinkGroupResult {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanGroupsResult {
    pub groups: Vec<HardLinkGroupResult>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiError {
    pub message: String,
    pub kind: String,
    pub raw_os_error: Option<i32>,
}

pub type GuiResult<T> = Result<T, GuiError>;

impl LaunchContext {
    pub fn from_env() -> Self {
        Self::from_args(env::args().skip(1))
    }

    fn from_args(args: impl IntoIterator<Item = String>) -> Self {
        let mut action = None;
        let mut paths = Vec::new();
        let mut collecting_paths = false;
        let mut background_target = false;
        let mut iter = args.into_iter().peekable();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--context-action" => {
                    action = iter.next();
                    collecting_paths = false;
                }
                "--paths" => {
                    collecting_paths = true;
                }
                "--path" => {
                    if let Some(path) = iter.next() {
                        paths.push(path);
                    }
                    collecting_paths = false;
                }
                "--context-background" => {
                    background_target = true;
                    collecting_paths = false;
                }
                _ if collecting_paths => paths.push(arg),
                _ => {}
            }
        }

        Self {
            action,
            paths,
            platform: env::consts::OS.to_string(),
            siblings_requires_root: cfg!(not(windows)),
            background_target,
        }
    }
}

#[tauri::command]
pub fn initial_context(context: tauri::State<'_, LaunchContext>) -> LaunchContext {
    context.inner().clone()
}

pub fn handle_direct_context_action(context: &LaunchContext) -> bool {
    match context.action.as_deref() {
        Some("pick-source") => {
            let Some(path) = context.paths.first() else {
                show_error("No source path was provided.");
                return true;
            };
            if let Err(error) = pick_source(Path::new(path)) {
                show_error(&format!("Failed to pick link source:\n{error}"));
            }
            true
        }
        Some("drop-symlink") => {
            invoke_drop_link(
                &context.paths,
                DirectLinkKind::Symlink,
                context.background_target,
            );
            true
        }
        Some("drop-hardlink") => {
            invoke_drop_link(
                &context.paths,
                DirectLinkKind::Hardlink,
                context.background_target,
            );
            true
        }
        _ => false,
    }
}

#[derive(Clone, Copy)]
enum DirectLinkKind {
    Symlink,
    Hardlink,
}

enum ConflictChoice {
    Overwrite,
    Rename,
    Cancel,
}

fn invoke_drop_link(targets: &[String], kind: DirectLinkKind, background_target: bool) {
    match drop_link(targets, kind, background_target) {
        Ok(Some(link)) => show_info(&format!("Created link:\n{}", link.display())),
        Ok(None) => {}
        Err(error) => show_error(&format!("Failed to create link:\n{error}")),
    }
}

fn drop_link(
    targets: &[String],
    kind: DirectLinkKind,
    background_target: bool,
) -> io::Result<Option<PathBuf>> {
    let Some(source) = picked_source() else {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            "no link source has been picked",
        ));
    };

    let Some(target_dir) = direct_link_target_dir(targets, background_target) else {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "select a target directory or right-click a directory background. Received target paths: {}. Current directory: {}",
                describe_targets(targets),
                describe_current_dir()
            ),
        ));
    };

    if matches!(kind, DirectLinkKind::Hardlink) && !source.is_file() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "hard links can only be created for files",
        ));
    }

    let file_name = source.file_name().ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!("source has no file name: {}", source.display()),
        )
    })?;
    let mut link = target_dir.join(file_name);
    let mut force = false;

    if fs::symlink_metadata(&link).is_ok() {
        match ask_conflict(&link) {
            ConflictChoice::Overwrite => force = true,
            ConflictChoice::Rename => link = available_link_path(&target_dir, Path::new(file_name)),
            ConflictChoice::Cancel => return Ok(None),
        }
    }

    match kind {
        DirectLinkKind::Symlink => linkforge_core::create_symlink(&source, &link, force),
        DirectLinkKind::Hardlink => linkforge_core::create_hard_link(&source, &link, force),
    }
    .map_err(link_error)?;

    Ok(Some(link))
}

fn direct_link_target_dir(targets: &[String], allow_current_dir: bool) -> Option<PathBuf> {
    let target = targets
        .iter()
        .map(Path::new)
        .find(|path| path.is_dir())
        .map(Path::to_path_buf);
    if target.is_some() {
        return target;
    }

    allow_current_dir
        .then(env::current_dir)
        .and_then(Result::ok)
        .filter(|path| path.is_dir())
}

fn describe_targets(targets: &[String]) -> String {
    if targets.is_empty() {
        return "<none>".to_string();
    }
    targets.join(", ")
}

fn describe_current_dir() -> String {
    env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|error| format!("<unavailable: {error}>"))
}

#[cfg(windows)]
fn link_error(error: io::Error) -> io::Error {
    if error.raw_os_error() == Some(1314) {
        return io::Error::new(
            error.kind(),
            format!(
                "{error}. Creating symbolic links on Windows requires administrator privileges or Developer Mode."
            ),
        );
    }
    error
}

#[cfg(not(windows))]
fn link_error(error: io::Error) -> io::Error {
    error
}

fn pick_source(path: &Path) -> io::Result<()> {
    let state_path = picked_source_state_path();
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(state_path, path.display().to_string())
}

fn picked_source() -> Option<PathBuf> {
    let value = fs::read_to_string(picked_source_state_path()).ok()?;
    let path = PathBuf::from(value.trim());
    path.exists().then_some(path)
}

#[cfg(windows)]
fn picked_source_state_path() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("LinkForge")
        .join("picked-source.txt")
}

#[cfg(not(windows))]
fn picked_source_state_path() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local").join("state"))
        })
        .unwrap_or_else(env::temp_dir)
        .join("LinkForge")
        .join("picked-source.txt")
}

fn available_link_path(target_dir: &Path, source_name: &Path) -> PathBuf {
    let stem = source_name
        .file_stem()
        .filter(|value| !value.is_empty())
        .unwrap_or(source_name.as_os_str())
        .to_string_lossy();
    let extension = source_name.extension().map(|value| value.to_string_lossy());

    for index in 1.. {
        let suffix = if index == 1 {
            " - Link".to_string()
        } else {
            format!(" - Link ({index})")
        };
        let candidate_name = match extension.as_deref() {
            Some(ext) if !ext.is_empty() => format!("{stem}{suffix}.{ext}"),
            _ => format!("{stem}{suffix}"),
        };
        let candidate = target_dir.join(candidate_name);
        if fs::symlink_metadata(&candidate).is_err() {
            return candidate;
        }
    }

    unreachable!("unbounded rename search should always return a candidate")
}

#[cfg(windows)]
fn ask_conflict(path: &Path) -> ConflictChoice {
    let message = format!(
        "The target already exists:\n{}\n\nYes: overwrite it\nNo: create an automatically renamed link\nCancel: do nothing",
        path.display()
    );
    let result = message_box(&message, "LinkForge", MB_YESNOCANCEL | MB_ICONQUESTION);
    if result == IDYES {
        ConflictChoice::Overwrite
    } else if result == IDNO {
        ConflictChoice::Rename
    } else {
        ConflictChoice::Cancel
    }
}

#[cfg(not(windows))]
fn ask_conflict(_path: &Path) -> ConflictChoice {
    ConflictChoice::Rename
}

#[cfg(windows)]
fn show_info(message: &str) {
    message_box(message, "LinkForge", MB_OK | MB_ICONINFORMATION);
}

#[cfg(not(windows))]
fn show_info(message: &str) {
    eprintln!("{message}");
}

#[cfg(windows)]
fn show_error(message: &str) {
    message_box(message, "LinkForge", MB_OK | MB_ICONERROR);
}

#[cfg(not(windows))]
fn show_error(message: &str) {
    eprintln!("{message}");
}

#[cfg(windows)]
fn message_box(message: &str, title: &str, style: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    let message = wide_null(message);
    let title = wide_null(title);
    unsafe {
        MessageBoxW(
            Some(HWND::default()),
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            style,
        )
    }
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[tauri::command]
pub fn create_symlink(source: String, link: String, force: bool) -> GuiResult<OperationResult> {
    let source = PathBuf::from(source);
    let link = PathBuf::from(link);
    linkforge_core::create_symlink(&source, &link, force).map_err(gui_error)?;
    Ok(OperationResult {
        message: format!(
            "Created symbolic link: {} -> {}",
            link.display(),
            source.display()
        ),
    })
}

#[tauri::command]
pub fn create_hardlink(source: String, link: String, force: bool) -> GuiResult<OperationResult> {
    let source = PathBuf::from(source);
    let link = PathBuf::from(link);
    linkforge_core::create_hard_link(&source, &link, force).map_err(gui_error)?;
    Ok(OperationResult {
        message: format!(
            "Created hard link: {} -> {}",
            link.display(),
            source.display()
        ),
    })
}

#[tauri::command]
pub fn same_file(path_a: String, path_b: String) -> GuiResult<SameFileResult> {
    let path_a = PathBuf::from(path_a);
    let path_b = PathBuf::from(path_b);
    let same = linkforge_core::is_same_file(&path_a, &path_b).map_err(gui_error)?;
    let relation = if same { "Same file" } else { "Different files" };
    Ok(SameFileResult {
        same,
        message: format!("{relation}: {} and {}", path_a.display(), path_b.display()),
    })
}

#[tauri::command]
pub fn link_count(path: String) -> GuiResult<LinkCountResult> {
    let path = PathBuf::from(path);
    let count = linkforge_core::hard_link_count(&path).map_err(gui_error)?;
    Ok(LinkCountResult {
        count,
        message: format!("Link count for {}: {count}", path.display()),
    })
}

#[tauri::command]
pub fn siblings(path: String, root: Option<String>) -> GuiResult<SiblingsResult> {
    let path = PathBuf::from(path);
    let root = root
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from);
    let siblings = linkforge_core::hard_link_siblings(&path, root.as_deref()).map_err(gui_error)?;
    Ok(SiblingsResult {
        paths: paths_to_strings(siblings),
    })
}

#[tauri::command]
pub fn scan_groups(root: String) -> GuiResult<ScanGroupsResult> {
    let root = PathBuf::from(root);
    let groups = linkforge_core::scan_hard_link_groups(&root).map_err(gui_error)?;
    Ok(ScanGroupsResult {
        groups: groups
            .into_iter()
            .map(|group| HardLinkGroupResult {
                paths: paths_to_strings(group.paths),
            })
            .collect(),
    })
}

#[tauri::command]
pub fn clone_tree(source_dir: String, dest_dir: String, force: bool) -> GuiResult<OperationResult> {
    let source_dir = PathBuf::from(source_dir);
    let dest_dir = PathBuf::from(dest_dir);
    linkforge_core::clone_tree_preserve_hardlinks(&source_dir, &dest_dir, force)
        .map_err(gui_error)?;
    Ok(OperationResult {
        message: format!(
            "Cloned directory tree: {} -> {}",
            source_dir.display(),
            dest_dir.display()
        ),
    })
}

#[tauri::command]
pub fn reveal_path(path: String) -> GuiResult<OperationResult> {
    reveal_path_inner(Path::new(&path)).map_err(gui_error)?;
    Ok(OperationResult {
        message: format!("Opened {}", path),
    })
}

fn paths_to_strings(paths: Vec<PathBuf>) -> Vec<String> {
    paths
        .into_iter()
        .map(|path| path.display().to_string())
        .collect()
}

fn gui_error(error: io::Error) -> GuiError {
    let mut message = error.to_string();
    if cfg!(windows) && error.raw_os_error() == Some(1314) {
        message.push_str(
            ". Creating symbolic links on Windows requires administrator privileges or Developer Mode.",
        );
    }

    GuiError {
        message,
        kind: format!("{:?}", error.kind()),
        raw_os_error: error.raw_os_error(),
    }
}

fn reveal_path_inner(path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        let status = Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .status()?;
        if status.success() {
            return Ok(());
        }
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open").arg("-R").arg(path).status()?;
        if status.success() {
            return Ok(());
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let target = path.parent().unwrap_or(path);
        let status = Command::new("xdg-open").arg(target).status()?;
        if status.success() {
            return Ok(());
        }
    }

    Err(io::Error::other(
        "failed to open the path in the file manager",
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn parses_context_launch_arguments() {
        let context = LaunchContext::from_args([
            "--context-action".to_string(),
            "link-count".to_string(),
            "--paths".to_string(),
            "one.txt".to_string(),
            "two.txt".to_string(),
        ]);

        assert_eq!(context.action.as_deref(), Some("link-count"));
        assert_eq!(context.paths, ["one.txt", "two.txt"]);
        assert!(!context.background_target);
    }

    #[test]
    fn parses_background_context_flag() {
        let context = LaunchContext::from_args([
            "--context-action".to_string(),
            "drop-symlink".to_string(),
            "--context-background".to_string(),
            "--paths".to_string(),
            "%V".to_string(),
        ]);

        assert_eq!(context.action.as_deref(), Some("drop-symlink"));
        assert!(context.background_target);
        assert_eq!(context.paths, ["%V"]);
    }

    #[test]
    fn direct_link_target_dir_uses_first_existing_directory() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("target");
        let file = temp.path().join("file.txt");
        fs::create_dir(&directory).unwrap();
        fs::write(&file, "hello").unwrap();

        let targets = vec![
            "%V".to_string(),
            file.display().to_string(),
            "%W".to_string(),
            directory.display().to_string(),
        ];

        assert_eq!(direct_link_target_dir(&targets, false), Some(directory));
    }

    #[test]
    fn direct_link_target_dir_can_fall_back_to_current_dir_for_background_targets() {
        let targets = vec!["%V".to_string(), "%1".to_string(), "%W".to_string()];
        let current_dir = env::current_dir().unwrap();

        assert_eq!(direct_link_target_dir(&targets, true), Some(current_dir));
        assert_eq!(direct_link_target_dir(&targets, false), None);
    }

    #[test]
    fn hardlink_same_file_and_link_count_commands_work() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let link = temp.path().join("linked.txt");
        fs::write(&source, "hello").unwrap();

        create_hardlink(
            source.display().to_string(),
            link.display().to_string(),
            false,
        )
        .unwrap();

        assert!(
            same_file(source.display().to_string(), link.display().to_string())
                .unwrap()
                .same
        );
        assert!(link_count(source.display().to_string()).unwrap().count >= 2);
    }

    #[test]
    fn force_replaces_file_but_not_directory() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let target = temp.path().join("target.txt");
        let directory = temp.path().join("target-dir");
        fs::write(&source, "source").unwrap();
        fs::write(&target, "old").unwrap();
        fs::create_dir(&directory).unwrap();

        assert!(
            create_hardlink(
                source.display().to_string(),
                target.display().to_string(),
                false,
            )
            .is_err()
        );
        create_hardlink(
            source.display().to_string(),
            target.display().to_string(),
            true,
        )
        .unwrap();
        assert!(
            create_hardlink(
                source.display().to_string(),
                directory.display().to_string(),
                true,
            )
            .is_err()
        );
    }

    #[test]
    fn siblings_and_scan_groups_return_paths() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let link = temp.path().join("linked.txt");
        fs::write(&source, "hello").unwrap();
        fs::hard_link(&source, &link).unwrap();

        let siblings = siblings(
            source.display().to_string(),
            Some(temp.path().display().to_string()),
        )
        .unwrap();
        assert_eq!(siblings.paths.len(), 2);

        let groups = scan_groups(temp.path().display().to_string()).unwrap();
        assert_eq!(groups.groups.len(), 1);
        assert_eq!(groups.groups[0].paths.len(), 2);
    }

    #[test]
    fn clone_tree_preserves_hardlink_relationships() {
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("source");
        let nested_dir = source_dir.join("nested");
        let dest_dir = temp.path().join("dest");
        fs::create_dir(&source_dir).unwrap();
        fs::create_dir(&nested_dir).unwrap();
        let original = source_dir.join("original.txt");
        let linked = nested_dir.join("linked.txt");
        fs::write(&original, "hello").unwrap();
        fs::hard_link(&original, &linked).unwrap();

        clone_tree(
            source_dir.display().to_string(),
            dest_dir.display().to_string(),
            false,
        )
        .unwrap();

        assert!(
            linkforge_core::is_same_file(
                dest_dir.join("original.txt"),
                dest_dir.join("nested").join("linked.txt"),
            )
            .unwrap()
        );
    }
}
