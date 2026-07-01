use std::env;
use std::fs;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use tauri::{AppHandle, LogicalSize, Manager, WebviewWindow};

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
pub struct DirectDropContext {
    pub sources: Vec<String>,
    pub target_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectLinkStepResult {
    pub status: String,
    pub source: String,
    pub link: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryListing {
    pub current: String,
    pub parent: Option<String>,
    pub entries: Vec<PathEntry>,
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

    pub fn is_drop_action(&self) -> bool {
        matches!(
            self.action.as_deref(),
            Some("drop-symlink" | "drop-hardlink")
        )
    }
}

#[tauri::command]
pub fn initial_context(context: tauri::State<'_, LaunchContext>) -> LaunchContext {
    context.inner().clone()
}

const DROP_WINDOW_WIDTH: f64 = 560.0;
const DROP_WINDOW_HEIGHT: f64 = 360.0;
const DROP_WINDOW_MIN_WIDTH: f64 = 520.0;
const DROP_WINDOW_MIN_HEIGHT: f64 = 320.0;
const FULL_WINDOW_WIDTH: f64 = 1080.0;
const FULL_WINDOW_HEIGHT: f64 = 760.0;
const FULL_WINDOW_MIN_WIDTH: f64 = 900.0;
const FULL_WINDOW_MIN_HEIGHT: f64 = 620.0;

pub fn configure_initial_window(app: &tauri::App) -> Result<(), String> {
    let context = app.state::<LaunchContext>();
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is not available".to_string())?;

    if context.is_drop_action() {
        configure_drop_window(&window)?;
    } else {
        configure_full_window(&window)?;
        show_and_focus_window(&window)?;
    }

    Ok(())
}

#[tauri::command]
pub fn show_drop_window(window: WebviewWindow) -> Result<(), String> {
    configure_drop_window(&window)?;
    show_and_focus_window(&window)
}

#[tauri::command]
pub fn close_drop_window(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn expand_to_full_window(window: WebviewWindow) -> Result<(), String> {
    configure_full_window(&window)?;
    show_and_focus_window(&window)
}

fn configure_drop_window(window: &WebviewWindow) -> Result<(), String> {
    window
        .set_title("LinkForge")
        .map_err(|error| error.to_string())?;
    window
        .set_min_size(Some(LogicalSize::new(
            DROP_WINDOW_MIN_WIDTH,
            DROP_WINDOW_MIN_HEIGHT,
        )))
        .map_err(|error| error.to_string())?;
    window
        .set_size(LogicalSize::new(DROP_WINDOW_WIDTH, DROP_WINDOW_HEIGHT))
        .map_err(|error| error.to_string())?;
    window.center().map_err(|error| error.to_string())
}

fn configure_full_window(window: &WebviewWindow) -> Result<(), String> {
    window
        .set_title("LinkForge")
        .map_err(|error| error.to_string())?;
    window
        .set_min_size(Some(LogicalSize::new(
            FULL_WINDOW_MIN_WIDTH,
            FULL_WINDOW_MIN_HEIGHT,
        )))
        .map_err(|error| error.to_string())?;
    window
        .set_size(LogicalSize::new(FULL_WINDOW_WIDTH, FULL_WINDOW_HEIGHT))
        .map_err(|error| error.to_string())?;
    window.center().map_err(|error| error.to_string())
}

fn show_and_focus_window(window: &WebviewWindow) -> Result<(), String> {
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

pub fn handle_direct_context_action(context: &LaunchContext) -> bool {
    match context.action.as_deref() {
        Some("pick-source") => {
            if let Err(error) = pick_sources(&context.paths) {
                eprintln!("Failed to pick link source:\n{error}");
            }
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

impl DirectLinkKind {
    fn from_str(value: &str) -> Option<Self> {
        match value {
            "symlink" | "drop-symlink" => Some(Self::Symlink),
            "hardlink" | "drop-hardlink" => Some(Self::Hardlink),
            _ => None,
        }
    }

    fn created_message(self, source: &Path, link: &Path) -> String {
        match self {
            Self::Symlink => {
                format!(
                    "Created symbolic link: {} -> {}",
                    link.display(),
                    source.display()
                )
            }
            Self::Hardlink => {
                format!(
                    "Created hard link: {} -> {}",
                    link.display(),
                    source.display()
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConflictChoice {
    Overwrite,
    Rename,
    Skip,
    Cancel,
}

impl ConflictChoice {
    fn from_str(value: &str) -> Option<Self> {
        match value {
            "overwrite" => Some(Self::Overwrite),
            "rename" => Some(Self::Rename),
            "skip" => Some(Self::Skip),
            "cancel" => Some(Self::Cancel),
            _ => None,
        }
    }
}

fn create_one_link(
    source: &Path,
    link: &Path,
    kind: DirectLinkKind,
    force: bool,
) -> io::Result<()> {
    match kind {
        DirectLinkKind::Symlink => linkforge_core::create_symlink(source, link, force),
        DirectLinkKind::Hardlink if source.is_dir() => {
            linkforge_core::create_hard_link_tree(source, link, force)
        }
        DirectLinkKind::Hardlink => linkforge_core::create_hard_link(source, link, force),
    }
}

fn direct_link_target_dir(targets: &[String], allow_current_dir: bool) -> Option<PathBuf> {
    let mut directories = targets
        .iter()
        .map(Path::new)
        .filter(|path| path.is_dir())
        .take(2)
        .map(Path::to_path_buf)
        .collect::<Vec<_>>();

    if directories.len() == 1 {
        return directories.pop();
    }

    if directories.is_empty() && allow_current_dir {
        return env::current_dir().ok().filter(|path| path.is_dir());
    }

    None
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

fn pick_sources(paths: &[String]) -> io::Result<()> {
    if paths.is_empty() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "no source paths were provided",
        ));
    }

    pick_sources_at(
        paths,
        &picked_sources_state_path(),
        &picked_source_state_path(),
    )
}

fn pick_sources_at(paths: &[String], state_path: &Path, legacy_path: &Path) -> io::Result<()> {
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        state_path,
        serde_json::to_string(paths).map_err(io::Error::other)?,
    )?;
    if let Some(parent) = legacy_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(legacy_path, paths[0].as_str())
}

fn picked_sources() -> Vec<PathBuf> {
    read_picked_sources_from(&picked_sources_state_path(), &picked_source_state_path())
}

fn read_picked_sources_from(state_path: &Path, legacy_path: &Path) -> Vec<PathBuf> {
    if let Ok(value) = fs::read_to_string(state_path)
        && let Ok(paths) = serde_json::from_str::<Vec<String>>(&value)
    {
        let paths = existing_paths(paths);
        if !paths.is_empty() {
            return paths;
        }
    }

    fs::read_to_string(legacy_path)
        .ok()
        .map(|value| existing_paths([value.trim().to_string()]))
        .unwrap_or_default()
}

fn existing_paths(paths: impl IntoIterator<Item = String>) -> Vec<PathBuf> {
    paths
        .into_iter()
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .collect()
}

#[cfg(windows)]
fn picked_source_state_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("LinkForge")
}

#[cfg(not(windows))]
fn picked_source_state_dir() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local").join("state"))
        })
        .unwrap_or_else(env::temp_dir)
        .join("LinkForge")
}

fn picked_sources_state_path() -> PathBuf {
    picked_source_state_dir().join("picked-sources.json")
}

fn picked_source_state_path() -> PathBuf {
    picked_source_state_dir().join("picked-source.txt")
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

#[tauri::command]
pub fn prepare_direct_drop(
    targets: Vec<String>,
    background_target: bool,
) -> GuiResult<DirectDropContext> {
    let sources = picked_sources();
    if sources.is_empty() {
        return Err(gui_error(io::Error::new(
            ErrorKind::NotFound,
            "no link source has been picked",
        )));
    };

    let Some(target_dir) = direct_link_target_dir(&targets, background_target) else {
        return Err(gui_error(io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "select a target directory or right-click a directory background. Received target paths: {}. Current directory: {}",
                describe_targets(&targets),
                describe_current_dir()
            ),
        )));
    };

    Ok(DirectDropContext {
        sources: paths_to_strings(sources),
        target_dir: target_dir.display().to_string(),
    })
}

#[tauri::command]
pub fn create_direct_link_step(
    source: String,
    target_dir: String,
    kind: String,
    conflict_choice: Option<String>,
) -> GuiResult<DirectLinkStepResult> {
    let kind = DirectLinkKind::from_str(&kind).ok_or_else(|| {
        gui_error(io::Error::new(
            ErrorKind::InvalidInput,
            format!("unsupported link kind: {kind}"),
        ))
    })?;
    let conflict_choice = conflict_choice
        .as_deref()
        .map(|value| {
            ConflictChoice::from_str(value).ok_or_else(|| {
                gui_error(io::Error::new(
                    ErrorKind::InvalidInput,
                    format!("unsupported conflict choice: {value}"),
                ))
            })
        })
        .transpose()?;

    Ok(create_direct_link_step_inner(
        Path::new(&source),
        Path::new(&target_dir),
        kind,
        conflict_choice,
    ))
}

fn create_direct_link_step_inner(
    source: &Path,
    target_dir: &Path,
    kind: DirectLinkKind,
    conflict_choice: Option<ConflictChoice>,
) -> DirectLinkStepResult {
    let source_text = source.display().to_string();
    let Some(file_name) = source.file_name() else {
        return direct_step_result(
            "skipped",
            &source_text,
            None,
            format!("{}: source has no file name", source.display()),
        );
    };

    let mut link = target_dir.join(file_name);
    let mut force = false;
    let mut renamed = false;

    if fs::symlink_metadata(&link).is_ok() {
        match conflict_choice {
            None => {
                return direct_step_result(
                    "needsConflict",
                    &source_text,
                    Some(&link),
                    "The target already exists.".to_string(),
                );
            }
            Some(ConflictChoice::Overwrite) => force = true,
            Some(ConflictChoice::Rename) => {
                link = available_link_path(target_dir, Path::new(file_name));
                renamed = true;
            }
            Some(ConflictChoice::Skip) => {
                return direct_step_result(
                    "skipped",
                    &source_text,
                    Some(&link),
                    format!("{}: target already exists", link.display()),
                );
            }
            Some(ConflictChoice::Cancel) => {
                return direct_step_result(
                    "cancelled",
                    &source_text,
                    Some(&link),
                    "Operation cancelled.".to_string(),
                );
            }
        }
    }

    match create_one_link(source, &link, kind, force).map_err(link_error) {
        Ok(()) => direct_step_result(
            if renamed { "renamed" } else { "created" },
            &source_text,
            Some(&link),
            kind.created_message(source, &link),
        ),
        Err(error) => direct_step_result(
            "failed",
            &source_text,
            Some(&link),
            format!("{}: {error}", source.display()),
        ),
    }
}

fn direct_step_result(
    status: &str,
    source: &str,
    link: Option<&Path>,
    message: String,
) -> DirectLinkStepResult {
    DirectLinkStepResult {
        status: status.to_string(),
        source: source.to_string(),
        link: link.map(|path| path.display().to_string()),
        message,
    }
}

#[tauri::command]
pub fn list_directory(path: Option<String>) -> GuiResult<DirectoryListing> {
    let directory = directory_for_listing(path.as_deref().map(PathBuf::from))?;
    let mut entries = Vec::new();

    for entry in fs::read_dir(&directory).map_err(gui_error)? {
        let entry = entry.map_err(gui_error)?;
        let file_type = entry.file_type().map_err(gui_error)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        entries.push(PathEntry {
            name,
            path: entry.path().display().to_string(),
            is_dir: file_type.is_dir(),
        });
    }

    entries.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    Ok(DirectoryListing {
        parent: directory.parent().map(|path| path.display().to_string()),
        current: directory.display().to_string(),
        entries,
    })
}

fn directory_for_listing(path: Option<PathBuf>) -> GuiResult<PathBuf> {
    let path = path.unwrap_or_else(default_start_dir);
    if path.is_dir() {
        return Ok(path);
    }
    if path.is_file()
        && let Some(parent) = path.parent()
        && parent.is_dir()
    {
        return Ok(parent.to_path_buf());
    }
    if !path.exists()
        && let Some(parent) = path.parent()
        && parent.is_dir()
    {
        return Ok(parent.to_path_buf());
    }

    Err(gui_error(io::Error::new(
        ErrorKind::NotFound,
        format!("directory not found: {}", path.display()),
    )))
}

fn default_start_dir() -> PathBuf {
    env::current_dir()
        .ok()
        .filter(|path| path.is_dir())
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_dir())
        })
        .or_else(|| {
            env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .filter(|path| path.is_dir())
        })
        .unwrap_or_else(env::temp_dir)
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
    fn parses_same_file_context_launch_arguments() {
        let context = LaunchContext::from_args([
            "--context-action".to_string(),
            "same-file".to_string(),
            "--paths".to_string(),
            "one.txt".to_string(),
            "two.txt".to_string(),
        ]);

        assert_eq!(context.action.as_deref(), Some("same-file"));
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
    fn pick_sources_writes_json_and_legacy_state() {
        let temp = tempfile::tempdir().unwrap();
        let state_path = temp.path().join("picked-sources.json");
        let legacy_path = temp.path().join("picked-source.txt");
        let paths = vec!["one.txt".to_string(), "two.txt".to_string()];

        pick_sources_at(&paths, &state_path, &legacy_path).unwrap();

        assert_eq!(
            serde_json::from_str::<Vec<String>>(&fs::read_to_string(state_path).unwrap()).unwrap(),
            paths
        );
        assert_eq!(fs::read_to_string(legacy_path).unwrap(), "one.txt");
    }

    #[test]
    fn picked_sources_prefers_json_and_falls_back_to_legacy() {
        let temp = tempfile::tempdir().unwrap();
        let state_path = temp.path().join("picked-sources.json");
        let legacy_path = temp.path().join("picked-source.txt");
        let first = temp.path().join("first.txt");
        let second = temp.path().join("second.txt");
        let legacy = temp.path().join("legacy.txt");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();
        fs::write(&legacy, "legacy").unwrap();
        fs::write(
            &state_path,
            serde_json::to_string(&[first.display().to_string(), second.display().to_string()])
                .unwrap(),
        )
        .unwrap();
        fs::write(&legacy_path, legacy.display().to_string()).unwrap();

        assert_eq!(
            read_picked_sources_from(&state_path, &legacy_path),
            vec![first.clone(), second]
        );

        fs::write(&state_path, "not json").unwrap();
        assert_eq!(
            read_picked_sources_from(&state_path, &legacy_path),
            vec![legacy]
        );
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
    fn direct_link_target_dir_rejects_multiple_directories() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();

        let targets = vec![first.display().to_string(), second.display().to_string()];

        assert_eq!(direct_link_target_dir(&targets, false), None);
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
    fn direct_link_step_creates_files_and_directory_trees() {
        let temp = tempfile::tempdir().unwrap();
        let target_dir = temp.path().join("target");
        let file_source = temp.path().join("file.txt");
        let tree_source = temp.path().join("tree");
        let nested = tree_source.join("nested");
        fs::create_dir(&target_dir).unwrap();
        fs::write(&file_source, "file").unwrap();
        fs::create_dir(&tree_source).unwrap();
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join("nested.txt"), "nested").unwrap();

        let file_result = create_direct_link_step_inner(
            &file_source,
            &target_dir,
            DirectLinkKind::Hardlink,
            None,
        );
        let tree_result = create_direct_link_step_inner(
            &tree_source,
            &target_dir,
            DirectLinkKind::Hardlink,
            None,
        );

        assert_eq!(file_result.status, "created");
        assert_eq!(tree_result.status, "created");
        assert!(linkforge_core::is_same_file(&file_source, target_dir.join("file.txt")).unwrap());
        assert!(
            linkforge_core::is_same_file(
                tree_source.join("nested").join("nested.txt"),
                target_dir.join("tree").join("nested").join("nested.txt"),
            )
            .unwrap()
        );
    }

    #[test]
    fn direct_link_step_creates_symlinks_when_supported() {
        let temp = tempfile::tempdir().unwrap();
        let target_dir = temp.path().join("target");
        let first = temp.path().join("first.txt");
        let second = temp.path().join("second.txt");
        fs::create_dir(&target_dir).unwrap();
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();

        let first_result =
            create_direct_link_step_inner(&first, &target_dir, DirectLinkKind::Symlink, None);
        let second_result =
            create_direct_link_step_inner(&second, &target_dir, DirectLinkKind::Symlink, None);

        let results = [&first_result, &second_result];
        if results.iter().any(|result| result.status == "failed") && cfg!(windows) {
            assert!(results.iter().any(|result| {
                result
                    .message
                    .contains("symbolic links on Windows requires")
            }));
            return;
        }

        assert!(results.iter().all(|result| result.status == "created"));
        assert_eq!(
            fs::read_to_string(target_dir.join("first.txt")).unwrap(),
            "first"
        );
        assert_eq!(
            fs::read_to_string(target_dir.join("second.txt")).unwrap(),
            "second"
        );
    }

    #[test]
    fn direct_link_step_reports_and_applies_conflict_choices() {
        let temp = tempfile::tempdir().unwrap();
        let first_dir = temp.path().join("first");
        let second_dir = temp.path().join("second");
        let target_dir = temp.path().join("target");
        fs::create_dir(&first_dir).unwrap();
        fs::create_dir(&second_dir).unwrap();
        fs::create_dir(&target_dir).unwrap();
        let first = first_dir.join("same.txt");
        let second = second_dir.join("same.txt");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();
        fs::write(target_dir.join("same.txt"), "existing").unwrap();

        let needs_choice =
            create_direct_link_step_inner(&first, &target_dir, DirectLinkKind::Hardlink, None);
        assert_eq!(needs_choice.status, "needsConflict");
        assert_eq!(
            needs_choice.link,
            Some(target_dir.join("same.txt").display().to_string())
        );

        let first_rename = create_direct_link_step_inner(
            &first,
            &target_dir,
            DirectLinkKind::Hardlink,
            Some(ConflictChoice::Rename),
        );
        let second_rename = create_direct_link_step_inner(
            &second,
            &target_dir,
            DirectLinkKind::Hardlink,
            Some(ConflictChoice::Rename),
        );
        assert_eq!(first_rename.status, "renamed");
        assert_eq!(second_rename.status, "renamed");
        assert!(target_dir.join("same - Link.txt").exists());
        assert!(target_dir.join("same - Link (2).txt").exists());

        let skip_target = temp.path().join("skip-target");
        fs::create_dir(&skip_target).unwrap();
        fs::write(skip_target.join("same.txt"), "existing").unwrap();
        let skipped = create_direct_link_step_inner(
            &first,
            &skip_target,
            DirectLinkKind::Hardlink,
            Some(ConflictChoice::Skip),
        );
        assert_eq!(skipped.status, "skipped");

        let cancel_target = temp.path().join("cancel-target");
        fs::create_dir(&cancel_target).unwrap();
        fs::write(cancel_target.join("same.txt"), "existing").unwrap();
        let cancelled = create_direct_link_step_inner(
            &first,
            &cancel_target,
            DirectLinkKind::Hardlink,
            Some(ConflictChoice::Cancel),
        );
        assert_eq!(cancelled.status, "cancelled");
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
