use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchContext {
    pub action: Option<String>,
    pub paths: Vec<String>,
    pub platform: String,
    pub siblings_requires_root: bool,
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
                _ if collecting_paths => paths.push(arg),
                _ => {}
            }
        }

        Self {
            action,
            paths,
            platform: env::consts::OS.to_string(),
            siblings_requires_root: cfg!(not(windows)),
        }
    }
}

#[tauri::command]
pub fn initial_context(context: tauri::State<'_, LaunchContext>) -> LaunchContext {
    context.inner().clone()
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
