use std::collections::HashMap;
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, symlink as symlink_any};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardLinkGroup {
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileId {
    device: u64,
    file: u64,
}

pub fn create_symlink(
    source: impl AsRef<Path>,
    link: impl AsRef<Path>,
    force: bool,
) -> io::Result<()> {
    let source = source.as_ref();
    let link = link.as_ref();
    reject_same_path_entry(source, link)?;
    prepare_target(link, force)?;

    let source_meta = fs::metadata(source)?;
    create_symlink_inner(source, link, source_meta.is_dir())
}

pub fn create_hard_link(
    source: impl AsRef<Path>,
    link: impl AsRef<Path>,
    force: bool,
) -> io::Result<()> {
    let source = source.as_ref();
    let link = link.as_ref();
    reject_same_path_entry(source, link)?;
    prepare_target(link, force)?;
    fs::hard_link(source, link)
}

pub fn is_same_file(path_a: impl AsRef<Path>, path_b: impl AsRef<Path>) -> io::Result<bool> {
    Ok(file_id(path_a.as_ref())? == file_id(path_b.as_ref())?)
}

pub fn hard_link_count(path: impl AsRef<Path>) -> io::Result<u64> {
    hard_link_count_inner(path.as_ref())
}

pub fn hard_link_siblings(
    path: impl AsRef<Path>,
    root: Option<impl AsRef<Path>>,
) -> io::Result<Vec<PathBuf>> {
    let path = path.as_ref();

    #[cfg(windows)]
    {
        if root.is_none() {
            return windows_hard_link_siblings(path);
        }
    }

    let Some(root) = root else {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "a scan root is required on this platform",
        ));
    };

    scan_hard_link_siblings(path, root.as_ref())
}

pub fn scan_hard_link_groups(root: impl AsRef<Path>) -> io::Result<Vec<HardLinkGroup>> {
    let mut paths_by_id: HashMap<FileId, Vec<PathBuf>> = HashMap::new();

    for entry in WalkDir::new(root.as_ref()).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if hard_link_count(path)? > 1 {
            paths_by_id
                .entry(file_id(path)?)
                .or_default()
                .push(path.to_path_buf());
        }
    }

    let mut groups: Vec<HardLinkGroup> = paths_by_id
        .into_values()
        .filter(|paths| paths.len() > 1)
        .map(|mut paths| {
            paths.sort();
            HardLinkGroup { paths }
        })
        .collect();
    groups.sort_by(|left, right| left.paths[0].cmp(&right.paths[0]));
    Ok(groups)
}

pub fn clone_tree_preserve_hardlinks(
    source_dir: impl AsRef<Path>,
    dest_dir: impl AsRef<Path>,
    force: bool,
) -> io::Result<()> {
    let source_dir = source_dir.as_ref();
    let dest_dir = dest_dir.as_ref();

    if !fs::metadata(source_dir)?.is_dir() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "source must be a directory",
        ));
    }
    prepare_directory_target(dest_dir, force)?;
    fs::create_dir(dest_dir)?;

    let mut first_dest_by_id: HashMap<FileId, PathBuf> = HashMap::new();

    for entry in WalkDir::new(source_dir)
        .follow_links(false)
        .min_depth(1)
        .sort_by_file_name()
    {
        let entry = entry?;
        let source_path = entry.path();
        let relative = source_path.strip_prefix(source_dir).map_err(|error| {
            Error::new(
                ErrorKind::InvalidData,
                format!("failed to compute relative path: {error}"),
            )
        })?;
        let dest_path = dest_dir.join(relative);
        let file_type = entry.file_type();

        if file_type.is_dir() {
            fs::create_dir(&dest_path)?;
        } else if file_type.is_symlink() {
            let target = fs::read_link(source_path)?;
            let target_is_dir = symlink_target_is_dir(source_path).unwrap_or(false);
            create_symlink_inner(&target, &dest_path, target_is_dir)?;
        } else if file_type.is_file() {
            let id = file_id(source_path)?;
            if let Some(first_dest) = first_dest_by_id.get(&id) {
                fs::hard_link(first_dest, &dest_path)?;
            } else {
                fs::copy(source_path, &dest_path)?;
                first_dest_by_id.insert(id, dest_path);
            }
        }
    }

    Ok(())
}

pub fn create_hard_link_tree(
    source_dir: impl AsRef<Path>,
    dest_dir: impl AsRef<Path>,
    force: bool,
) -> io::Result<()> {
    let source_dir = source_dir.as_ref();
    let dest_dir = dest_dir.as_ref();

    if !fs::metadata(source_dir)?.is_dir() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "source must be a directory",
        ));
    }
    prepare_directory_target(dest_dir, force)?;
    fs::create_dir(dest_dir)?;

    for entry in WalkDir::new(source_dir)
        .follow_links(false)
        .min_depth(1)
        .sort_by_file_name()
    {
        let entry = entry?;
        let source_path = entry.path();
        let relative = source_path.strip_prefix(source_dir).map_err(|error| {
            Error::new(
                ErrorKind::InvalidData,
                format!("failed to compute relative path: {error}"),
            )
        })?;
        let dest_path = dest_dir.join(relative);
        let file_type = entry.file_type();

        if file_type.is_dir() {
            fs::create_dir(&dest_path)?;
        } else if file_type.is_symlink() {
            let target = fs::read_link(source_path)?;
            let target_is_dir = symlink_target_is_dir(source_path).unwrap_or(false);
            create_symlink_inner(&target, &dest_path, target_is_dir)?;
        } else if file_type.is_file() {
            fs::hard_link(source_path, &dest_path)?;
        }
    }

    Ok(())
}

fn scan_hard_link_siblings(path: &Path, root: &Path) -> io::Result<Vec<PathBuf>> {
    let target_id = file_id(path)?;
    let mut siblings = Vec::new();

    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let candidate = entry.path();
        if file_id(candidate)? == target_id {
            siblings.push(candidate.to_path_buf());
        }
    }

    siblings.sort();
    Ok(siblings)
}

fn reject_same_path_entry(source: &Path, link: &Path) -> io::Result<()> {
    if same_path_entry(source, link)? {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "source and target must be different paths",
        ));
    }

    Ok(())
}

fn same_path_entry(left: &Path, right: &Path) -> io::Result<bool> {
    let (Some(left_parent), Some(right_parent), Some(left_name), Some(right_name)) = (
        left.parent(),
        right.parent(),
        left.file_name(),
        right.file_name(),
    ) else {
        return Ok(false);
    };

    Ok(fs::canonicalize(canonical_parent(left_parent))?
        == fs::canonicalize(canonical_parent(right_parent))?
        && same_file_name(left_name, right_name))
}

fn canonical_parent(parent: &Path) -> &Path {
    if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    }
}

#[cfg(windows)]
fn same_file_name(left: &std::ffi::OsStr, right: &std::ffi::OsStr) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(not(windows))]
fn same_file_name(left: &std::ffi::OsStr, right: &std::ffi::OsStr) -> bool {
    left == right
}

fn prepare_target(path: &Path, force: bool) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_metadata) if !force => Err(Error::new(
            ErrorKind::AlreadyExists,
            format!("target already exists: {}", path.display()),
        )),
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("refusing to replace existing directory: {}", path.display()),
            ))
        }
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn prepare_directory_target(path: &Path, force: bool) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_metadata) if !force => Err(Error::new(
            ErrorKind::AlreadyExists,
            format!("target already exists: {}", path.display()),
        )),
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("refusing to replace existing directory: {}", path.display()),
            ))
        }
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn create_symlink_inner(source: &Path, link: &Path, _source_is_dir: bool) -> io::Result<()> {
    symlink_any(source, link)
}

#[cfg(windows)]
fn create_symlink_inner(source: &Path, link: &Path, source_is_dir: bool) -> io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::{
        CreateSymbolicLinkW, SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE,
        SYMBOLIC_LINK_FLAG_DIRECTORY,
    };

    let source_wide: Vec<u16> = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let link_wide: Vec<u16> = link
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut flags = SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE;
    if source_is_dir {
        flags |= SYMBOLIC_LINK_FLAG_DIRECTORY;
    }

    let created = unsafe { CreateSymbolicLinkW(link_wide.as_ptr(), source_wide.as_ptr(), flags) };
    if created == 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn file_id(path: &Path) -> io::Result<FileId> {
    let metadata = fs::metadata(path)?;
    Ok(FileId {
        device: metadata.dev(),
        file: metadata.ino(),
    })
}

#[cfg(windows)]
fn file_id(path: &Path) -> io::Result<FileId> {
    let info = windows_file_info(path)?;
    Ok(FileId {
        device: u64::from(info.dwVolumeSerialNumber),
        file: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    })
}

#[cfg(unix)]
fn hard_link_count_inner(path: &Path) -> io::Result<u64> {
    Ok(fs::metadata(path)?.nlink())
}

#[cfg(windows)]
fn hard_link_count_inner(path: &Path) -> io::Result<u64> {
    Ok(u64::from(windows_file_info(path)?.nNumberOfLinks))
}

fn symlink_target_is_dir(path: &Path) -> io::Result<bool> {
    Ok(fs::metadata(path)?.is_dir())
}

#[cfg(windows)]
fn windows_file_info(
    path: &Path,
) -> io::Result<windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION> {
    use std::mem::MaybeUninit;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;

    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, GetFileInformationByHandle, OPEN_EXISTING,
    };

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            null_mut(),
        )
    };

    if handle == INVALID_HANDLE_VALUE {
        return Err(Error::last_os_error());
    }

    let mut info = MaybeUninit::uninit();
    let ok = unsafe { GetFileInformationByHandle(handle, info.as_mut_ptr()) };
    let close_result = unsafe { CloseHandle(handle) };

    if ok == 0 {
        return Err(Error::last_os_error());
    }
    if close_result == 0 {
        return Err(Error::last_os_error());
    }

    Ok(unsafe { info.assume_init() })
}

#[cfg(windows)]
fn windows_hard_link_siblings(path: &Path) -> io::Result<Vec<PathBuf>> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Component, Prefix};

    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::{
        FindClose, FindFirstFileNameW, FindNextFileNameW,
    };

    let canonical = fs::canonicalize(path)?;
    let drive = canonical.components().find_map(|component| {
        if let Component::Prefix(prefix) = component {
            match prefix.kind() {
                Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => Some(letter as char),
                _ => None,
            }
        } else {
            None
        }
    });

    let wide_path: Vec<u16> = canonical
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut buffer_len = 32_768u32;
    let mut buffer = vec![0u16; buffer_len as usize];

    let handle =
        unsafe { FindFirstFileNameW(wide_path.as_ptr(), 0, &mut buffer_len, buffer.as_mut_ptr()) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(Error::last_os_error());
    }

    let mut paths = Vec::new();
    loop {
        let mut path_len = buffer_len as usize;
        while path_len > 0 && buffer[path_len - 1] == 0 {
            path_len -= 1;
        }
        let raw = std::ffi::OsString::from_wide(&buffer[..path_len]);
        let link_path = PathBuf::from(raw);
        paths.push(match drive {
            Some(drive) => PathBuf::from(format!("{drive}:{}", link_path.display())),
            None => link_path,
        });

        buffer_len = buffer.len() as u32;
        let found_next = unsafe { FindNextFileNameW(handle, &mut buffer_len, buffer.as_mut_ptr()) };
        if found_next == 0 {
            let error = Error::last_os_error();
            unsafe {
                FindClose(handle);
            }
            if matches!(error.raw_os_error(), Some(18 | 38)) {
                break;
            }
            return Err(error);
        }
    }

    paths.sort();
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_file_symlink_and_hard_link() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let symlink = temp.path().join("source-link.txt");
        let hardlink = temp.path().join("source-hard.txt");
        fs::write(&source, "hello").unwrap();

        if let Err(error) = create_symlink(&source, &symlink, false) {
            if is_windows_symlink_privilege_error(&error) {
                return;
            }
            panic!("failed to create symlink: {error}");
        }
        create_hard_link(&source, &hardlink, false).unwrap();

        assert_eq!(fs::read_to_string(&symlink).unwrap(), "hello");
        assert!(is_same_file(&source, &hardlink).unwrap());
        assert!(hard_link_count(&source).unwrap() >= 2);
    }

    #[test]
    fn creates_directory_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source-dir");
        let symlink = temp.path().join("source-dir-link");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("nested.txt"), "nested").unwrap();

        if let Err(error) = create_symlink(&source, &symlink, false) {
            if is_windows_symlink_privilege_error(&error) {
                return;
            }
            panic!("failed to create symlink: {error}");
        }

        assert_eq!(
            fs::read_to_string(symlink.join("nested.txt")).unwrap(),
            "nested"
        );
    }

    #[test]
    fn force_replaces_files_but_not_directories() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let target = temp.path().join("target.txt");
        let directory = temp.path().join("target-dir");
        fs::write(&source, "source").unwrap();
        fs::write(&target, "old").unwrap();
        fs::create_dir(&directory).unwrap();

        assert!(create_hard_link(&source, &target, false).is_err());
        create_hard_link(&source, &target, true).unwrap();
        assert!(is_same_file(&source, &target).unwrap());
        assert!(create_hard_link(&source, &directory, true).is_err());
    }

    #[test]
    fn force_rejects_hard_link_to_same_path_without_removing_source() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        fs::write(&source, "source").unwrap();

        let error = create_hard_link(&source, &source, true).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InvalidInput);
        assert!(error.to_string().contains("different paths"));
        assert_eq!(fs::read_to_string(&source).unwrap(), "source");
    }

    #[test]
    fn force_rejects_symlink_to_same_path_without_removing_source() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        fs::write(&source, "source").unwrap();

        let error = create_symlink(&source, &source, true).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InvalidInput);
        assert!(error.to_string().contains("different paths"));
        assert_eq!(fs::read_to_string(&source).unwrap(), "source");
    }

    #[test]
    fn same_file_distinguishes_copied_files() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let hardlink = temp.path().join("hard.txt");
        let copy = temp.path().join("copy.txt");
        fs::write(&source, "hello").unwrap();
        fs::hard_link(&source, &hardlink).unwrap();
        fs::copy(&source, &copy).unwrap();

        assert!(is_same_file(&source, &hardlink).unwrap());
        assert!(!is_same_file(&source, &copy).unwrap());
    }

    #[test]
    fn scans_hard_link_groups() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let hardlink = temp.path().join("hard.txt");
        let copy = temp.path().join("copy.txt");
        fs::write(&source, "hello").unwrap();
        fs::hard_link(&source, &hardlink).unwrap();
        fs::copy(&source, &copy).unwrap();

        let groups = scan_hard_link_groups(temp.path()).unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].paths.len(), 2);
        assert!(groups[0].paths.contains(&source));
        assert!(groups[0].paths.contains(&hardlink));
    }

    #[cfg(unix)]
    #[test]
    fn linux_siblings_require_root() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        fs::write(&source, "hello").unwrap();

        assert!(hard_link_siblings(&source, Option::<&Path>::None).is_err());
    }

    #[test]
    fn finds_siblings_by_scanning_root() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let hardlink = temp.path().join("hard.txt");
        fs::write(&source, "hello").unwrap();
        fs::hard_link(&source, &hardlink).unwrap();

        let siblings = hard_link_siblings(&source, Some(temp.path())).unwrap();

        assert_eq!(siblings.len(), 2);
        assert!(siblings.contains(&source));
        assert!(siblings.contains(&hardlink));
    }

    #[cfg(windows)]
    #[test]
    fn windows_finds_siblings_without_scan_root() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let hardlink = temp.path().join("hard.txt");
        fs::write(&source, "hello").unwrap();
        fs::hard_link(&source, &hardlink).unwrap();

        let mut siblings: Vec<PathBuf> = hard_link_siblings(&source, Option::<&Path>::None)
            .unwrap()
            .into_iter()
            .map(|path| fs::canonicalize(path).unwrap())
            .collect();
        siblings.sort();
        let source = fs::canonicalize(source).unwrap();
        let hardlink = fs::canonicalize(hardlink).unwrap();

        assert_eq!(siblings.len(), 2);
        assert!(siblings.contains(&source));
        assert!(siblings.contains(&hardlink));
    }

    #[test]
    fn clones_tree_and_preserves_internal_hard_links() {
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("source");
        let nested = source_dir.join("nested");
        let dest_dir = temp.path().join("dest");
        fs::create_dir(&source_dir).unwrap();
        fs::create_dir(&nested).unwrap();
        let original = source_dir.join("original.txt");
        let hardlink = nested.join("hard.txt");
        fs::write(&original, "hello").unwrap();
        fs::hard_link(&original, &hardlink).unwrap();

        let symlink_created = create_test_file_symlink(
            Path::new("original.txt"),
            &source_dir.join("original-link.txt"),
        );

        clone_tree_preserve_hardlinks(&source_dir, &dest_dir, false).unwrap();

        let cloned_original = dest_dir.join("original.txt");
        let cloned_hardlink = dest_dir.join("nested").join("hard.txt");
        assert_eq!(fs::read_to_string(&cloned_original).unwrap(), "hello");
        assert!(is_same_file(&cloned_original, &cloned_hardlink).unwrap());
        if symlink_created {
            let cloned_symlink = dest_dir.join("original-link.txt");
            assert!(
                fs::symlink_metadata(&cloned_symlink)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
        }
    }

    #[test]
    fn creates_hard_link_tree_for_nested_files() {
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("source");
        let nested = source_dir.join("nested");
        let dest_dir = temp.path().join("dest");
        fs::create_dir(&source_dir).unwrap();
        fs::create_dir(&nested).unwrap();
        let source_file = nested.join("file.txt");
        fs::write(&source_file, "hello").unwrap();

        create_hard_link_tree(&source_dir, &dest_dir, false).unwrap();

        let linked_file = dest_dir.join("nested").join("file.txt");
        assert_eq!(fs::read_to_string(&linked_file).unwrap(), "hello");
        assert!(is_same_file(&source_file, &linked_file).unwrap());
    }

    #[test]
    fn hard_link_tree_copies_symlinks_without_following_them() {
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("source");
        let dest_dir = temp.path().join("dest");
        fs::create_dir(&source_dir).unwrap();
        fs::write(source_dir.join("original.txt"), "hello").unwrap();

        if !create_test_file_symlink(
            Path::new("original.txt"),
            &source_dir.join("original-link.txt"),
        ) {
            return;
        }

        create_hard_link_tree(&source_dir, &dest_dir, false).unwrap();

        let copied_symlink = dest_dir.join("original-link.txt");
        assert!(
            fs::symlink_metadata(copied_symlink)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn hard_link_tree_force_replaces_files_but_not_directories() {
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("source");
        let file_target = temp.path().join("file-target");
        let dir_target = temp.path().join("dir-target");
        fs::create_dir(&source_dir).unwrap();
        fs::write(source_dir.join("file.txt"), "hello").unwrap();
        fs::write(&file_target, "old").unwrap();
        fs::create_dir(&dir_target).unwrap();

        assert!(create_hard_link_tree(&source_dir, &file_target, false).is_err());
        create_hard_link_tree(&source_dir, &file_target, true).unwrap();
        assert!(is_same_file(source_dir.join("file.txt"), file_target.join("file.txt")).unwrap());
        assert!(create_hard_link_tree(&source_dir, &dir_target, true).is_err());
    }

    fn create_test_file_symlink(source: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        {
            symlink_any(source, link).unwrap();
            true
        }

        #[cfg(windows)]
        {
            match create_symlink_inner(source, link, false) {
                Ok(()) => true,
                Err(error) if is_windows_symlink_privilege_error(&error) => false,
                Err(error) => panic!("failed to create test symlink: {error}"),
            }
        }
    }

    fn is_windows_symlink_privilege_error(error: &io::Error) -> bool {
        cfg!(windows) && error.raw_os_error() == Some(1314)
    }
}
