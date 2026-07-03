use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const EXTENSION_FILE_NAME: &str = "linkforge.py";
const DEFAULT_GUI_EXE: &str = "linkforge-gui";

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    match parse_args(env::args().skip(1))? {
        CliCommand::Install {
            gui_exe,
            skip_gui_check,
        } => install(&gui_exe, skip_gui_check),
        CliCommand::Verify {
            gui_exe,
            skip_gui_check,
        } => verify(&gui_exe, skip_gui_check),
        CliCommand::Uninstall => uninstall(),
        CliCommand::Help => {
            print_help();
            Ok(())
        }
    }
}

enum CliCommand {
    Install {
        gui_exe: String,
        skip_gui_check: bool,
    },
    Verify {
        gui_exe: String,
        skip_gui_check: bool,
    },
    Uninstall,
    Help,
}

fn parse_args(args: impl IntoIterator<Item = String>) -> io::Result<CliCommand> {
    let mut args = args.into_iter();
    let command = args.next().unwrap_or_else(|| "help".to_string());

    match command.as_str() {
        "install" => {
            let options = parse_install_options(args)?;
            Ok(CliCommand::Install {
                gui_exe: options.gui_exe,
                skip_gui_check: options.skip_gui_check,
            })
        }
        "verify" => {
            let options = parse_install_options(args)?;
            Ok(CliCommand::Verify {
                gui_exe: options.gui_exe,
                skip_gui_check: options.skip_gui_check,
            })
        }
        "uninstall" => {
            reject_extra_args(args)?;
            Ok(CliCommand::Uninstall)
        }
        "help" | "--help" | "-h" => {
            reject_extra_args(args)?;
            Ok(CliCommand::Help)
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown command: {other}"),
        )),
    }
}

struct InstallOptions {
    gui_exe: String,
    skip_gui_check: bool,
}

fn parse_install_options(args: impl Iterator<Item = String>) -> io::Result<InstallOptions> {
    let mut gui_exe = DEFAULT_GUI_EXE.to_string();
    let mut skip_gui_check = false;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--gui-exe" => {
                gui_exe = args.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--gui-exe requires a value")
                })?;
            }
            "--skip-gui-check" => skip_gui_check = true,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown argument: {arg}"),
                ));
            }
        }
    }

    Ok(InstallOptions {
        gui_exe,
        skip_gui_check,
    })
}

fn reject_extra_args(args: impl Iterator<Item = String>) -> io::Result<()> {
    if let Some(arg) = args.into_iter().next() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown argument: {arg}"),
        ));
    }
    Ok(())
}

fn install(gui_exe: &str, skip_gui_check: bool) -> io::Result<()> {
    println!("Checking nautilus-python...");
    ensure_nautilus_python_available()?;
    println!("  ok");

    let gui_check = check_gui_exe(gui_exe, skip_gui_check)?;
    print_gui_check(&gui_check);

    let target_dir = nautilus_python_extension_dir()?;
    fs::create_dir_all(&target_dir)?;

    let path = target_dir.join(EXTENSION_FILE_NAME);
    fs::write(&path, extension_contents(gui_exe))?;

    if !path.is_file() {
        return Err(io::Error::other(format!(
            "failed to install extension at {}",
            path.display()
        )));
    }

    println!("Installing extension... ok");
    println!("Installed LinkForge GNOME Files extension:");
    println!("  {}", path.display());
    println!("Restart GNOME Files with:");
    println!("  nautilus -q");
    Ok(())
}

fn verify(gui_exe: &str, skip_gui_check: bool) -> io::Result<()> {
    println!("Checking nautilus-python...");
    ensure_nautilus_python_available()?;
    println!("  ok");

    let gui_check = check_gui_exe(gui_exe, skip_gui_check)?;
    print_gui_check(&gui_check);

    let path = nautilus_python_extension_path()?;
    if !path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "LinkForge GNOME Files extension is not installed at {}",
                path.display()
            ),
        ));
    }

    let contents = fs::read_to_string(&path)?;
    verify_extension_contents(&contents, gui_exe)?;

    println!("Verified LinkForge GNOME Files extension:");
    println!("  {}", path.display());
    Ok(())
}

fn uninstall() -> io::Result<()> {
    let path = nautilus_python_extension_path()?;
    if !path.exists() {
        println!(
            "No LinkForge GNOME Files extension found at {}.",
            path.display()
        );
        println!("Nothing to remove.");
        return Ok(());
    }

    fs::remove_file(&path)?;

    if path.exists() {
        return Err(io::Error::other(format!(
            "failed to remove extension at {}",
            path.display()
        )));
    }

    println!("Removed LinkForge GNOME Files extension:");
    println!("  {}", path.display());
    println!("Restart GNOME Files with:");
    println!("  nautilus -q");
    Ok(())
}

fn nautilus_python_extension_path() -> io::Result<PathBuf> {
    Ok(nautilus_python_extension_dir()?.join(EXTENSION_FILE_NAME))
}

fn nautilus_python_extension_dir() -> io::Result<PathBuf> {
    let home = home_dir()?;
    Ok(home
        .join(".local")
        .join("share")
        .join("nautilus-python")
        .join("extensions"))
}

fn home_dir() -> io::Result<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))
}

fn ensure_nautilus_python_available() -> io::Result<()> {
    run_python_check("python3 is required", "print('ok')")?;
    run_python_check(
        "PyGObject is required: python3 cannot import gi",
        "import gi",
    )?;
    run_python_check(
        "Nautilus 4.0 or 3.0 introspection bindings are required",
        "import gi\nfor version in ('4.0', '3.0'):\n    try:\n        gi.require_version('Nautilus', version)\n        break\n    except ValueError:\n        pass\nelse:\n    raise ValueError('Namespace Nautilus not available for version 4.0 or 3.0')",
    )?;
    run_python_check(
        "nautilus-python is required with Nautilus 4.0 or 3.0 bindings. Install your distribution's nautilus-python package, then rerun this command.",
        "import gi\nfor version in ('4.0', '3.0'):\n    try:\n        gi.require_version('Nautilus', version)\n        from gi.repository import Nautilus\n        break\n    except ValueError:\n        pass\nelse:\n    raise ValueError('Namespace Nautilus not available for version 4.0 or 3.0')",
    )
}

fn run_python_check(error: &str, script: &str) -> io::Result<()> {
    let status = Command::new("python3").arg("-c").arg(script).status();

    if matches!(status, Ok(status) if status.success()) {
        return Ok(());
    }

    Err(io::Error::new(io::ErrorKind::NotFound, error))
}

struct GuiCheck {
    configured: String,
    resolved: Option<PathBuf>,
    skipped: bool,
}

fn check_gui_exe(gui_exe: &str, skip_gui_check: bool) -> io::Result<GuiCheck> {
    if gui_exe.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--gui-exe must not be empty",
        ));
    }

    if skip_gui_check {
        return Ok(GuiCheck {
            configured: gui_exe.to_string(),
            resolved: None,
            skipped: true,
        });
    }

    let resolved = resolve_gui_exe(gui_exe)?;
    Ok(GuiCheck {
        configured: gui_exe.to_string(),
        resolved: Some(resolved),
        skipped: false,
    })
}

fn print_gui_check(check: &GuiCheck) {
    println!("Checking GUI executable...");
    println!("  configured: {}", check.configured);
    if check.skipped {
        println!("  resolved: skipped by --skip-gui-check");
    } else if let Some(resolved) = &check.resolved {
        println!("  resolved: {}", resolved.display());
    }
}

fn resolve_gui_exe(gui_exe: &str) -> io::Result<PathBuf> {
    if looks_like_path(gui_exe) {
        let path = PathBuf::from(gui_exe);
        return validate_executable_file(&path, gui_exe);
    }

    find_executable_on_path(gui_exe).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("could not resolve GUI executable `{gui_exe}` on PATH"),
        )
    })
}

fn looks_like_path(value: &str) -> bool {
    let path = Path::new(value);
    path.is_absolute() || value.contains('/') || value.contains('\\')
}

fn validate_executable_file(path: &Path, display: &str) -> io::Result<PathBuf> {
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("GUI executable does not exist: {display}"),
        ));
    }

    if !path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("GUI executable is not a file: {display}"),
        ));
    }

    if !is_executable_file(path) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("GUI executable is not executable: {display}"),
        ));
    }

    path.canonicalize()
}

fn find_executable_on_path(command: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        for candidate_name in executable_candidate_names(command) {
            let candidate = dir.join(candidate_name);
            if is_executable_file(&candidate) {
                return candidate.canonicalize().ok().or(Some(candidate));
            }
        }
    }
    None
}

fn executable_candidate_names(command: &str) -> Vec<OsString> {
    let command_path = Path::new(command);
    if command_path.extension().is_some() {
        return vec![OsString::from(command)];
    }

    #[cfg(windows)]
    {
        let mut candidates = vec![OsString::from(command)];
        let extensions = env::var_os("PATHEXT").unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into());
        for extension in extensions.to_string_lossy().split(';') {
            if extension.is_empty() {
                continue;
            }
            candidates.push(OsString::from(format!("{command}{extension}")));
        }
        candidates
    }

    #[cfg(not(windows))]
    {
        vec![OsString::from(command)]
    }
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        path.metadata()
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn verify_extension_contents(contents: &str, gui_exe: &str) -> io::Result<()> {
    if !contents.contains("class LinkForgeMenuProvider") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "extension does not contain LinkForgeMenuProvider",
        ));
    }

    if !contents.contains("Nautilus.MenuProvider") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "extension does not contain Nautilus.MenuProvider",
        ));
    }

    for marker in [
        "def _pick_sources(paths):",
        "if _pick_sources(paths):",
        "def _file_is_directory(file_info, path):",
    ] {
        if !contents.contains(marker) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("extension does not contain required marker {marker:?}"),
            ));
        }
    }

    let configured_gui = python_string(gui_exe);
    if !contents.contains(&configured_gui) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("extension does not contain configured GUI executable {configured_gui}"),
        ));
    }

    Ok(())
}

fn extension_contents(gui_exe: &str) -> String {
    PYTHON_EXTENSION.replace("__LINKFORGE_GUI_EXE__", &python_string(gui_exe))
}

fn python_string(value: &str) -> String {
    format!("{value:?}")
}

const PYTHON_EXTENSION: &str = r#"# Generated by linkforge-context-menu-gnome.
import json
import os
import subprocess

import gi


def _require_nautilus():
    last_error = None
    for version in ("4.0", "3.0"):
        try:
            gi.require_version("Nautilus", version)
            return
        except ValueError as error:
            last_error = error
    raise last_error


_require_nautilus()
from gi.repository import GObject, Nautilus


GUI_EXE = os.environ.get("LINKFORGE_GUI", __LINKFORGE_GUI_EXE__)


def _picked_sources_state_path():
    return os.path.join(_picked_source_state_dir(), "picked-sources.json")


def _picked_source_state_dir():
    state_home = os.environ.get("XDG_STATE_HOME")
    if not state_home:
        home = os.path.expanduser("~")
        state_home = os.path.join(home, ".local", "state")
    return os.path.join(state_home, "LinkForge")


def _existing_paths(paths):
    return [path for path in paths if path and os.path.exists(path)]


def _picked_sources():
    try:
        with open(_picked_sources_state_path(), "r", encoding="utf-8") as handle:
            paths = json.load(handle)
        if isinstance(paths, list):
            return _existing_paths([path for path in paths if isinstance(path, str)])
    except (OSError, ValueError):
        return []


def _pick_sources(paths):
    if not paths or len(_existing_paths(paths)) != len(paths):
        return False
    state_path = _picked_sources_state_path()
    os.makedirs(os.path.dirname(state_path), exist_ok=True)
    temporary_path = f"{state_path}.tmp"
    with open(temporary_path, "w", encoding="utf-8") as handle:
        json.dump(paths, handle)
    os.replace(temporary_path, state_path)
    return True


def _picked_source_label(kind, picked):
    if not picked:
        return f"Create {kind} from Picked Source"
    if len(picked) > 1:
        suffix = "Symlinks" if kind == "Symlink" else "Hard Links"
        return f"Create {suffix} from {len(picked)} Sources"
    name = os.path.basename(picked[0])
    if kind == "Hard Link" and os.path.isdir(picked[0]):
        return f"Create Hard-Link Tree from {name}"
    return f"Create {kind} from {name}"


def _file_path(file_info):
    location = file_info.get_location()
    return location.get_path() if location else None


def _file_is_directory(file_info, path):
    try:
        if file_info.is_directory():
            return True
    except (AttributeError, TypeError):
        pass
    return bool(path and os.path.isdir(path))


def _first_name(paths):
    return os.path.basename(paths[0]) if paths else None


def _run_action(action, paths, background=False):
    command = [GUI_EXE, "--context-action", action]
    if background:
        command.append("--context-background")
    command.append("--paths")
    command.extend(paths)
    subprocess.Popen(
        command,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )


class LinkForgeMenuProvider(GObject.GObject, Nautilus.MenuProvider):
    def get_file_items(self, *args):
        files = args[-1]
        paths = [_file_path(file_info) for file_info in files]
        paths = [path for path in paths if path]
        if not paths:
            return []

        first = files[0]
        first_is_dir = _file_is_directory(first, paths[0])
        picked = _picked_sources()
        items = []

        if len(paths) >= 1:
            name = _first_name(paths)
            if len(paths) > 1:
                label = f"Pick {len(paths)} Link Sources"
            else:
                label = f"Pick Link Source: {name}" if name else "Pick Link Source"
            items.append(self._action_item("pick-source", label, "pick-source", paths))

        if len(paths) == 1 and first_is_dir and picked:
            items.append(self._action_item("drop-symlink", _picked_source_label("Symlink", picked), "drop-symlink", [paths[0]]))
            items.append(self._action_item("drop-hardlink", _picked_source_label("Hard Link", picked), "drop-hardlink", [paths[0]]))

        if len(files) == 2 and len(paths) == 2 and all(not _file_is_directory(file_info, path) for file_info, path in zip(files, paths)):
            items.append(self._action_item("same-file", "Compare Same File", "same-file", paths))

        if len(paths) == 1:
            items.append(self._action_item("symlink", "Open Symlink in LinkForge...", "symlink", [paths[0]]))
            if not first_is_dir:
                items.append(self._action_item("hardlink", "Open Hard Link in LinkForge...", "hardlink", [paths[0]]))
                items.append(self._action_item("link-count", "Show Link Count", "link-count", [paths[0]]))
            items.append(self._action_item("siblings", "Find Hard Link Siblings...", "siblings", [paths[0]]))
            if first_is_dir:
                items.append(self._action_item("scan-groups", "Scan Hard Link Groups", "scan-groups", [paths[0]]))
                items.append(self._action_item("clone-tree", "Clone Tree Preserving Hard Links...", "clone-tree", [paths[0]]))

        return [self._root_item(items)] if items else []

    def get_background_items(self, *args):
        current_folder = args[-1]
        path = _file_path(current_folder)
        picked = _picked_sources()
        if not path or not picked:
            return []

        items = [
            self._action_item("drop-symlink-background", _picked_source_label("Symlink", picked), "drop-symlink", [path], True)
        ]
        items.append(self._action_item("drop-hardlink-background", _picked_source_label("Hard Link", picked), "drop-hardlink", [path], True))
        return [self._root_item(items)]

    def _root_item(self, items):
        root = Nautilus.MenuItem(
            name="LinkForge::Root",
            label="LinkForge",
            tip="Open LinkForge actions",
        )
        submenu = Nautilus.Menu()
        for item in items:
            submenu.append_item(item)
        root.set_submenu(submenu)
        return root

    def _action_item(self, name, label, action, paths, background=False):
        item = Nautilus.MenuItem(
            name=f"LinkForge::{name}",
            label=label,
            tip="Open LinkForge",
        )
        item.connect("activate", self._activate, action, paths, background)
        return item

    def _activate(self, _menu, action, paths, background):
        if action == "pick-source":
            try:
                if _pick_sources(paths):
                    return
            except OSError:
                pass
        _run_action(action, paths, background)
"#;

fn print_help() {
    println!("Usage:");
    println!(
        "  linkforge-context-menu-gnome install [--gui-exe <path-or-command>] [--skip-gui-check]"
    );
    println!(
        "  linkforge-context-menu-gnome verify [--gui-exe <path-or-command>] [--skip-gui-check]"
    );
    println!("  linkforge-context-menu-gnome uninstall");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_path_uses_nautilus_python_extensions() {
        let path = nautilus_python_extension_dir().unwrap();
        assert!(path.ends_with(".local/share/nautilus-python/extensions"));
    }

    #[test]
    fn generated_extension_uses_menu_provider_and_gui_exe() {
        let extension = extension_contents("/opt/linkforge/linkforge-gui");
        assert!(extension.contains("class LinkForgeMenuProvider"));
        assert!(extension.contains("Nautilus.MenuProvider"));
        assert!(extension.contains("for version in (\"4.0\", \"3.0\")"));
        assert!(extension.contains("/opt/linkforge/linkforge-gui"));
        assert!(extension.contains("--context-action"));
        assert!(extension.contains("picked-sources.json"));
        assert!(!extension.contains("picked-source.txt"));
        assert!(extension.contains("def _pick_sources(paths):"));
        assert!(extension.contains("json.dump(paths, handle)"));
        assert!(extension.contains("os.replace(temporary_path, state_path)"));
        assert!(extension.contains("return True"));
        assert!(extension.contains("if action == \"pick-source\":"));
        assert!(extension.contains("if _pick_sources(paths):"));
        assert!(extension.contains("def _file_is_directory(file_info, path):"));
        assert!(extension.contains("os.path.isdir(path)"));
        assert!(extension.contains("first_is_dir = _file_is_directory(first, paths[0])"));
        assert!(extension.contains("Pick {len(paths)} Link Sources"));
        assert!(extension.contains("Create Hard-Link Tree"));
        assert!(extension.contains("same-file"));
        assert!(extension.contains("Compare Same File"));
        assert!(extension.contains("drop-symlink"));
        assert!(extension.contains("--context-background"));
    }

    #[test]
    fn generated_extension_does_not_use_nautilus_scripts() {
        let extension = extension_contents("linkforge-gui");
        assert!(!extension.contains("NAUTILUS_SCRIPT_SELECTED_FILE_PATHS"));
        assert!(!extension.contains(".local/share/nautilus/scripts"));
    }

    #[test]
    fn install_options_parse_gui_exe_and_skip_check() {
        let command = parse_args([
            "install".to_string(),
            "--gui-exe".to_string(),
            "/opt/linkforge/linkforge-gui".to_string(),
            "--skip-gui-check".to_string(),
        ])
        .unwrap();

        match command {
            CliCommand::Install {
                gui_exe,
                skip_gui_check,
            } => {
                assert_eq!(gui_exe, "/opt/linkforge/linkforge-gui");
                assert!(skip_gui_check);
            }
            _ => panic!("expected install command"),
        }
    }

    #[test]
    fn verify_options_default_to_linkforge_gui() {
        let command = parse_args(["verify".to_string()]).unwrap();

        match command {
            CliCommand::Verify {
                gui_exe,
                skip_gui_check,
            } => {
                assert_eq!(gui_exe, DEFAULT_GUI_EXE);
                assert!(!skip_gui_check);
            }
            _ => panic!("expected verify command"),
        }
    }

    #[test]
    fn skip_gui_check_does_not_resolve_gui() {
        let check = check_gui_exe("definitely-not-installed-linkforge-gui", true).unwrap();
        assert_eq!(check.configured, "definitely-not-installed-linkforge-gui");
        assert!(check.skipped);
        assert!(check.resolved.is_none());
    }

    #[test]
    fn path_like_gui_exe_must_exist() {
        let error = resolve_gui_exe("/definitely/missing/linkforge-gui").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn command_gui_exe_must_be_on_path() {
        let error = resolve_gui_exe("definitely-not-installed-linkforge-gui").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn verify_extension_requires_markers_and_gui_exe() {
        let extension = extension_contents("/opt/linkforge/linkforge-gui");
        verify_extension_contents(&extension, "/opt/linkforge/linkforge-gui").unwrap();

        let error = verify_extension_contents(&extension, "/other/linkforge-gui").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);

        let old_extension = extension.replace("def _pick_sources(paths):", "def _old_pick(paths):");
        let error =
            verify_extension_contents(&old_extension, "/opt/linkforge/linkforge-gui").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
