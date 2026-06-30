use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const SCRIPT_DIR_NAME: &str = "LinkForge";

const SCRIPTS: &[(&str, &str, &str)] = &[
    ("linkforge-symlink.sh", "symlink", "Create Symbolic Link"),
    ("linkforge-hardlink.sh", "hardlink", "Create Hard Link"),
    ("linkforge-link-count.sh", "link-count", "Show Link Count"),
    (
        "linkforge-siblings.sh",
        "siblings",
        "Find Hard Link Siblings",
    ),
    (
        "linkforge-scan-groups.sh",
        "scan-groups",
        "Scan Hard Link Groups",
    ),
    (
        "linkforge-clone-tree.sh",
        "clone-tree",
        "Clone Tree Preserving Hard Links",
    ),
];

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_string());

    match command.as_str() {
        "install" => {
            let mut gui_exe = "linkforge-gui".to_string();
            while let Some(arg) = args.next() {
                if arg == "--gui-exe" {
                    gui_exe = args.next().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "--gui-exe requires a value")
                    })?;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("unknown argument: {arg}"),
                    ));
                }
            }
            install(&gui_exe)
        }
        "uninstall" => uninstall(),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown command: {other}"),
        )),
    }
}

fn install(gui_exe: &str) -> io::Result<()> {
    let target_dir = nautilus_script_dir()?;
    fs::create_dir_all(&target_dir)?;

    for (file_name, action, _label) in SCRIPTS {
        let path = target_dir.join(file_name);
        fs::write(&path, script_contents(gui_exe, action))?;
        make_executable(&path)?;
    }

    println!(
        "Installed LinkForge GNOME Files scripts to {}",
        target_dir.display()
    );
    Ok(())
}

fn uninstall() -> io::Result<()> {
    let target_dir = nautilus_script_dir()?;
    if !target_dir.exists() {
        println!("No LinkForge GNOME Files scripts found.");
        return Ok(());
    }

    for (file_name, _action, _label) in SCRIPTS {
        let path = target_dir.join(file_name);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }

    if target_dir.read_dir()?.next().is_none() {
        fs::remove_dir(&target_dir)?;
    }

    println!(
        "Removed LinkForge GNOME Files scripts from {}",
        target_dir.display()
    );
    Ok(())
}

fn nautilus_script_dir() -> io::Result<PathBuf> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
    Ok(home
        .join(".local")
        .join("share")
        .join("nautilus")
        .join("scripts")
        .join(SCRIPT_DIR_NAME))
}

fn script_contents(gui_exe: &str, action: &str) -> String {
    format!(
        r#"#!/usr/bin/env sh
set -eu

EXE=${{LINKFORGE_GUI:-{gui_exe}}}
first_path=$(printf '%s\n' "${{NAUTILUS_SCRIPT_SELECTED_FILE_PATHS:-}}" | sed '/^$/d' | head -n 1)

if [ -n "$first_path" ]; then
  "$EXE" --context-action {action} --paths "$first_path" >/dev/null 2>&1 &
fi
"#
    )
}

#[cfg(unix)]
fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn print_help() {
    println!("Usage:");
    println!("  linkforge-context-menu-gnome install [--gui-exe <path-or-command>]");
    println!("  linkforge-context-menu-gnome uninstall");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_script_uses_action_and_gui_exe() {
        let script = script_contents("/opt/linkforge/linkforge-gui", "link-count");
        assert!(script.contains("/opt/linkforge/linkforge-gui"));
        assert!(script.contains("--context-action link-count"));
    }
}
