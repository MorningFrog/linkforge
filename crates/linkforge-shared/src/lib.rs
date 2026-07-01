use std::env;
use std::fs;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub mod action {
    pub const SYMLINK: &str = "symlink";
    pub const HARDLINK: &str = "hardlink";
    pub const SAME_FILE: &str = "same-file";
    pub const LINK_COUNT: &str = "link-count";
    pub const SIBLINGS: &str = "siblings";
    pub const SCAN_GROUPS: &str = "scan-groups";
    pub const CLONE_TREE: &str = "clone-tree";
    pub const PICK_SOURCE: &str = "pick-source";
    pub const DROP_SYMLINK: &str = "drop-symlink";
    pub const DROP_HARDLINK: &str = "drop-hardlink";
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuLinkKind {
    Symlink,
    Hardlink,
}

pub fn pick_sources(paths: &[String]) -> io::Result<()> {
    if paths.is_empty() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "no source paths were provided",
        ));
    }

    for path in paths {
        if !Path::new(path).exists() {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                format!("source path not found: {path}"),
            ));
        }
    }

    pick_sources_at(
        paths,
        &picked_sources_state_path(),
        &picked_source_state_path(),
    )
}

pub fn pick_sources_at(paths: &[String], state_path: &Path, legacy_path: &Path) -> io::Result<()> {
    if paths.is_empty() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "no source paths were provided",
        ));
    }

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

pub fn picked_sources() -> Vec<PathBuf> {
    read_picked_sources_from(&picked_sources_state_path(), &picked_source_state_path())
}

pub fn read_picked_sources_from(state_path: &Path, legacy_path: &Path) -> Vec<PathBuf> {
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

pub fn existing_paths(paths: impl IntoIterator<Item = String>) -> Vec<PathBuf> {
    paths
        .into_iter()
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .collect()
}

#[cfg(windows)]
pub fn picked_source_state_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("LinkForge")
}

#[cfg(not(windows))]
pub fn picked_source_state_dir() -> PathBuf {
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

pub fn picked_sources_state_path() -> PathBuf {
    picked_source_state_dir().join("picked-sources.json")
}

pub fn picked_source_state_path() -> PathBuf {
    picked_source_state_dir().join("picked-source.txt")
}

pub fn pick_source_title(count: u32, first_name: Option<&str>) -> String {
    if count > 1 {
        format!("Pick {count} Link Sources")
    } else {
        first_name
            .map(|name| format!("Pick Link Source: {name}"))
            .unwrap_or_else(|| "Pick Link Source".to_string())
    }
}

pub fn picked_sources_title(kind: MenuLinkKind, sources: &[PathBuf]) -> String {
    match (kind, sources) {
        (_, []) => match kind {
            MenuLinkKind::Symlink => "Create Symlink from Picked Source".to_string(),
            MenuLinkKind::Hardlink => "Create Hard Link from Picked Source".to_string(),
        },
        (MenuLinkKind::Symlink, [source]) => path_display_name(source)
            .map(|name| format!("Create Symlink from {name}"))
            .unwrap_or_else(|| "Create Symlink from Picked Source".to_string()),
        (MenuLinkKind::Hardlink, [source]) if source.is_dir() => path_display_name(source)
            .map(|name| format!("Create Hard-Link Tree from {name}"))
            .unwrap_or_else(|| "Create Hard-Link Tree from Picked Source".to_string()),
        (MenuLinkKind::Hardlink, [source]) => path_display_name(source)
            .map(|name| format!("Create Hard Link from {name}"))
            .unwrap_or_else(|| "Create Hard Link from Picked Source".to_string()),
        (MenuLinkKind::Symlink, sources) => {
            format!("Create Symlinks from {} Sources", sources.len())
        }
        (MenuLinkKind::Hardlink, sources) => {
            format!("Create Hard Links from {} Sources", sources.len())
        }
    }
}

pub fn path_display_name(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn existing_paths_filters_missing_paths() {
        let temp = tempfile::tempdir().unwrap();
        let existing = temp.path().join("existing.txt");
        let missing = temp.path().join("missing.txt");
        fs::write(&existing, "hello").unwrap();

        assert_eq!(
            existing_paths([
                existing.display().to_string(),
                missing.display().to_string()
            ]),
            vec![existing]
        );
    }

    #[test]
    fn state_paths_use_linkforge_dir() {
        assert!(picked_source_state_dir().ends_with("LinkForge"));
        assert!(picked_sources_state_path().ends_with("picked-sources.json"));
        assert!(picked_source_state_path().ends_with("picked-source.txt"));
    }

    #[test]
    fn menu_titles_handle_single_multi_and_directory_sources() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("file.txt");
        let directory = temp.path().join("directory");
        fs::write(&file, "file").unwrap();
        fs::create_dir(&directory).unwrap();

        assert_eq!(
            pick_source_title(1, Some("file.txt")),
            "Pick Link Source: file.txt"
        );
        assert_eq!(
            pick_source_title(2, Some("file.txt")),
            "Pick 2 Link Sources"
        );
        assert_eq!(
            picked_sources_title(MenuLinkKind::Symlink, std::slice::from_ref(&file)),
            "Create Symlink from file.txt"
        );
        assert_eq!(
            picked_sources_title(MenuLinkKind::Hardlink, std::slice::from_ref(&directory)),
            "Create Hard-Link Tree from directory"
        );
        assert_eq!(
            picked_sources_title(MenuLinkKind::Hardlink, &[file, directory]),
            "Create Hard Links from 2 Sources"
        );
    }
}
