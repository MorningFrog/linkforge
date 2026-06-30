#![cfg(windows)]

use std::cell::Cell;
use std::env;
use std::ffi::c_void;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::Win32::Foundation::{E_NOINTERFACE, E_NOTIMPL, HMODULE, HWND, S_FALSE, S_OK};
use windows::Win32::System::Com::{
    CoTaskMemAlloc, CoTaskMemFree, IBindCtx, IClassFactory, IClassFactory_Impl,
};
use windows::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};
use windows::Win32::UI::Shell::{
    ECF_DEFAULT, ECF_HASSUBCOMMANDS, ECS_ENABLED, ECS_HIDDEN, IEnumExplorerCommand,
    IEnumExplorerCommand_Impl, IExplorerCommand, IExplorerCommand_Impl, IShellItemArray,
    SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    IDNO, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_YESNOCANCEL,
    MessageBoxW,
};
use windows::core::{BOOL, GUID, IUnknown, Interface, PCWSTR, PWSTR, Ref, Result, implement};

const CLSID_LINKFORGE_EXPLORER_COMMAND: GUID =
    GUID::from_u128(0x7d4d6e4b_2c72_4a54_9367_6d2f4a3d1c8e);

static SERVER_LOCKS: AtomicU32 = AtomicU32::new(0);
static OBJECT_COUNT: AtomicU32 = AtomicU32::new(0);

#[unsafe(no_mangle)]
/// # Safety
///
/// Called by COM. `rclsid`, `riid`, and `ppv` must be valid pointers following
/// the `DllGetClassObject` contract; `ppv` receives a COM interface pointer on
/// success and is set to null on failure.
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> windows::core::HRESULT {
    if ppv.is_null() {
        return E_NOINTERFACE;
    }

    unsafe {
        *ppv = null_mut();
    }

    if rclsid.is_null() || riid.is_null() {
        return E_NOINTERFACE;
    }

    let requested_clsid = unsafe { *rclsid };
    if requested_clsid != CLSID_LINKFORGE_EXPLORER_COMMAND {
        return E_NOINTERFACE;
    }

    let factory: IClassFactory = ExplorerCommandFactory::new().into();
    unsafe { factory.query(riid, ppv) }
}

#[unsafe(no_mangle)]
pub extern "system" fn DllCanUnloadNow() -> windows::core::HRESULT {
    if SERVER_LOCKS.load(Ordering::SeqCst) == 0 && OBJECT_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[implement(IClassFactory)]
struct ExplorerCommandFactory;

impl ExplorerCommandFactory {
    fn new() -> Self {
        OBJECT_COUNT.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for ExplorerCommandFactory {
    fn drop(&mut self) {
        OBJECT_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

impl IClassFactory_Impl for ExplorerCommandFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<'_, IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> Result<()> {
        if outer.is_some() {
            return Err(E_NOINTERFACE.into());
        }

        let command: IExplorerCommand = LinkForgeCommand::root().into();
        unsafe { command.query(riid, ppvobject).ok() }
    }

    fn LockServer(&self, flock: BOOL) -> Result<()> {
        if flock.as_bool() {
            SERVER_LOCKS.fetch_add(1, Ordering::SeqCst);
        } else {
            SERVER_LOCKS.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum CommandKind {
    Root,
    PickSource,
    DropSymlink,
    DropHardlink,
    Symlink,
    Hardlink,
    LinkCount,
    Siblings,
    ScanGroups,
    CloneTree,
}

impl CommandKind {
    fn title(self, selection: Option<&SelectionInfo>) -> String {
        match self {
            Self::Root => "LinkForge".to_string(),
            Self::PickSource => selection
                .and_then(SelectionInfo::first_name)
                .map(|name| format!("Pick Link Source: {name}"))
                .unwrap_or_else(|| "Pick Link Source".to_string()),
            Self::DropSymlink => picked_source_name()
                .map(|name| format!("Create Symlink from {name}"))
                .unwrap_or_else(|| "Create Symlink from Picked Source".to_string()),
            Self::DropHardlink => picked_source_name()
                .map(|name| format!("Create Hard Link from {name}"))
                .unwrap_or_else(|| "Create Hard Link from Picked Source".to_string()),
            Self::Symlink => "Open Symlink in LinkForge...".to_string(),
            Self::Hardlink => "Open Hard Link in LinkForge...".to_string(),
            Self::LinkCount => "Show Link Count".to_string(),
            Self::Siblings => "Find Hard Link Siblings...".to_string(),
            Self::ScanGroups => "Scan Hard Link Groups".to_string(),
            Self::CloneTree => "Clone Tree Preserving Hard Links...".to_string(),
        }
    }

    fn action(self) -> Option<&'static str> {
        match self {
            Self::Root | Self::PickSource | Self::DropSymlink | Self::DropHardlink => None,
            Self::Symlink => Some("symlink"),
            Self::Hardlink => Some("hardlink"),
            Self::LinkCount => Some("link-count"),
            Self::Siblings => Some("siblings"),
            Self::ScanGroups => Some("scan-groups"),
            Self::CloneTree => Some("clone-tree"),
        }
    }

    fn canonical_name(self) -> GUID {
        match self {
            Self::Root => CLSID_LINKFORGE_EXPLORER_COMMAND,
            Self::PickSource => GUID::from_u128(0xb459d672_17d9_456a_bf56_568032f9fc35),
            Self::DropSymlink => GUID::from_u128(0x6029e9ca_6457_4767_9200_6ae0f2416041),
            Self::DropHardlink => GUID::from_u128(0x797f8a7a_a43c_4103_85ba_c59042475f6c),
            Self::Symlink => GUID::from_u128(0x4ce8a4ab_6d96_41af_84d8_126d4486e1d0),
            Self::Hardlink => GUID::from_u128(0x0af744da_6a93_4c1a_98bf_ef4c969e7a5c),
            Self::LinkCount => GUID::from_u128(0x7918bb2c_9693_4026_a435_e1860ee35587),
            Self::Siblings => GUID::from_u128(0xaf16a719_b2e6_4263_a3af_dee068e736b2),
            Self::ScanGroups => GUID::from_u128(0x55b7a2f4_6316_40a3_8ef9_7caefbf70ab3),
            Self::CloneTree => GUID::from_u128(0xa8325431_f2d6_4c46_8f6a_7fb2cbe63a6d),
        }
    }

    fn supports(self, selection: &SelectionInfo) -> bool {
        match self {
            Self::Root => true,
            Self::PickSource => selection.count == 1,
            Self::DropSymlink => picked_source().is_some(),
            Self::DropHardlink => picked_source().filter(|source| source.is_file()).is_some(),
            Self::Symlink | Self::Siblings => selection.count == 1,
            Self::Hardlink | Self::LinkCount => selection.count == 1 && !selection.first_is_dir,
            Self::ScanGroups | Self::CloneTree => selection.count == 1 && selection.first_is_dir,
        }
    }
}

#[implement(IExplorerCommand)]
struct LinkForgeCommand {
    kind: CommandKind,
}

impl LinkForgeCommand {
    fn root() -> Self {
        OBJECT_COUNT.fetch_add(1, Ordering::SeqCst);
        Self {
            kind: CommandKind::Root,
        }
    }

    fn child(kind: CommandKind) -> Self {
        OBJECT_COUNT.fetch_add(1, Ordering::SeqCst);
        Self { kind }
    }
}

impl Drop for LinkForgeCommand {
    fn drop(&mut self) {
        OBJECT_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

impl IExplorerCommand_Impl for LinkForgeCommand_Impl {
    fn GetTitle(&self, items: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        let selection = items.as_ref().map(selection_info);
        alloc_wide(&self.kind.title(selection.as_ref()))
    }

    fn GetIcon(&self, _items: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        let icon = current_module_dir()
            .map(|dir| dir.join("linkforge-gui.exe"))
            .unwrap_or_else(|| PathBuf::from("linkforge-gui.exe"));
        alloc_wide(&format!("{},0", icon.display()))
    }

    fn GetToolTip(&self, _items: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        alloc_wide("Open LinkForge")
    }

    fn GetCanonicalName(&self) -> Result<GUID> {
        Ok(self.kind.canonical_name())
    }

    fn GetState(&self, items: Ref<'_, IShellItemArray>, _slow: BOOL) -> Result<u32> {
        if matches!(self.kind, CommandKind::Root) {
            return Ok(ECS_ENABLED.0 as u32);
        }

        let selection = items.as_ref().map(selection_info).unwrap_or_default();
        if self.kind.supports(&selection) {
            Ok(ECS_ENABLED.0 as u32)
        } else {
            Ok(ECS_HIDDEN.0 as u32)
        }
    }

    fn Invoke(&self, items: Ref<'_, IShellItemArray>, _bind_ctx: Ref<'_, IBindCtx>) -> Result<()> {
        let selection = items.as_ref().map(selection_info).unwrap_or_default();

        match self.kind {
            CommandKind::PickSource => {
                if let Some(path) = selection.paths.first()
                    && let Err(error) = spawn_gui_action("pick-source", std::slice::from_ref(path))
                {
                    show_error(&format!("Failed to pick link source:\n{error}"));
                }
                return Ok(());
            }
            CommandKind::DropSymlink => {
                invoke_drop_link(&selection, LinkKind::Symlink);
                return Ok(());
            }
            CommandKind::DropHardlink => {
                invoke_drop_link(&selection, LinkKind::Hardlink);
                return Ok(());
            }
            _ => {}
        }

        let Some(action) = self.kind.action() else {
            return Ok(());
        };

        if selection.paths.is_empty() {
            return Ok(());
        }

        let _child = spawn_gui_action(action, &selection.paths);
        Ok(())
    }

    fn GetFlags(&self) -> Result<u32> {
        if matches!(self.kind, CommandKind::Root) {
            Ok((ECF_HASSUBCOMMANDS | ECF_DEFAULT).0 as u32)
        } else {
            Ok(ECF_DEFAULT.0 as u32)
        }
    }

    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        if !matches!(self.kind, CommandKind::Root) {
            return Err(E_NOTIMPL.into());
        }

        Ok(CommandEnumerator::new(vec![
            CommandKind::PickSource,
            CommandKind::DropSymlink,
            CommandKind::DropHardlink,
            CommandKind::Symlink,
            CommandKind::Hardlink,
            CommandKind::LinkCount,
            CommandKind::Siblings,
            CommandKind::ScanGroups,
            CommandKind::CloneTree,
        ])
        .into())
    }
}

#[implement(IEnumExplorerCommand)]
struct CommandEnumerator {
    commands: Vec<CommandKind>,
    index: Cell<usize>,
}

impl CommandEnumerator {
    fn new(commands: Vec<CommandKind>) -> Self {
        OBJECT_COUNT.fetch_add(1, Ordering::SeqCst);
        Self {
            commands,
            index: Cell::new(0),
        }
    }
}

impl Drop for CommandEnumerator {
    fn drop(&mut self) {
        OBJECT_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

impl IEnumExplorerCommand_Impl for CommandEnumerator_Impl {
    fn Next(
        &self,
        celt: u32,
        command: *mut Option<IExplorerCommand>,
        fetched: *mut u32,
    ) -> windows::core::HRESULT {
        if command.is_null() {
            return E_NOINTERFACE;
        }

        let mut count = 0u32;
        let mut index = self.index.get();
        while count < celt && index < self.commands.len() {
            let item: IExplorerCommand = LinkForgeCommand::child(self.commands[index]).into();
            unsafe {
                command.add(count as usize).write(Some(item));
            }
            count += 1;
            index += 1;
        }
        self.index.set(index);

        if !fetched.is_null() {
            unsafe {
                *fetched = count;
            }
        }

        if count == celt { S_OK } else { S_FALSE }
    }

    fn Skip(&self, celt: u32) -> Result<()> {
        self.index
            .set((self.index.get() + celt as usize).min(self.commands.len()));
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        self.index.set(0);
        Ok(())
    }

    fn Clone(&self) -> Result<IEnumExplorerCommand> {
        Ok(CommandEnumerator {
            commands: self.commands.clone(),
            index: Cell::new(self.index.get()),
        }
        .into())
    }
}

#[derive(Default)]
struct SelectionInfo {
    count: u32,
    first_is_dir: bool,
    paths: Vec<String>,
}

fn selection_info(items: &IShellItemArray) -> SelectionInfo {
    let paths = selected_paths(items);
    let first_is_dir = paths
        .first()
        .map(|path| PathBuf::from(path).is_dir())
        .unwrap_or(false);
    SelectionInfo {
        count: paths.len() as u32,
        first_is_dir,
        paths,
    }
}

impl SelectionInfo {
    fn first_name(&self) -> Option<String> {
        self.paths.first().and_then(path_display_name)
    }

    fn target_dir(&self) -> Option<PathBuf> {
        if self.count != 1 || !self.first_is_dir {
            return None;
        }
        self.paths.first().map(PathBuf::from)
    }
}

#[derive(Clone, Copy)]
enum LinkKind {
    Symlink,
    Hardlink,
}

enum ConflictChoice {
    Overwrite,
    Rename,
    Cancel,
}

fn invoke_drop_link(selection: &SelectionInfo, kind: LinkKind) {
    match drop_link(selection, kind) {
        Ok(Some(link)) => show_info(&format!("Created link:\n{}", link.display())),
        Ok(None) => {}
        Err(error) => show_error(&format!("Failed to create link:\n{error}")),
    }
}

fn spawn_gui_action(action: &str, paths: &[String]) -> io::Result<()> {
    let gui = current_module_dir()
        .map(|dir| dir.join("linkforge-gui.exe"))
        .unwrap_or_else(|| PathBuf::from("linkforge-gui.exe"));
    let mut command = Command::new(gui);
    command.arg("--context-action").arg(action).arg("--paths");
    for path in paths {
        command.arg(path);
    }
    command.spawn().map(|_| ())
}

fn drop_link(selection: &SelectionInfo, kind: LinkKind) -> io::Result<Option<PathBuf>> {
    let Some(source) = picked_source() else {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            "no link source has been picked",
        ));
    };
    let Some(target_dir) = selection.target_dir() else {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "select a target directory or right-click a directory background",
        ));
    };

    if matches!(kind, LinkKind::Hardlink) && !source.is_file() {
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

    if link.exists() || fs::symlink_metadata(&link).is_ok() {
        match ask_conflict(&link) {
            ConflictChoice::Overwrite => force = true,
            ConflictChoice::Rename => link = available_link_path(&target_dir, Path::new(file_name)),
            ConflictChoice::Cancel => return Ok(None),
        }
    }

    match kind {
        LinkKind::Symlink => linkforge_core::create_symlink(&source, &link, force),
        LinkKind::Hardlink => linkforge_core::create_hard_link(&source, &link, force),
    }
    .map_err(link_error)?;

    Ok(Some(link))
}

fn link_error(mut error: io::Error) -> io::Error {
    if error.raw_os_error() == Some(1314) {
        error = io::Error::new(
            error.kind(),
            format!(
                "{error}. Creating symbolic links on Windows requires administrator privileges or Developer Mode."
            ),
        );
    }
    error
}

fn picked_source() -> Option<PathBuf> {
    let value = fs::read_to_string(picked_source_state_path()).ok()?;
    let path = PathBuf::from(value.trim());
    path.exists().then_some(path)
}

fn picked_source_name() -> Option<String> {
    picked_source().and_then(|path| path.file_name().map(|name| name.to_string_lossy().into()))
}

fn picked_source_state_path() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
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

fn path_display_name(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

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

fn show_info(message: &str) {
    message_box(message, "LinkForge", MB_OK | MB_ICONINFORMATION);
}

fn show_error(message: &str) {
    message_box(message, "LinkForge", MB_OK | MB_ICONERROR);
}

fn message_box(
    message: &str,
    title: &str,
    style: windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE,
) -> windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_RESULT {
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

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn selected_paths(items: &IShellItemArray) -> Vec<String> {
    let mut paths = Vec::new();
    let count = unsafe { items.GetCount().unwrap_or(0) };
    for index in 0..count {
        let Ok(item) = (unsafe { items.GetItemAt(index) }) else {
            continue;
        };
        let Ok(name) = (unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }) else {
            continue;
        };
        if let Ok(path) = unsafe { name.to_string() } {
            paths.push(path);
        }
        unsafe {
            CoTaskMemFree(Some(name.0.cast()));
        }
    }
    paths
}

fn current_module_dir() -> Option<PathBuf> {
    let mut module = HMODULE::default();
    let flags =
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
    let address = DllCanUnloadNow as *const () as *const u16;
    unsafe {
        GetModuleHandleExW(flags, PCWSTR(address), &mut module).ok()?;
    }

    let mut buffer = [0u16; 32768];
    let len = unsafe { GetModuleFileNameW(Some(module), &mut buffer) } as usize;
    if len == 0 {
        return None;
    }
    let path = String::from_utf16_lossy(&buffer[..len]);
    PathBuf::from(path).parent().map(PathBuf::from)
}

fn alloc_wide(value: &str) -> Result<PWSTR> {
    let mut wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_len = wide.len() * std::mem::size_of::<u16>();
    let raw = unsafe { CoTaskMemAlloc(byte_len) } as *mut u16;
    if raw.is_null() {
        return Err(windows::core::Error::from_win32());
    }

    unsafe {
        raw.copy_from_nonoverlapping(wide.as_mut_ptr(), wide.len());
    }
    Ok(PWSTR(raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_kinds_map_to_gui_actions() {
        assert_eq!(CommandKind::PickSource.action(), None);
        assert_eq!(CommandKind::DropSymlink.action(), None);
        assert_eq!(CommandKind::DropHardlink.action(), None);
        assert_eq!(CommandKind::Symlink.action(), Some("symlink"));
        assert_eq!(CommandKind::Hardlink.action(), Some("hardlink"));
        assert_eq!(CommandKind::LinkCount.action(), Some("link-count"));
        assert_eq!(CommandKind::Siblings.action(), Some("siblings"));
        assert_eq!(CommandKind::ScanGroups.action(), Some("scan-groups"));
        assert_eq!(CommandKind::CloneTree.action(), Some("clone-tree"));
    }

    #[test]
    fn creates_available_link_names() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("source.txt"), "hello").unwrap();

        assert_eq!(
            available_link_path(temp.path(), Path::new("source.txt")),
            temp.path().join("source - Link.txt")
        );

        fs::write(temp.path().join("source - Link.txt"), "hello").unwrap();
        assert_eq!(
            available_link_path(temp.path(), Path::new("source.txt")),
            temp.path().join("source - Link (2).txt")
        );
    }

    #[test]
    fn creates_available_link_names_without_extensions() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("folder")).unwrap();

        assert_eq!(
            available_link_path(temp.path(), Path::new("folder")),
            temp.path().join("folder - Link")
        );
    }
}
