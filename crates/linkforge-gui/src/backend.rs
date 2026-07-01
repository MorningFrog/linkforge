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
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
#[cfg(windows)]
use windows::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TDF_ALLOW_DIALOG_CANCELLATION,
    TDF_POSITION_RELATIVE_TO_WINDOW, TDF_SIZE_TO_CONTENT, TDF_USE_COMMAND_LINKS,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    IDCANCEL, IDNO, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK,
    MB_SETFOREGROUND, MB_TASKMODAL, MB_TOPMOST, MB_YESNO, MB_YESNOCANCEL, MESSAGEBOX_RESULT,
    MESSAGEBOX_STYLE, MessageBoxW,
};
#[cfg(windows)]
use windows::core::{BOOL, HRESULT, PCSTR, PCWSTR};

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
            if let Err(error) = pick_sources(&context.paths) {
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

impl DirectLinkKind {
    fn noun(self) -> &'static str {
        match self {
            Self::Symlink => "symlink",
            Self::Hardlink => "hard link",
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ConflictAnswer {
    choice: ConflictChoice,
    apply_to_remaining: bool,
}

#[derive(Default)]
struct ConflictResolver {
    applied_choice: Option<ConflictChoice>,
}

impl ConflictResolver {
    fn resolve(&mut self, path: &Path, allow_apply_to_remaining: bool) -> ConflictChoice {
        if let Some(choice) = self.applied_choice {
            return choice;
        }

        let answer = ask_conflict(path, allow_apply_to_remaining);
        if answer.apply_to_remaining && !matches!(answer.choice, ConflictChoice::Cancel) {
            self.applied_choice = Some(answer.choice);
        }
        answer.choice
    }

    #[cfg(test)]
    fn with_applied(choice: ConflictChoice) -> Self {
        Self {
            applied_choice: Some(choice),
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
struct BatchLinkSummary {
    created: usize,
    renamed: usize,
    skipped: usize,
    failed: usize,
    cancelled: usize,
    skipped_details: Vec<String>,
    failed_details: Vec<String>,
}

impl BatchLinkSummary {
    fn message(&self, kind: DirectLinkKind) -> String {
        let mut lines = vec![
            format!("Batch {} operation complete.", kind.noun()),
            format!("Created: {}", self.created),
            format!("Renamed: {}", self.renamed),
            format!("Skipped: {}", self.skipped),
            format!("Failed: {}", self.failed),
            format!("Cancelled: {}", self.cancelled),
        ];

        if !self.skipped_details.is_empty() {
            lines.push("Skipped items:".to_string());
            lines.extend(self.skipped_details.iter().cloned());
        }
        if !self.failed_details.is_empty() {
            lines.push("Failed items:".to_string());
            lines.extend(self.failed_details.iter().cloned());
        }

        lines.join("\n")
    }
}

fn invoke_drop_link(targets: &[String], kind: DirectLinkKind, background_target: bool) {
    match drop_links(targets, kind, background_target) {
        Ok(summary) => show_info(&summary.message(kind)),
        Err(error) => show_error(&format!("Failed to create link:\n{error}")),
    }
}

fn drop_links(
    targets: &[String],
    kind: DirectLinkKind,
    background_target: bool,
) -> io::Result<BatchLinkSummary> {
    let sources = picked_sources();
    if sources.is_empty() {
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

    let mut resolver = ConflictResolver::default();
    Ok(create_batch_links(
        &sources,
        &target_dir,
        kind,
        &mut resolver,
    ))
}

fn create_batch_links(
    sources: &[PathBuf],
    target_dir: &Path,
    kind: DirectLinkKind,
    resolver: &mut ConflictResolver,
) -> BatchLinkSummary {
    let mut summary = BatchLinkSummary::default();

    for (index, source) in sources.iter().enumerate() {
        let Some(file_name) = source.file_name() else {
            summary.skipped += 1;
            summary
                .skipped_details
                .push(format!("{}: source has no file name", source.display()));
            continue;
        };

        let mut link = target_dir.join(file_name);
        let mut force = false;
        let mut renamed = false;

        if fs::symlink_metadata(&link).is_ok() {
            match resolver.resolve(&link, sources.len() > 1) {
                ConflictChoice::Overwrite => force = true,
                ConflictChoice::Rename => {
                    link = available_link_path(target_dir, Path::new(file_name));
                    renamed = true;
                }
                ConflictChoice::Skip => {
                    summary.skipped += 1;
                    summary
                        .skipped_details
                        .push(format!("{}: target already exists", link.display()));
                    continue;
                }
                ConflictChoice::Cancel => {
                    summary.cancelled += sources.len() - index;
                    break;
                }
            }
        }

        match create_one_link(source, &link, kind, force).map_err(link_error) {
            Ok(()) => {
                summary.created += 1;
                if renamed {
                    summary.renamed += 1;
                }
            }
            Err(error) => {
                summary.failed += 1;
                summary
                    .failed_details
                    .push(format!("{}: {error}", source.display()));
            }
        }
    }

    summary
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

#[cfg(windows)]
fn ask_conflict(path: &Path, allow_apply_to_remaining: bool) -> ConflictAnswer {
    const RENAME_BUTTON: i32 = 1001;
    const OVERWRITE_BUTTON: i32 = 1002;
    const SKIP_BUTTON: i32 = 1003;
    const CANCEL_BUTTON: i32 = 1004;

    let title = wide_null("LinkForge");
    let instruction = wide_null("The target already exists.");
    let content = wide_null(&path.display().to_string());
    let rename = wide_null("Rename\nCreate an automatically renamed link.");
    let overwrite = wide_null("Overwrite\nReplace the existing file or symbolic link.");
    let skip = wide_null("Skip\nDo not create this link.");
    let cancel = wide_null("Cancel\nStop the remaining batch operation.");
    let verification = wide_null("Apply this choice to remaining conflicts");

    let buttons = [
        TASKDIALOG_BUTTON {
            nButtonID: RENAME_BUTTON,
            pszButtonText: PCWSTR(rename.as_ptr()),
        },
        TASKDIALOG_BUTTON {
            nButtonID: OVERWRITE_BUTTON,
            pszButtonText: PCWSTR(overwrite.as_ptr()),
        },
        TASKDIALOG_BUTTON {
            nButtonID: SKIP_BUTTON,
            pszButtonText: PCWSTR(skip.as_ptr()),
        },
        TASKDIALOG_BUTTON {
            nButtonID: CANCEL_BUTTON,
            pszButtonText: PCWSTR(cancel.as_ptr()),
        },
    ];

    let mut config = TASKDIALOGCONFIG::default();
    config.cbSize = std::mem::size_of::<TASKDIALOGCONFIG>() as u32;
    config.hwndParent = HWND::default();
    config.dwFlags = TDF_ALLOW_DIALOG_CANCELLATION
        | TDF_USE_COMMAND_LINKS
        | TDF_SIZE_TO_CONTENT
        | TDF_POSITION_RELATIVE_TO_WINDOW;
    config.pszWindowTitle = PCWSTR(title.as_ptr());
    config.pszMainInstruction = PCWSTR(instruction.as_ptr());
    config.pszContent = PCWSTR(content.as_ptr());
    config.cButtons = buttons.len() as u32;
    config.pButtons = buttons.as_ptr();
    config.nDefaultButton = RENAME_BUTTON;
    if allow_apply_to_remaining {
        config.pszVerificationText = PCWSTR(verification.as_ptr());
    }

    let mut button = 0;
    let mut verification_checked = BOOL(0);
    if !run_task_dialog_indirect(&config, &mut button, &mut verification_checked) {
        return ask_conflict_with_message_box(path, allow_apply_to_remaining);
    }

    let choice = match button {
        OVERWRITE_BUTTON => ConflictChoice::Overwrite,
        SKIP_BUTTON => ConflictChoice::Skip,
        CANCEL_BUTTON => ConflictChoice::Cancel,
        _ => ConflictChoice::Rename,
    };

    ConflictAnswer {
        choice,
        apply_to_remaining: verification_checked.as_bool(),
    }
}

#[cfg(windows)]
fn ask_conflict_with_message_box(path: &Path, allow_apply_to_remaining: bool) -> ConflictAnswer {
    let first_prompt = format!(
        "The target already exists:\n{}\n\nChoose Yes to create an automatically renamed link.\nChoose No for overwrite or skip options.\nChoose Cancel to stop this operation.",
        path.display()
    );
    let first = message_box(
        &first_prompt,
        "LinkForge",
        MB_YESNOCANCEL | MB_ICONINFORMATION,
    );

    let choice = if first == IDNO {
        let second = ask_overwrite_or_skip_with_message_box(path);
        conflict_choice_from_message_box_results(first, Some(second))
    } else {
        conflict_choice_from_message_box_results(first, None)
    };

    let apply_to_remaining = allow_apply_to_remaining
        && !matches!(choice, ConflictChoice::Cancel)
        && ask_apply_to_remaining_with_message_box();

    ConflictAnswer {
        choice,
        apply_to_remaining,
    }
}

#[cfg(windows)]
fn ask_overwrite_or_skip_with_message_box(path: &Path) -> MESSAGEBOX_RESULT {
    let prompt = format!(
        "The target already exists:\n{}\n\nChoose Yes to overwrite the existing file or symbolic link.\nChoose No to skip this source.\nChoose Cancel to stop this operation.",
        path.display()
    );
    message_box(&prompt, "LinkForge", MB_YESNOCANCEL | MB_ICONINFORMATION)
}

#[cfg(windows)]
fn conflict_choice_from_message_box_results(
    first: MESSAGEBOX_RESULT,
    second: Option<MESSAGEBOX_RESULT>,
) -> ConflictChoice {
    if first == IDYES {
        ConflictChoice::Rename
    } else if first == IDNO && second == Some(IDYES) {
        ConflictChoice::Overwrite
    } else if first == IDNO && second == Some(IDNO) {
        ConflictChoice::Skip
    } else {
        ConflictChoice::Cancel
    }
}

#[cfg(windows)]
fn ask_apply_to_remaining_with_message_box() -> bool {
    let result = message_box(
        "Apply this conflict choice to remaining conflicts in this batch?",
        "LinkForge",
        MB_YESNO | MB_ICONQUESTION,
    );
    result == IDYES
}

#[cfg(windows)]
type TaskDialogIndirectProc =
    unsafe extern "system" fn(*const TASKDIALOGCONFIG, *mut i32, *mut i32, *mut BOOL) -> HRESULT;

#[cfg(windows)]
fn run_task_dialog_indirect(
    config: &TASKDIALOGCONFIG,
    button: &mut i32,
    verification_checked: &mut BOOL,
) -> bool {
    let dll_name = wide_null("comctl32.dll");
    let Ok(module) = (unsafe { LoadLibraryW(PCWSTR(dll_name.as_ptr())) }) else {
        return false;
    };
    let Some(proc) = (unsafe { GetProcAddress(module, PCSTR(b"TaskDialogIndirect\0".as_ptr())) })
    else {
        return false;
    };
    let task_dialog: TaskDialogIndirectProc = unsafe { std::mem::transmute(proc) };
    let mut radio_button = 0;
    let result = unsafe { task_dialog(config, button, &mut radio_button, verification_checked) };
    result.is_ok()
}

#[cfg(not(windows))]
fn ask_conflict(_path: &Path, allow_apply_to_remaining: bool) -> ConflictAnswer {
    ConflictAnswer {
        choice: ConflictChoice::Rename,
        apply_to_remaining: allow_apply_to_remaining,
    }
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
    let style = MESSAGEBOX_STYLE(style.0 | MB_SETFOREGROUND.0 | MB_TOPMOST.0 | MB_TASKMODAL.0);
    unsafe {
        MessageBoxW(
            None,
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
    fn batch_hardlink_creates_files_and_directory_trees() {
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

        let mut resolver = ConflictResolver::default();
        let summary = create_batch_links(
            &[file_source.clone(), tree_source.clone()],
            &target_dir,
            DirectLinkKind::Hardlink,
            &mut resolver,
        );

        assert_eq!(summary.created, 2);
        assert_eq!(summary.failed, 0);
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
    fn batch_symlink_creates_multiple_links_when_supported() {
        let temp = tempfile::tempdir().unwrap();
        let target_dir = temp.path().join("target");
        let first = temp.path().join("first.txt");
        let second = temp.path().join("second.txt");
        fs::create_dir(&target_dir).unwrap();
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();

        let mut resolver = ConflictResolver::default();
        let summary = create_batch_links(
            &[first.clone(), second.clone()],
            &target_dir,
            DirectLinkKind::Symlink,
            &mut resolver,
        );

        if summary.failed > 0 && cfg!(windows) {
            assert!(
                summary
                    .failed_details
                    .iter()
                    .any(|detail| detail.contains("symbolic links on Windows requires"))
            );
            return;
        }

        assert_eq!(summary.created, 2);
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
    fn batch_conflicts_can_rename_skip_or_cancel() {
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

        let mut rename = ConflictResolver::with_applied(ConflictChoice::Rename);
        let summary = create_batch_links(
            &[first.clone(), second.clone()],
            &target_dir,
            DirectLinkKind::Hardlink,
            &mut rename,
        );
        assert_eq!(summary.created, 2);
        assert_eq!(summary.renamed, 2);
        assert!(target_dir.join("same - Link.txt").exists());
        assert!(target_dir.join("same - Link (2).txt").exists());

        let skip_target = temp.path().join("skip-target");
        fs::create_dir(&skip_target).unwrap();
        fs::write(skip_target.join("same.txt"), "existing").unwrap();
        let mut skip = ConflictResolver::with_applied(ConflictChoice::Skip);
        let summary = create_batch_links(
            &[first.clone()],
            &skip_target,
            DirectLinkKind::Hardlink,
            &mut skip,
        );
        assert_eq!(summary.created, 0);
        assert_eq!(summary.skipped, 1);

        let cancel_target = temp.path().join("cancel-target");
        fs::create_dir(&cancel_target).unwrap();
        fs::write(cancel_target.join("same.txt"), "existing").unwrap();
        let mut cancel = ConflictResolver::with_applied(ConflictChoice::Cancel);
        let summary = create_batch_links(
            &[first, second],
            &cancel_target,
            DirectLinkKind::Hardlink,
            &mut cancel,
        );
        assert_eq!(summary.created, 0);
        assert_eq!(summary.cancelled, 2);
    }

    #[cfg(windows)]
    #[test]
    fn conflict_message_box_fallback_maps_button_results() {
        assert_eq!(
            conflict_choice_from_message_box_results(IDYES, None),
            ConflictChoice::Rename
        );
        assert_eq!(
            conflict_choice_from_message_box_results(IDNO, Some(IDYES)),
            ConflictChoice::Overwrite
        );
        assert_eq!(
            conflict_choice_from_message_box_results(IDNO, Some(IDNO)),
            ConflictChoice::Skip
        );
        assert_eq!(
            conflict_choice_from_message_box_results(IDCANCEL, None),
            ConflictChoice::Cancel
        );
        assert_eq!(
            conflict_choice_from_message_box_results(IDNO, Some(IDCANCEL)),
            ConflictChoice::Cancel
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
