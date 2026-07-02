use std::env;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

use linkforge_core::{ConflictPolicy, LinkKind, LinkStepStatus};
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
    pub preflight: DirectDropPreflight,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectDropPreflight {
    pub problems: Vec<DirectDropPreflightItem>,
    pub conflicts: Vec<DirectDropPreflightConflict>,
    pub warnings: Vec<DirectDropPreflightItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectDropPreflightItem {
    pub source: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectDropPreflightConflict {
    pub source: String,
    pub link: String,
    pub message: String,
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

    pub fn is_lightweight_action(&self) -> bool {
        matches!(
            self.action.as_deref(),
            Some(
                linkforge_shared::action::DROP_SYMLINK
                    | linkforge_shared::action::DROP_HARDLINK
                    | linkforge_shared::action::PICK_SOURCE
                    | linkforge_shared::action::SAME_FILE
                    | linkforge_shared::action::LINK_COUNT
            )
        )
    }
}

#[tauri::command]
pub fn initial_context(context: tauri::State<'_, LaunchContext>) -> LaunchContext {
    context.inner().clone()
}

const DROP_WINDOW_WIDTH: f64 = 560.0;
const DROP_WINDOW_HEIGHT: f64 = 300.0;
const DROP_WINDOW_MIN_WIDTH: f64 = 520.0;
const DROP_WINDOW_MIN_HEIGHT: f64 = 260.0;
const FULL_WINDOW_WIDTH: f64 = 1080.0;
const FULL_WINDOW_HEIGHT: f64 = 760.0;
const FULL_WINDOW_MIN_WIDTH: f64 = 900.0;
const FULL_WINDOW_MIN_HEIGHT: f64 = 620.0;

pub fn configure_initial_window(app: &tauri::App) -> Result<(), String> {
    let context = app.state::<LaunchContext>();
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is not available".to_string())?;

    if context.is_lightweight_action() {
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
        .set_resizable(false)
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
        .set_resizable(true)
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

fn hardlink_created_message(source: &Path, link: &Path) -> String {
    if source.is_dir() {
        format!(
            "Created hard-link tree: {} -> {}",
            link.display(),
            source.display()
        )
    } else {
        format!(
            "Created file hard link: {} -> {}",
            link.display(),
            source.display()
        )
    }
}

fn link_kind_from_action(value: &str) -> Option<LinkKind> {
    match value {
        linkforge_shared::action::SYMLINK | linkforge_shared::action::DROP_SYMLINK => {
            Some(LinkKind::Symlink)
        }
        linkforge_shared::action::HARDLINK | linkforge_shared::action::DROP_HARDLINK => {
            Some(LinkKind::Hardlink)
        }
        _ => None,
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

fn pick_sources(paths: &[String]) -> io::Result<()> {
    linkforge_shared::pick_sources(paths)
}

#[tauri::command]
pub fn pick_context_sources(paths: Vec<String>) -> GuiResult<OperationResult> {
    pick_sources(&paths).map_err(gui_error)?;
    Ok(OperationResult {
        message: format!("Picked {} source(s).", paths.len()),
    })
}

fn picked_sources() -> Vec<PathBuf> {
    linkforge_shared::picked_sources()
}

#[tauri::command]
pub fn prepare_direct_drop(
    targets: Vec<String>,
    background_target: bool,
    kind: String,
) -> GuiResult<DirectDropContext> {
    let kind = link_kind_from_action(&kind).ok_or_else(|| {
        gui_error(io::Error::new(
            ErrorKind::InvalidInput,
            format!("unsupported link kind: {kind}"),
        ))
    })?;
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

    let preflight = direct_drop_preflight_from_core(linkforge_core::preflight_link_batch(
        &sources,
        &target_dir,
        kind,
    ));

    Ok(DirectDropContext {
        sources: paths_to_strings(sources),
        target_dir: target_dir.display().to_string(),
        preflight,
    })
}

#[tauri::command]
pub fn create_direct_link_step(
    source: String,
    target_dir: String,
    kind: String,
    conflict_choice: Option<String>,
) -> GuiResult<DirectLinkStepResult> {
    let kind = link_kind_from_action(&kind).ok_or_else(|| {
        gui_error(io::Error::new(
            ErrorKind::InvalidInput,
            format!("unsupported link kind: {kind}"),
        ))
    })?;
    let conflict_policy = match conflict_choice.as_deref() {
        None => ConflictPolicy::Fail,
        Some("overwrite") => ConflictPolicy::Overwrite,
        Some("rename") => ConflictPolicy::Rename,
        Some("skip") => ConflictPolicy::Skip,
        Some("cancel") => {
            return Ok(DirectLinkStepResult {
                status: "cancelled".to_string(),
                source: source.clone(),
                link: None,
                message: "Operation cancelled.".to_string(),
            });
        }
        Some(value) => {
            return Err(gui_error(io::Error::new(
                ErrorKind::InvalidInput,
                format!("unsupported conflict choice: {value}"),
            )));
        }
    };

    Ok(direct_step_result_from_core(
        linkforge_core::create_link_batch_step(
            Path::new(&source),
            Path::new(&target_dir),
            kind,
            conflict_policy,
        ),
    ))
}

#[cfg(test)]
fn create_direct_link_step_inner(
    source: &Path,
    target_dir: &Path,
    kind: LinkKind,
    conflict_policy: ConflictPolicy,
) -> DirectLinkStepResult {
    direct_step_result_from_core(linkforge_core::create_link_batch_step(
        source,
        target_dir,
        kind,
        conflict_policy,
    ))
}

fn direct_drop_preflight_from_core(
    preflight: linkforge_core::BatchPreflight,
) -> DirectDropPreflight {
    DirectDropPreflight {
        problems: preflight
            .problems
            .into_iter()
            .map(|problem| DirectDropPreflightItem {
                source: problem.source.map(|path| path.display().to_string()),
                message: problem.message,
            })
            .collect(),
        conflicts: preflight
            .conflicts
            .into_iter()
            .map(|conflict| DirectDropPreflightConflict {
                source: conflict.source.display().to_string(),
                link: conflict.link.display().to_string(),
                message: conflict.message,
            })
            .collect(),
        warnings: preflight
            .warnings
            .into_iter()
            .map(|warning| DirectDropPreflightItem {
                source: warning.source.map(|path| path.display().to_string()),
                message: warning.message,
            })
            .collect(),
    }
}

fn direct_step_result_from_core(result: linkforge_core::LinkStepResult) -> DirectLinkStepResult {
    DirectLinkStepResult {
        status: link_step_status_name(result.status).to_string(),
        source: result.source.display().to_string(),
        link: result.link.map(|path| path.display().to_string()),
        message: result.message,
    }
}

fn link_step_status_name(status: LinkStepStatus) -> &'static str {
    match status {
        LinkStepStatus::Created => "created",
        LinkStepStatus::Renamed => "renamed",
        LinkStepStatus::Skipped => "skipped",
        LinkStepStatus::NeedsConflict => "needsConflict",
        LinkStepStatus::Failed => "failed",
    }
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
    linkforge_core::create_link(&source, &link, LinkKind::Hardlink, force).map_err(gui_error)?;
    Ok(OperationResult {
        message: hardlink_created_message(&source, &link),
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
    fn classifies_lightweight_context_actions() {
        for action in [
            "drop-symlink",
            "drop-hardlink",
            "pick-source",
            "same-file",
            "link-count",
        ] {
            let context = LaunchContext::from_args([
                "--context-action".to_string(),
                action.to_string(),
                "--paths".to_string(),
                "one.txt".to_string(),
            ]);
            assert!(
                context.is_lightweight_action(),
                "{action} should be lightweight"
            );
        }

        for action in [
            "symlink",
            "hardlink",
            "siblings",
            "scan-groups",
            "clone-tree",
        ] {
            let context = LaunchContext::from_args([
                "--context-action".to_string(),
                action.to_string(),
                "--paths".to_string(),
                "one.txt".to_string(),
            ]);
            assert!(
                !context.is_lightweight_action(),
                "{action} should open the full window"
            );
        }
    }

    #[test]
    fn pick_sources_rejects_empty_paths() {
        let error = pick_sources(&[]).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn pick_sources_rejects_missing_paths() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing.txt");
        let error = pick_sources(&[missing.display().to_string()]).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::NotFound);
    }

    #[test]
    fn pick_sources_writes_json_state() {
        let temp = tempfile::tempdir().unwrap();
        let state_path = temp.path().join("picked-sources.json");
        let paths = vec!["one.txt".to_string(), "two.txt".to_string()];

        linkforge_shared::pick_sources_at(&paths, &state_path).unwrap();

        assert_eq!(
            serde_json::from_str::<Vec<String>>(&fs::read_to_string(state_path).unwrap()).unwrap(),
            paths
        );
    }

    #[test]
    fn picked_sources_reads_json_without_legacy_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let state_path = temp.path().join("picked-sources.json");
        let legacy_path = temp.path().join("picked-source.txt");
        let first = temp.path().join("first.txt");
        let second = temp.path().join("second.txt");
        let legacy = temp.path().join("legacy.txt");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();
        fs::write(&legacy, "legacy").unwrap();
        fs::write(&legacy_path, legacy.display().to_string()).unwrap();

        assert!(linkforge_shared::read_picked_sources_from(&state_path).is_empty());

        fs::write(
            &state_path,
            serde_json::to_string(&[first.display().to_string(), second.display().to_string()])
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            linkforge_shared::read_picked_sources_from(&state_path),
            vec![first.clone(), second]
        );

        fs::write(&state_path, "not json").unwrap();
        assert!(linkforge_shared::read_picked_sources_from(&state_path).is_empty());
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
    fn direct_drop_preflight_reports_missing_sources() {
        let temp = tempfile::tempdir().unwrap();
        let target_dir = temp.path().join("target");
        let missing = temp.path().join("missing.txt");
        fs::create_dir(&target_dir).unwrap();

        let preflight = direct_drop_preflight_from_core(linkforge_core::preflight_link_batch(
            std::slice::from_ref(&missing),
            &target_dir,
            LinkKind::Symlink,
        ));

        assert_eq!(preflight.problems.len(), 1);
        assert_eq!(
            preflight.problems[0].source,
            Some(missing.display().to_string())
        );
        assert!(
            preflight.problems[0]
                .message
                .contains("Source cannot be read")
        );
    }

    #[test]
    fn direct_drop_preflight_reports_invalid_target() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let target_file = temp.path().join("target.txt");
        fs::write(&source, "source").unwrap();
        fs::write(&target_file, "target").unwrap();

        let preflight = direct_drop_preflight_from_core(linkforge_core::preflight_link_batch(
            &[source],
            &target_file,
            LinkKind::Symlink,
        ));

        assert_eq!(preflight.problems.len(), 1);
        assert!(preflight.problems[0].message.contains("is not a directory"));
    }

    #[test]
    fn direct_drop_preflight_reports_existing_target_conflicts() {
        let temp = tempfile::tempdir().unwrap();
        let target_dir = temp.path().join("target");
        let source = temp.path().join("same.txt");
        fs::create_dir(&target_dir).unwrap();
        fs::write(&source, "source").unwrap();
        fs::write(target_dir.join("same.txt"), "existing").unwrap();

        let preflight = direct_drop_preflight_from_core(linkforge_core::preflight_link_batch(
            std::slice::from_ref(&source),
            &target_dir,
            LinkKind::Hardlink,
        ));

        assert!(preflight.problems.is_empty());
        assert_eq!(preflight.conflicts.len(), 1);
        assert_eq!(preflight.conflicts[0].source, source.display().to_string());
        assert_eq!(
            preflight.conflicts[0].link,
            target_dir.join("same.txt").display().to_string()
        );
    }

    #[test]
    fn direct_drop_preflight_reports_source_as_target_conflict() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("same.txt");
        fs::write(&source, "source").unwrap();

        let preflight = direct_drop_preflight_from_core(linkforge_core::preflight_link_batch(
            std::slice::from_ref(&source),
            temp.path(),
            LinkKind::Hardlink,
        ));

        assert!(preflight.problems.is_empty());
        assert_eq!(preflight.conflicts.len(), 1);
        assert_eq!(preflight.conflicts[0].source, source.display().to_string());
        assert_eq!(preflight.conflicts[0].link, source.display().to_string());
        assert!(preflight.conflicts[0].message.contains("picked source"));
    }

    #[test]
    fn direct_drop_preflight_reports_source_without_file_name() {
        let temp = tempfile::tempdir().unwrap();
        let target_dir = temp.path().join("target");
        fs::create_dir(&target_dir).unwrap();

        let preflight = direct_drop_preflight_from_core(linkforge_core::preflight_link_batch(
            &[PathBuf::from(std::path::MAIN_SEPARATOR.to_string())],
            &target_dir,
            LinkKind::Symlink,
        ));

        assert_eq!(preflight.problems.len(), 1);
        assert!(preflight.problems[0].message.contains("no file name"));
    }

    #[test]
    fn direct_drop_preflight_warns_for_hardlink_directory_trees() {
        let temp = tempfile::tempdir().unwrap();
        let target_dir = temp.path().join("target");
        let source_dir = temp.path().join("source-dir");
        fs::create_dir(&target_dir).unwrap();
        fs::create_dir(&source_dir).unwrap();

        let preflight = direct_drop_preflight_from_core(linkforge_core::preflight_link_batch(
            &[source_dir],
            &target_dir,
            LinkKind::Hardlink,
        ));

        assert!(preflight.problems.is_empty());
        assert_eq!(preflight.warnings.len(), 1);
        assert!(preflight.warnings[0].message.contains("hard-link tree"));
    }

    #[test]
    fn direct_drop_preflight_clean_batch_has_no_findings() {
        let temp = tempfile::tempdir().unwrap();
        let target_dir = temp.path().join("target");
        let source = temp.path().join("source.txt");
        fs::create_dir(&target_dir).unwrap();
        fs::write(&source, "source").unwrap();

        let preflight = direct_drop_preflight_from_core(linkforge_core::preflight_link_batch(
            &[source],
            &target_dir,
            LinkKind::Symlink,
        ));

        assert!(preflight.problems.is_empty());
        assert!(preflight.conflicts.is_empty());
        assert!(preflight.warnings.is_empty());
    }

    #[test]
    fn hardlink_same_file_and_link_count_commands_work() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let link = temp.path().join("linked.txt");
        fs::write(&source, "hello").unwrap();

        let result = create_hardlink(
            source.display().to_string(),
            link.display().to_string(),
            false,
        )
        .unwrap();

        assert_eq!(
            result.message,
            format!(
                "Created file hard link: {} -> {}",
                link.display(),
                source.display()
            )
        );
        assert!(
            same_file(source.display().to_string(), link.display().to_string())
                .unwrap()
                .same
        );
        assert!(link_count(source.display().to_string()).unwrap().count >= 2);
    }

    #[test]
    fn hardlink_command_creates_directory_tree_for_directory_sources() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let nested = source.join("nested");
        let link = temp.path().join("linked-tree");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join("nested.txt"), "nested").unwrap();

        let result = create_hardlink(
            source.display().to_string(),
            link.display().to_string(),
            false,
        )
        .unwrap();

        assert_eq!(
            result.message,
            format!(
                "Created hard-link tree: {} -> {}",
                link.display(),
                source.display()
            )
        );
        assert!(
            linkforge_core::is_same_file(
                nested.join("nested.txt"),
                link.join("nested").join("nested.txt"),
            )
            .unwrap()
        );
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
            LinkKind::Hardlink,
            ConflictPolicy::Fail,
        );
        let tree_result = create_direct_link_step_inner(
            &tree_source,
            &target_dir,
            LinkKind::Hardlink,
            ConflictPolicy::Fail,
        );

        assert_eq!(file_result.status, "created");
        assert_eq!(tree_result.status, "created");
        assert!(file_result.message.contains("Created file hard link"));
        assert!(tree_result.message.contains("Created hard-link tree"));
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

        let first_result = create_direct_link_step_inner(
            &first,
            &target_dir,
            LinkKind::Symlink,
            ConflictPolicy::Fail,
        );
        let second_result = create_direct_link_step_inner(
            &second,
            &target_dir,
            LinkKind::Symlink,
            ConflictPolicy::Fail,
        );

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

        let needs_choice = create_direct_link_step_inner(
            &first,
            &target_dir,
            LinkKind::Hardlink,
            ConflictPolicy::Fail,
        );
        assert_eq!(needs_choice.status, "needsConflict");
        assert_eq!(
            needs_choice.link,
            Some(target_dir.join("same.txt").display().to_string())
        );

        let first_rename = create_direct_link_step_inner(
            &first,
            &target_dir,
            LinkKind::Hardlink,
            ConflictPolicy::Rename,
        );
        let second_rename = create_direct_link_step_inner(
            &second,
            &target_dir,
            LinkKind::Hardlink,
            ConflictPolicy::Rename,
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
            LinkKind::Hardlink,
            ConflictPolicy::Skip,
        );
        assert_eq!(skipped.status, "skipped");
    }

    #[test]
    fn direct_link_step_rejects_overwriting_source_in_place() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("same.txt");
        fs::write(&source, "source").unwrap();

        let result = create_direct_link_step_inner(
            &source,
            temp.path(),
            LinkKind::Hardlink,
            ConflictPolicy::Overwrite,
        );

        assert_eq!(result.status, "failed");
        assert!(result.message.contains("different paths"));
        assert_eq!(fs::read_to_string(&source).unwrap(), "source");
    }

    #[test]
    fn direct_link_step_renames_source_in_own_directory() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("same.txt");
        let renamed = temp.path().join("same - Link.txt");
        fs::write(&source, "source").unwrap();

        let result = create_direct_link_step_inner(
            &source,
            temp.path(),
            LinkKind::Hardlink,
            ConflictPolicy::Rename,
        );

        assert_eq!(result.status, "renamed");
        assert_eq!(result.link, Some(renamed.display().to_string()));
        assert_eq!(fs::read_to_string(&source).unwrap(), "source");
        assert!(linkforge_core::is_same_file(&source, &renamed).unwrap());
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
