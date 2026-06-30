const tauriApi = window.__TAURI__ || {};
const invoke = tauriApi.core?.invoke;
const dialogOpen = tauriApi.dialog?.open;
const dialogSave = tauriApi.dialog?.save;

const state = {
  mode: "symlink",
  results: [],
  contextPath: null,
  launch: null,
};

const titles = {
  quick: "Quick Action",
  inspect: "Inspector",
  groups: "Hard Link Groups",
  clone: "Clone Tree",
  about: "About",
};

const quickModes = {
  symlink: {
    command: "create_symlink",
    needsLink: true,
    needsForce: true,
    successType: "message",
  },
  hardlink: {
    command: "create_hardlink",
    needsLink: true,
    needsForce: true,
    successType: "message",
  },
  "link-count": {
    command: "link_count",
    needsLink: false,
    needsForce: false,
    successType: "link-count",
  },
  siblings: {
    command: "siblings",
    needsLink: false,
    needsForce: false,
    needsRoot: true,
    successType: "paths",
  },
};

function requireTauri() {
  if (!invoke) {
    setStatus("This screen must run inside the LinkForge Tauri app.", "error");
    return false;
  }
  return true;
}

function byId(id) {
  return document.getElementById(id);
}

function inputValue(id) {
  return byId(id).value.trim();
}

function setInput(id, value) {
  byId(id).value = value || "";
}

function setStatus(message, type = "idle") {
  const status = byId("status");
  status.textContent = message;
  status.className = `status ${type}`;
}

function setResults(items) {
  state.results = items;
  const list = byId("results-list");
  list.innerHTML = "";

  if (!items.length) {
    const empty = document.createElement("div");
    empty.className = "empty";
    empty.textContent = "No results.";
    list.append(empty);
    return;
  }

  items.forEach((item) => {
    if (item.type === "group") {
      const title = document.createElement("div");
      title.className = "group-title";
      title.textContent = item.label;
      list.append(title);
      return;
    }

    const row = document.createElement("div");
    row.className = "result-item";
    row.textContent = item.text;
    row.dataset.path = item.path || "";
    row.addEventListener("contextmenu", showPathMenu);
    list.append(row);
  });
}

function resultMessage(message) {
  setResults([{ text: message }]);
}

function resultPaths(paths) {
  setResults(paths.map((path) => ({ text: path, path })));
}

function resultGroups(groups) {
  const items = [];
  groups.forEach((group, index) => {
    items.push({ type: "group", label: `Group ${index + 1}` });
    group.paths.forEach((path) => items.push({ text: path, path }));
  });
  setResults(items);
}

function displayError(error) {
  const message = error?.message || String(error);
  setStatus(message, "error");
  resultMessage(message);
}

async function runCommand(name, args, onSuccess) {
  if (!requireTauri()) return;
  setStatus("Running...", "idle");
  try {
    const result = await invoke(name, args);
    onSuccess(result);
    setStatus("Done.", "ok");
  } catch (error) {
    displayError(error);
  }
}

function applyQuickMode(mode) {
  state.mode = mode;
  document.querySelectorAll(".mode").forEach((button) => {
    button.classList.toggle("active", button.dataset.mode === mode);
  });

  const config = quickModes[mode];
  document.querySelectorAll(".quick-link-only").forEach((el) => {
    el.classList.toggle("hidden", !config.needsLink);
  });
  document.querySelectorAll(".quick-force-only").forEach((el) => {
    el.classList.toggle("hidden", !config.needsForce);
  });
  document.querySelectorAll(".quick-siblings-only").forEach((el) => {
    el.classList.toggle("hidden", !config.needsRoot);
  });
}

async function choosePath(target, options) {
  if (!dialogOpen && !dialogSave) {
    return;
  }

  const value = options.save
    ? await dialogSave({ title: "Choose path" })
    : await dialogOpen({
        title: "Choose path",
        directory: Boolean(options.directory),
        multiple: false,
      });

  if (typeof value === "string") {
    setInput(target, value);
  }
}

function switchView(view) {
  document.querySelectorAll(".tab").forEach((button) => {
    button.classList.toggle("active", button.dataset.view === view);
  });
  document.querySelectorAll(".view").forEach((section) => {
    section.classList.toggle("active", section.id === `view-${view}`);
  });
  byId("view-title").textContent = titles[view] || "LinkForge";
}

function requireField(id, label) {
  const value = inputValue(id);
  if (!value) {
    throw new Error(`${label} is required.`);
  }
  return value;
}

async function handleQuickSubmit(event) {
  event.preventDefault();
  const config = quickModes[state.mode];
  try {
    const source = requireField("quick-source", "Source");
    const args = { source };

    if (config.needsLink) {
      args.link = requireField("quick-link", "Link path");
      args.force = byId("quick-force").checked;
    }
    if (config.needsRoot) {
      args.path = source;
      args.root = inputValue("quick-root") || null;
      delete args.source;
    }
    if (state.mode === "link-count") {
      args.path = source;
      delete args.source;
    }

    await runCommand(config.command, args, (result) => {
      if (config.successType === "paths") {
        resultPaths(result.paths);
      } else {
        resultMessage(result.message);
      }
    });
  } catch (error) {
    displayError(error);
  }
}

function showPathMenu(event) {
  const path = event.currentTarget.dataset.path;
  if (!path) return;
  event.preventDefault();
  state.contextPath = path;
  const menu = byId("path-menu");
  menu.style.left = `${event.clientX}px`;
  menu.style.top = `${event.clientY}px`;
  menu.classList.remove("hidden");
}

function hidePathMenu() {
  byId("path-menu").classList.add("hidden");
}

async function handleMenuAction(action) {
  const path = state.contextPath;
  hidePathMenu();
  if (!path) return;

  if (action === "copy") {
    await navigator.clipboard.writeText(path);
    setStatus("Path copied.", "ok");
    return;
  }
  if (action === "reveal") {
    await runCommand("reveal_path", { path }, (result) => resultMessage(result.message));
    return;
  }
  if (action === "link-count") {
    switchView("inspect");
    setInput("inspect-path", path);
    await runCommand("link_count", { path }, (result) => resultMessage(result.message));
    return;
  }
  if (action === "siblings") {
    switchView("inspect");
    setInput("inspect-path", path);
    await runCommand(
      "siblings",
      { path, root: inputValue("inspect-root") || null },
      (result) => resultPaths(result.paths),
    );
  }
}

async function loadInitialContext() {
  if (!requireTauri()) return;
  try {
    const context = await invoke("initial_context");
    state.launch = context;
    byId("platform-label").textContent = context.platform;
    const label = context.action
      ? `${context.action}: ${context.paths.join(", ")}`
      : "Ready";
    byId("context-label").textContent = label;

    if (context.paths?.length) {
      setInput("quick-source", context.paths[0]);
      setInput("inspect-path", context.paths[0]);
      setInput("groups-root", context.paths[0]);
      setInput("clone-source", context.paths[0]);
    }

    const actionMap = {
      symlink: "symlink",
      hardlink: "hardlink",
      "link-count": "link-count",
      siblings: "siblings",
      "scan-groups": "groups",
      "clone-tree": "clone",
    };

    const mapped = actionMap[context.action];
    if (mapped === "groups" || mapped === "clone") {
      switchView(mapped);
    } else if (mapped) {
      applyQuickMode(mapped);
      switchView("quick");
    }
  } catch (error) {
    displayError(error);
  }
}

function bindEvents() {
  document.querySelectorAll(".tab").forEach((button) => {
    button.addEventListener("click", () => switchView(button.dataset.view));
  });
  document.querySelectorAll(".mode").forEach((button) => {
    button.addEventListener("click", () => applyQuickMode(button.dataset.mode));
  });
  document.querySelectorAll(".browse-file").forEach((button) => {
    button.addEventListener("click", () => choosePath(button.dataset.target, { directory: false }));
  });
  document.querySelectorAll(".browse-dir").forEach((button) => {
    button.addEventListener("click", () => choosePath(button.dataset.target, { directory: true }));
  });
  document.querySelectorAll(".save-path").forEach((button) => {
    button.addEventListener("click", () => choosePath(button.dataset.target, { save: true }));
  });

  byId("quick-form").addEventListener("submit", handleQuickSubmit);
  byId("same-file-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    try {
      await runCommand(
        "same_file",
        {
          pathA: requireField("same-a", "Path A"),
          pathB: requireField("same-b", "Path B"),
        },
        (result) => resultMessage(result.message),
      );
    } catch (error) {
      displayError(error);
    }
  });

  byId("inspect-link-count").addEventListener("click", async () => {
    try {
      const path = requireField("inspect-path", "Path");
      await runCommand("link_count", { path }, (result) => resultMessage(result.message));
    } catch (error) {
      displayError(error);
    }
  });
  byId("inspect-siblings").addEventListener("click", async () => {
    try {
      const path = requireField("inspect-path", "Path");
      await runCommand(
        "siblings",
        { path, root: inputValue("inspect-root") || null },
        (result) => resultPaths(result.paths),
      );
    } catch (error) {
      displayError(error);
    }
  });

  byId("groups-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    try {
      const root = requireField("groups-root", "Root");
      await runCommand("scan_groups", { root }, (result) => resultGroups(result.groups));
    } catch (error) {
      displayError(error);
    }
  });

  byId("clone-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    try {
      await runCommand(
        "clone_tree",
        {
          sourceDir: requireField("clone-source", "Source directory"),
          destDir: requireField("clone-dest", "Destination directory"),
          force: byId("clone-force").checked,
        },
        (result) => resultMessage(result.message),
      );
    } catch (error) {
      displayError(error);
    }
  });

  byId("copy-results").addEventListener("click", async () => {
    await navigator.clipboard.writeText(
      state.results
        .filter((item) => item.text)
        .map((item) => item.text)
        .join("\n"),
    );
    setStatus("Results copied.", "ok");
  });

  byId("clear-status").addEventListener("click", () => {
    setStatus("No recent operation.");
    setResults([]);
  });

  byId("path-menu").addEventListener("click", (event) => {
    const action = event.target.dataset.menuAction;
    if (action) {
      handleMenuAction(action);
    }
  });

  window.addEventListener("click", hidePathMenu);
  window.addEventListener("blur", hidePathMenu);
}

bindEvents();
applyQuickMode("symlink");
setResults([]);
loadInitialContext();
