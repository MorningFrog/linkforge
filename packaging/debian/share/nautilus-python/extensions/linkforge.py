# Packaged LinkForge GNOME Files extension.
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


GUI_EXE = os.environ.get("LINKFORGE_GUI", "/usr/bin/linkforge-gui")


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
