#![cfg(windows)]

use std::cell::Cell;
use std::ffi::c_void;
use std::path::PathBuf;
use std::process::Command;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::Win32::Foundation::{E_NOINTERFACE, E_NOTIMPL, HMODULE, S_FALSE, S_OK};
use windows::Win32::System::Com::{
    CoTaskMemAlloc, CoTaskMemFree, IBindCtx, IClassFactory, IClassFactory_Impl,
};
use windows::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};
use windows::Win32::UI::Shell::{
    ECF_DEFAULT, ECF_HASSUBCOMMANDS, ECS_DISABLED, ECS_ENABLED, ECS_HIDDEN, IEnumExplorerCommand,
    IEnumExplorerCommand_Impl, IExplorerCommand, IExplorerCommand_Impl, IShellItemArray,
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
    Symlink,
    Hardlink,
    LinkCount,
    Siblings,
    ScanGroups,
    CloneTree,
}

impl CommandKind {
    fn title(self) -> &'static str {
        match self {
            Self::Root => "LinkForge",
            Self::Symlink => "Create Symbolic Link...",
            Self::Hardlink => "Create Hard Link...",
            Self::LinkCount => "Show Link Count",
            Self::Siblings => "Find Hard Link Siblings...",
            Self::ScanGroups => "Scan Hard Link Groups",
            Self::CloneTree => "Clone Tree Preserving Hard Links...",
        }
    }

    fn action(self) -> Option<&'static str> {
        match self {
            Self::Root => None,
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
            Self::Root => selection.count > 0,
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
    fn GetTitle(&self, _items: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        alloc_wide(self.kind.title())
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
        let Some(items) = items.as_ref() else {
            return Ok(ECS_HIDDEN.0 as u32);
        };
        let selection = selection_info(items);
        if self.kind.supports(&selection) {
            Ok(ECS_ENABLED.0 as u32)
        } else if matches!(self.kind, CommandKind::Root) {
            Ok(ECS_DISABLED.0 as u32)
        } else {
            Ok(ECS_HIDDEN.0 as u32)
        }
    }

    fn Invoke(&self, items: Ref<'_, IShellItemArray>, _bind_ctx: Ref<'_, IBindCtx>) -> Result<()> {
        let Some(action) = self.kind.action() else {
            return Ok(());
        };
        let Some(items) = items.as_ref() else {
            return Ok(());
        };

        let paths = selected_paths(items);
        if paths.is_empty() {
            return Ok(());
        }

        let gui = current_module_dir()
            .map(|dir| dir.join("linkforge-gui.exe"))
            .unwrap_or_else(|| PathBuf::from("linkforge-gui.exe"));
        let mut command = Command::new(gui);
        command.arg("--context-action").arg(action).arg("--paths");
        for path in paths {
            command.arg(path);
        }

        let _child = command.spawn();
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

struct SelectionInfo {
    count: u32,
    first_is_dir: bool,
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
    }
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
        assert_eq!(CommandKind::Symlink.action(), Some("symlink"));
        assert_eq!(CommandKind::Hardlink.action(), Some("hardlink"));
        assert_eq!(CommandKind::LinkCount.action(), Some("link-count"));
        assert_eq!(CommandKind::Siblings.action(), Some("siblings"));
        assert_eq!(CommandKind::ScanGroups.action(), Some("scan-groups"));
        assert_eq!(CommandKind::CloneTree.action(), Some("clone-tree"));
    }
}
