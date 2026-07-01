#![cfg(windows)]

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::Win32::Foundation::{E_NOINTERFACE, E_NOTIMPL, E_POINTER, HMODULE, S_FALSE, S_OK};
use windows::Win32::System::Com::{
    CoTaskMemAlloc, CoTaskMemFree, IBindCtx, IClassFactory, IClassFactory_Impl, IServiceProvider,
};
use windows::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};
use windows::Win32::System::Ole::{IObjectWithSite, IObjectWithSite_Impl};
use windows::Win32::UI::Shell::{
    ECF_DEFAULT, ECF_HASSUBCOMMANDS, ECS_ENABLED, ECS_HIDDEN, GPFIDL_DEFAULT, IEnumExplorerCommand,
    IEnumExplorerCommand_Impl, IExplorerCommand, IExplorerCommand_Impl, IFolderView,
    IPersistFolder2, IShellBrowser, IShellItemArray, SHGetPathFromIDListEx, SID_STopLevelBrowser,
    SIGDN_FILESYSPATH,
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
    SameFile,
    LinkCount,
    Siblings,
    ScanGroups,
    CloneTree,
}

const CHILD_COMMANDS: [CommandKind; 10] = [
    CommandKind::PickSource,
    CommandKind::DropSymlink,
    CommandKind::DropHardlink,
    CommandKind::Symlink,
    CommandKind::Hardlink,
    CommandKind::SameFile,
    CommandKind::LinkCount,
    CommandKind::Siblings,
    CommandKind::ScanGroups,
    CommandKind::CloneTree,
];

impl CommandKind {
    fn title(self, selection: Option<&SelectionInfo>) -> String {
        match self {
            Self::Root => "LinkForge".to_string(),
            Self::PickSource => selection
                .map(SelectionInfo::pick_source_title)
                .unwrap_or_else(|| "Pick Link Source".to_string()),
            Self::DropSymlink => picked_sources_title(linkforge_shared::MenuLinkKind::Symlink),
            Self::DropHardlink => picked_sources_title(linkforge_shared::MenuLinkKind::Hardlink),
            Self::Symlink => "Open Symlink in LinkForge...".to_string(),
            Self::Hardlink => "Open Hard Link in LinkForge...".to_string(),
            Self::SameFile => "Compare Same File".to_string(),
            Self::LinkCount => "Show Link Count".to_string(),
            Self::Siblings => "Find Hard Link Siblings...".to_string(),
            Self::ScanGroups => "Scan Hard Link Groups".to_string(),
            Self::CloneTree => "Clone Tree Preserving Hard Links...".to_string(),
        }
    }

    fn action(self) -> Option<&'static str> {
        match self {
            Self::Root => None,
            Self::PickSource => Some(linkforge_shared::action::PICK_SOURCE),
            Self::DropSymlink => Some(linkforge_shared::action::DROP_SYMLINK),
            Self::DropHardlink => Some(linkforge_shared::action::DROP_HARDLINK),
            Self::Symlink => Some(linkforge_shared::action::SYMLINK),
            Self::Hardlink => Some(linkforge_shared::action::HARDLINK),
            Self::SameFile => Some(linkforge_shared::action::SAME_FILE),
            Self::LinkCount => Some(linkforge_shared::action::LINK_COUNT),
            Self::Siblings => Some(linkforge_shared::action::SIBLINGS),
            Self::ScanGroups => Some(linkforge_shared::action::SCAN_GROUPS),
            Self::CloneTree => Some(linkforge_shared::action::CLONE_TREE),
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
            Self::SameFile => GUID::from_u128(0x340c50d3_178a_4c66_a97c_0ef3c087de62),
            Self::LinkCount => GUID::from_u128(0x7918bb2c_9693_4026_a435_e1860ee35587),
            Self::Siblings => GUID::from_u128(0xaf16a719_b2e6_4263_a3af_dee068e736b2),
            Self::ScanGroups => GUID::from_u128(0x55b7a2f4_6316_40a3_8ef9_7caefbf70ab3),
            Self::CloneTree => GUID::from_u128(0xa8325431_f2d6_4c46_8f6a_7fb2cbe63a6d),
        }
    }

    fn supports(self, selection: &SelectionInfo) -> bool {
        self.supports_with_picked(selection, !picked_sources().is_empty())
    }

    fn supports_with_picked(self, selection: &SelectionInfo, has_picked_sources: bool) -> bool {
        match self {
            Self::Root => CHILD_COMMANDS
                .iter()
                .any(|kind| kind.supports_with_picked(selection, has_picked_sources)),
            Self::PickSource => selection.count >= 1,
            Self::DropSymlink | Self::DropHardlink => {
                (selection.background_target || selection.count == 1)
                    && selection.target_dir().is_some()
                    && has_picked_sources
            }
            Self::Symlink | Self::Siblings => selection.count == 1,
            Self::Hardlink | Self::LinkCount => selection.count == 1 && !selection.first_is_dir,
            Self::SameFile => selection.count == 2 && selection.all_are_files,
            Self::ScanGroups | Self::CloneTree => selection.count == 1 && selection.first_is_dir,
        }
    }
}

#[implement(IExplorerCommand, IObjectWithSite)]
struct LinkForgeCommand {
    kind: CommandKind,
    site: RefCell<Option<IUnknown>>,
}

impl LinkForgeCommand {
    fn root() -> Self {
        OBJECT_COUNT.fetch_add(1, Ordering::SeqCst);
        Self {
            kind: CommandKind::Root,
            site: RefCell::new(None),
        }
    }

    fn child(kind: CommandKind, site: Option<IUnknown>) -> Self {
        OBJECT_COUNT.fetch_add(1, Ordering::SeqCst);
        Self {
            kind,
            site: RefCell::new(site),
        }
    }
}

impl Drop for LinkForgeCommand {
    fn drop(&mut self) {
        OBJECT_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

impl IObjectWithSite_Impl for LinkForgeCommand_Impl {
    fn SetSite(&self, site: Ref<'_, IUnknown>) -> Result<()> {
        *self.site.borrow_mut() = site.cloned();
        Ok(())
    }

    fn GetSite(&self, riid: *const GUID, site: *mut *mut c_void) -> Result<()> {
        if site.is_null() {
            return Err(E_POINTER.into());
        }

        unsafe {
            *site = null_mut();
        }

        let Some(stored_site) = self.site.borrow().clone() else {
            return Err(E_NOINTERFACE.into());
        };

        unsafe { stored_site.query(riid, site).ok() }
    }
}

impl IExplorerCommand_Impl for LinkForgeCommand_Impl {
    fn GetTitle(&self, items: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        let site = self.site.borrow().clone();
        let selection = selection_info(items.as_ref(), site.as_ref());
        alloc_wide(&self.kind.title(Some(&selection)))
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
        let site = self.site.borrow().clone();
        let selection = selection_info(items.as_ref(), site.as_ref());
        if self.kind.supports(&selection) {
            Ok(ECS_ENABLED.0 as u32)
        } else {
            Ok(ECS_HIDDEN.0 as u32)
        }
    }

    fn Invoke(&self, items: Ref<'_, IShellItemArray>, _bind_ctx: Ref<'_, IBindCtx>) -> Result<()> {
        let site = self.site.borrow().clone();
        let selection = selection_info(items.as_ref(), site.as_ref());

        let Some(action) = self.kind.action() else {
            return Ok(());
        };

        let action_paths = selection.action_paths();
        if action_paths.is_empty() {
            return Ok(());
        }

        let background_target = selection.background_target
            && matches!(
                self.kind,
                CommandKind::DropSymlink | CommandKind::DropHardlink
            );
        let _child = spawn_gui_action(action, &action_paths, background_target);
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

        Ok(CommandEnumerator::new(CHILD_COMMANDS.to_vec(), self.site.borrow().clone()).into())
    }
}

#[implement(IEnumExplorerCommand)]
struct CommandEnumerator {
    commands: Vec<CommandKind>,
    index: Cell<usize>,
    site: Option<IUnknown>,
}

impl CommandEnumerator {
    fn new(commands: Vec<CommandKind>, site: Option<IUnknown>) -> Self {
        OBJECT_COUNT.fetch_add(1, Ordering::SeqCst);
        Self {
            commands,
            index: Cell::new(0),
            site,
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
            let item: IExplorerCommand =
                LinkForgeCommand::child(self.commands[index], self.site.clone()).into();
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
            site: self.site.clone(),
        }
        .into())
    }
}

#[derive(Default)]
struct SelectionInfo {
    count: u32,
    first_is_dir: bool,
    all_are_files: bool,
    target_dir: Option<PathBuf>,
    background_target: bool,
    paths: Vec<String>,
}

fn selection_info(items: Option<&IShellItemArray>, site: Option<&IUnknown>) -> SelectionInfo {
    let paths = items.map(selected_paths).unwrap_or_default();
    let background_dir = if paths.is_empty() {
        background_dir_from_site(site)
    } else {
        None
    };

    SelectionInfo::from_paths(paths, background_dir)
}

impl SelectionInfo {
    fn from_paths(paths: Vec<String>, background_dir: Option<PathBuf>) -> Self {
        let first_is_dir = paths
            .first()
            .map(|path| PathBuf::from(path).is_dir())
            .unwrap_or(false);
        let all_are_files =
            !paths.is_empty() && paths.iter().all(|path| PathBuf::from(path).is_file());
        let target_dir = target_dir_for_paths(&paths, background_dir);
        let background_target = paths.is_empty() && target_dir.is_some();

        Self {
            count: paths.len() as u32,
            first_is_dir,
            all_are_files,
            target_dir,
            background_target,
            paths,
        }
    }

    fn pick_source_title(&self) -> String {
        linkforge_shared::pick_source_title(self.count, self.first_name().as_deref())
    }

    fn first_name(&self) -> Option<String> {
        self.paths.first().and_then(path_display_name)
    }

    fn target_dir(&self) -> Option<PathBuf> {
        self.target_dir.clone()
    }

    fn action_paths(&self) -> Vec<String> {
        if self.background_target {
            return self
                .target_dir
                .iter()
                .map(|path| path.display().to_string())
                .collect();
        }
        self.paths.clone()
    }
}

fn target_dir_for_paths(paths: &[String], background_dir: Option<PathBuf>) -> Option<PathBuf> {
    if paths.is_empty() {
        return background_dir.filter(|path| path.is_dir());
    }

    if paths.len() == 1 {
        let path = PathBuf::from(&paths[0]);
        if path.is_dir() {
            return Some(path);
        }
    }

    None
}

fn background_dir_from_site(site: Option<&IUnknown>) -> Option<PathBuf> {
    let browser = shell_browser_from_site(site?)?;
    let view = unsafe { browser.QueryActiveShellView().ok()? };
    let folder_view: IFolderView = view.cast().ok()?;
    let persist_folder: IPersistFolder2 = unsafe { folder_view.GetFolder().ok()? };
    let pidl = unsafe { persist_folder.GetCurFolder().ok()? };
    if pidl.is_null() {
        return None;
    }

    let mut buffer = vec![0u16; 32768];
    let ok = unsafe { SHGetPathFromIDListEx(pidl, &mut buffer, GPFIDL_DEFAULT) }.as_bool();
    unsafe {
        CoTaskMemFree(Some(pidl.cast()));
    }

    if !ok {
        return None;
    }

    let len = buffer.iter().position(|value| *value == 0)?;
    if len == 0 {
        return None;
    }

    Some(PathBuf::from(String::from_utf16_lossy(&buffer[..len])))
}

fn shell_browser_from_site(site: &IUnknown) -> Option<IShellBrowser> {
    let provider: IServiceProvider = site.cast().ok()?;
    unsafe { provider.QueryService(&SID_STopLevelBrowser).ok() }
}

fn spawn_gui_action(action: &str, paths: &[String], background_target: bool) -> io::Result<()> {
    let gui = current_module_dir()
        .map(|dir| dir.join("linkforge-gui.exe"))
        .unwrap_or_else(|| PathBuf::from("linkforge-gui.exe"));
    let mut command = Command::new(gui);
    command.arg("--context-action").arg(action);
    if background_target {
        command.arg("--context-background");
    }
    command.arg("--paths");
    for path in paths {
        command.arg(path);
    }
    command.spawn().map(|_| ())
}

fn picked_sources() -> Vec<PathBuf> {
    linkforge_shared::picked_sources()
}

fn picked_sources_title(kind: linkforge_shared::MenuLinkKind) -> String {
    let sources = picked_sources();
    picked_sources_title_for(kind, &sources)
}

fn picked_sources_title_for(kind: linkforge_shared::MenuLinkKind, sources: &[PathBuf]) -> String {
    linkforge_shared::picked_sources_title(kind, sources)
}

fn path_display_name(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
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
    use std::fs;

    use super::*;

    #[test]
    fn command_kinds_map_to_gui_actions() {
        assert_eq!(CommandKind::PickSource.action(), Some("pick-source"));
        assert_eq!(CommandKind::DropSymlink.action(), Some("drop-symlink"));
        assert_eq!(CommandKind::DropHardlink.action(), Some("drop-hardlink"));
        assert_eq!(CommandKind::Symlink.action(), Some("symlink"));
        assert_eq!(CommandKind::Hardlink.action(), Some("hardlink"));
        assert_eq!(CommandKind::SameFile.action(), Some("same-file"));
        assert_eq!(CommandKind::LinkCount.action(), Some("link-count"));
        assert_eq!(CommandKind::Siblings.action(), Some("siblings"));
        assert_eq!(CommandKind::ScanGroups.action(), Some("scan-groups"));
        assert_eq!(CommandKind::CloneTree.action(), Some("clone-tree"));
    }

    #[test]
    fn same_file_command_supports_exactly_two_files() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first.txt");
        let second = temp.path().join("second.txt");
        let third = temp.path().join("third.txt");
        let directory = temp.path().join("directory");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();
        fs::write(&third, "third").unwrap();
        fs::create_dir(&directory).unwrap();

        assert!(
            CommandKind::SameFile.supports(&selection_for_paths([first.clone(), second.clone(),]))
        );
        assert!(!CommandKind::SameFile.supports(&selection_for_paths([first.clone()])));
        assert!(!CommandKind::SameFile.supports(&selection_for_paths([
            directory.clone(),
            temp.path().to_path_buf(),
        ])));
        assert!(!CommandKind::SameFile.supports(&selection_for_paths([first.clone(), directory,])));
        assert!(!CommandKind::SameFile.supports(&selection_for_paths([first, second, third,])));
    }

    #[test]
    fn pick_source_supports_single_and_multi_selection() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first.txt");
        let second = temp.path().join("second.txt");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();

        assert!(CommandKind::PickSource.supports(&selection_for_paths([first.clone()])));
        assert!(CommandKind::PickSource.supports(&selection_for_paths([first, second])));
    }

    #[test]
    fn target_dir_requires_exactly_one_directory() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        let file = temp.path().join("file.txt");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        fs::write(&file, "file").unwrap();

        assert_eq!(
            selection_for_paths([first.clone()]).target_dir(),
            Some(first)
        );
        assert!(selection_for_paths([file]).target_dir().is_none());
        assert!(
            selection_for_paths([second.clone(), temp.path().to_path_buf()])
                .target_dir()
                .is_none()
        );
    }

    #[test]
    fn target_dir_accepts_directory_background() {
        let temp = tempfile::tempdir().unwrap();
        let selection = selection_for_background(temp.path().to_path_buf());

        assert_eq!(selection.count, 0);
        assert!(selection.background_target);
        assert_eq!(selection.target_dir(), Some(temp.path().to_path_buf()));
        assert_eq!(
            selection.action_paths(),
            vec![temp.path().display().to_string()]
        );
    }

    #[test]
    fn drop_commands_require_single_target_and_picked_sources() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("target");
        let second = temp.path().join("second");
        let file = temp.path().join("file.txt");
        fs::create_dir(&directory).unwrap();
        fs::create_dir(&second).unwrap();
        fs::write(&file, "file").unwrap();

        let selected_directory = selection_for_paths([directory]);
        let background = selection_for_background(temp.path().to_path_buf());
        let selected_file = selection_for_paths([file]);
        let two_directories = selection_for_paths([temp.path().join("target"), second]);

        assert!(CommandKind::DropSymlink.supports_with_picked(&selected_directory, true));
        assert!(CommandKind::DropHardlink.supports_with_picked(&background, true));
        assert!(!CommandKind::DropSymlink.supports_with_picked(&selected_directory, false));
        assert!(!CommandKind::DropHardlink.supports_with_picked(&selected_file, true));
        assert!(!CommandKind::DropSymlink.supports_with_picked(&two_directories, true));
    }

    #[test]
    fn root_visibility_tracks_available_child_commands() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("file.txt");
        fs::write(&file, "file").unwrap();

        let empty_background = SelectionInfo::default();
        let target_background = selection_for_background(temp.path().to_path_buf());
        let selected_file = selection_for_paths([file]);

        assert!(!CommandKind::Root.supports_with_picked(&empty_background, false));
        assert!(!CommandKind::Root.supports_with_picked(&target_background, false));
        assert!(CommandKind::Root.supports_with_picked(&target_background, true));
        assert!(CommandKind::Root.supports_with_picked(&selected_file, false));
    }

    #[test]
    fn picked_source_titles_handle_multi_source_and_directory_hardlinks() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("file.txt");
        let directory = temp.path().join("directory");
        fs::write(&file, "file").unwrap();
        fs::create_dir(&directory).unwrap();

        assert_eq!(
            picked_sources_title_for(
                linkforge_shared::MenuLinkKind::Symlink,
                std::slice::from_ref(&file)
            ),
            "Create Symlink from file.txt"
        );
        assert_eq!(
            picked_sources_title_for(
                linkforge_shared::MenuLinkKind::Hardlink,
                std::slice::from_ref(&directory)
            ),
            "Create Hard-Link Tree from directory"
        );
        assert_eq!(
            picked_sources_title_for(linkforge_shared::MenuLinkKind::Hardlink, &[file, directory]),
            "Create Hard Links from 2 Sources"
        );
    }

    fn selection_for_paths<const N: usize>(paths: [PathBuf; N]) -> SelectionInfo {
        SelectionInfo::from_paths(
            paths
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
            None,
        )
    }

    fn selection_for_background(path: PathBuf) -> SelectionInfo {
        SelectionInfo::from_paths(Vec::new(), Some(path))
    }
}
