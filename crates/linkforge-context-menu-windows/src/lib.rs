#![cfg(windows)]

use std::cell::{Cell, RefCell};
use std::env;
use std::ffi::c_void;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::Win32::Foundation::{
    E_NOINTERFACE, E_NOTIMPL, E_POINTER, HMODULE, HWND, S_FALSE, S_OK,
};
use windows::Win32::System::Com::{
    CoTaskMemAlloc, CoTaskMemFree, IBindCtx, IClassFactory, IClassFactory_Impl, IServiceProvider,
};
use windows::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW, GetProcAddress, LoadLibraryW,
};
use windows::Win32::System::Ole::{IObjectWithSite, IObjectWithSite_Impl};
use windows::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TDF_ALLOW_DIALOG_CANCELLATION,
    TDF_POSITION_RELATIVE_TO_WINDOW, TDF_SIZE_TO_CONTENT, TDF_USE_COMMAND_LINKS,
};
use windows::Win32::UI::Shell::{
    ECF_DEFAULT, ECF_HASSUBCOMMANDS, ECS_ENABLED, ECS_HIDDEN, GPFIDL_DEFAULT, IEnumExplorerCommand,
    IEnumExplorerCommand_Impl, IExplorerCommand, IExplorerCommand_Impl, IFolderView,
    IPersistFolder2, IShellBrowser, IShellItemArray, SHGetPathFromIDListEx, SID_STopLevelBrowser,
    SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    IDCANCEL, IDNO, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK,
    MB_SETFOREGROUND, MB_TASKMODAL, MB_TOPMOST, MB_YESNO, MB_YESNOCANCEL, MESSAGEBOX_RESULT,
    MESSAGEBOX_STYLE, MessageBoxW,
};
use windows::core::{
    BOOL, GUID, HRESULT, IUnknown, Interface, PCSTR, PCWSTR, PWSTR, Ref, Result, implement,
};

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
            Self::DropSymlink => picked_sources_title(LinkKind::Symlink),
            Self::DropHardlink => picked_sources_title(LinkKind::Hardlink),
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
            Self::Root | Self::PickSource | Self::DropSymlink | Self::DropHardlink => None,
            Self::Symlink => Some("symlink"),
            Self::Hardlink => Some("hardlink"),
            Self::SameFile => Some("same-file"),
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
        let owner = explorer_owner_hwnd(site.as_ref());

        match self.kind {
            CommandKind::PickSource => {
                if let Err(error) = spawn_gui_action("pick-source", &selection.paths) {
                    show_error(owner, &format!("Failed to pick link source:\n{error}"));
                }
                return Ok(());
            }
            CommandKind::DropSymlink => {
                invoke_drop_link(&selection, LinkKind::Symlink, owner);
                return Ok(());
            }
            CommandKind::DropHardlink => {
                invoke_drop_link(&selection, LinkKind::Hardlink, owner);
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
        if self.count > 1 {
            format!("Pick {} Link Sources", self.count)
        } else {
            self.first_name()
                .map(|name| format!("Pick Link Source: {name}"))
                .unwrap_or_else(|| "Pick Link Source".to_string())
        }
    }

    fn first_name(&self) -> Option<String> {
        self.paths.first().and_then(path_display_name)
    }

    fn target_dir(&self) -> Option<PathBuf> {
        self.target_dir.clone()
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

fn explorer_owner_hwnd(site: Option<&IUnknown>) -> HWND {
    let Some(site) = site else {
        return HWND::default();
    };
    let Some(browser) = shell_browser_from_site(site) else {
        return HWND::default();
    };

    unsafe { browser.GetWindow().unwrap_or_default() }
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

#[derive(Clone, Copy)]
enum LinkKind {
    Symlink,
    Hardlink,
}

impl LinkKind {
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
    owner: HWND,
}

impl ConflictResolver {
    fn new(owner: HWND) -> Self {
        Self {
            applied_choice: None,
            owner,
        }
    }

    fn resolve(&mut self, path: &Path, allow_apply_to_remaining: bool) -> ConflictChoice {
        if let Some(choice) = self.applied_choice {
            return choice;
        }

        let answer = ask_conflict(path, allow_apply_to_remaining, self.owner);
        if answer.apply_to_remaining && !matches!(answer.choice, ConflictChoice::Cancel) {
            self.applied_choice = Some(answer.choice);
        }
        answer.choice
    }

    #[cfg(test)]
    fn with_applied(choice: ConflictChoice) -> Self {
        Self {
            applied_choice: Some(choice),
            owner: HWND::default(),
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
    fn message(&self, kind: LinkKind) -> String {
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

fn invoke_drop_link(selection: &SelectionInfo, kind: LinkKind, owner: HWND) {
    match drop_links(selection, kind, owner) {
        Ok(summary) => show_info(owner, &summary.message(kind)),
        Err(error) => show_error(owner, &format!("Failed to create link:\n{error}")),
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

fn drop_links(
    selection: &SelectionInfo,
    kind: LinkKind,
    owner: HWND,
) -> io::Result<BatchLinkSummary> {
    let sources = picked_sources();
    if sources.is_empty() {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            "no link source has been picked",
        ));
    }
    let Some(target_dir) = selection.target_dir() else {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "select a target directory or right-click a directory background",
        ));
    };

    let mut resolver = ConflictResolver::new(owner);
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
    kind: LinkKind,
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

fn create_one_link(source: &Path, link: &Path, kind: LinkKind, force: bool) -> io::Result<()> {
    match kind {
        LinkKind::Symlink => linkforge_core::create_symlink(source, link, force),
        LinkKind::Hardlink if source.is_dir() => {
            linkforge_core::create_hard_link_tree(source, link, force)
        }
        LinkKind::Hardlink => linkforge_core::create_hard_link(source, link, force),
    }
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

fn picked_sources_title(kind: LinkKind) -> String {
    let sources = picked_sources();
    picked_sources_title_for(kind, &sources)
}

fn picked_sources_title_for(kind: LinkKind, sources: &[PathBuf]) -> String {
    match (kind, sources) {
        (_, []) => match kind {
            LinkKind::Symlink => "Create Symlink from Picked Source".to_string(),
            LinkKind::Hardlink => "Create Hard Link from Picked Source".to_string(),
        },
        (LinkKind::Symlink, [source]) => path_display_name(source)
            .map(|name| format!("Create Symlink from {name}"))
            .unwrap_or_else(|| "Create Symlink from Picked Source".to_string()),
        (LinkKind::Hardlink, [source]) if source.is_dir() => path_display_name(source)
            .map(|name| format!("Create Hard-Link Tree from {name}"))
            .unwrap_or_else(|| "Create Hard-Link Tree from Picked Source".to_string()),
        (LinkKind::Hardlink, [source]) => path_display_name(source)
            .map(|name| format!("Create Hard Link from {name}"))
            .unwrap_or_else(|| "Create Hard Link from Picked Source".to_string()),
        (LinkKind::Symlink, sources) => {
            format!("Create Symlinks from {} Sources", sources.len())
        }
        (LinkKind::Hardlink, sources) => {
            format!("Create Hard Links from {} Sources", sources.len())
        }
    }
}

fn picked_source_state_path() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("LinkForge")
        .join("picked-source.txt")
}

fn picked_sources_state_path() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("LinkForge")
        .join("picked-sources.json")
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

fn ask_conflict(path: &Path, allow_apply_to_remaining: bool, owner: HWND) -> ConflictAnswer {
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
    config.hwndParent = owner;
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
        return ask_conflict_with_message_box(path, allow_apply_to_remaining, owner);
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

fn ask_conflict_with_message_box(
    path: &Path,
    allow_apply_to_remaining: bool,
    owner: HWND,
) -> ConflictAnswer {
    let first_prompt = format!(
        "The target already exists:\n{}\n\nChoose Yes to create an automatically renamed link.\nChoose No for overwrite or skip options.\nChoose Cancel to stop this operation.",
        path.display()
    );
    let first = message_box(
        owner,
        &first_prompt,
        "LinkForge",
        MB_YESNOCANCEL | MB_ICONINFORMATION,
    );

    let choice = if first == IDNO {
        let second = ask_overwrite_or_skip_with_message_box(path, owner);
        conflict_choice_from_message_box_results(first, Some(second))
    } else {
        conflict_choice_from_message_box_results(first, None)
    };

    let apply_to_remaining = allow_apply_to_remaining
        && !matches!(choice, ConflictChoice::Cancel)
        && ask_apply_to_remaining_with_message_box(owner);

    ConflictAnswer {
        choice,
        apply_to_remaining,
    }
}

fn ask_overwrite_or_skip_with_message_box(path: &Path, owner: HWND) -> MESSAGEBOX_RESULT {
    let prompt = format!(
        "The target already exists:\n{}\n\nChoose Yes to overwrite the existing file or symbolic link.\nChoose No to skip this source.\nChoose Cancel to stop this operation.",
        path.display()
    );
    message_box(
        owner,
        &prompt,
        "LinkForge",
        MB_YESNOCANCEL | MB_ICONINFORMATION,
    )
}

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

fn ask_apply_to_remaining_with_message_box(owner: HWND) -> bool {
    let result = message_box(
        owner,
        "Apply this conflict choice to remaining conflicts in this batch?",
        "LinkForge",
        MB_YESNO | MB_ICONQUESTION,
    );
    result == IDYES
}

type TaskDialogIndirectProc =
    unsafe extern "system" fn(*const TASKDIALOGCONFIG, *mut i32, *mut i32, *mut BOOL) -> HRESULT;

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

fn show_info(owner: HWND, message: &str) {
    message_box(owner, message, "LinkForge", MB_OK | MB_ICONINFORMATION);
}

fn show_error(owner: HWND, message: &str) {
    message_box(owner, message, "LinkForge", MB_OK | MB_ICONERROR);
}

fn message_box(
    owner: HWND,
    message: &str,
    title: &str,
    style: MESSAGEBOX_STYLE,
) -> MESSAGEBOX_RESULT {
    let message = wide_null(message);
    let title = wide_null(title);
    let owner = if owner.0.is_null() { None } else { Some(owner) };
    let mut style = MESSAGEBOX_STYLE(style.0 | MB_SETFOREGROUND.0 | MB_TOPMOST.0);
    if owner.is_none() {
        style = MESSAGEBOX_STYLE(style.0 | MB_TASKMODAL.0);
    }
    unsafe {
        MessageBoxW(
            owner,
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
            picked_sources_title_for(LinkKind::Symlink, std::slice::from_ref(&file)),
            "Create Symlink from file.txt"
        );
        assert_eq!(
            picked_sources_title_for(LinkKind::Hardlink, std::slice::from_ref(&directory)),
            "Create Hard-Link Tree from directory"
        );
        assert_eq!(
            picked_sources_title_for(LinkKind::Hardlink, &[file, directory]),
            "Create Hard Links from 2 Sources"
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
            LinkKind::Hardlink,
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
            LinkKind::Hardlink,
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
            LinkKind::Hardlink,
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
            LinkKind::Hardlink,
            &mut cancel,
        );
        assert_eq!(summary.created, 0);
        assert_eq!(summary.cancelled, 2);
    }

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
