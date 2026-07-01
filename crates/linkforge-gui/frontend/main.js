const tauriApi = window.__TAURI__ || {};
const invoke = tauriApi.core?.invoke;
const dialog = tauriApi.dialog || {};

const state = {
  mode: "symlink",
  results: [],
  contextPath: null,
  launch: null,
  modalResolver: null,
  lightweightMode: false,
  fullWindowOpen: false,
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

function clearElement(element) {
  while (element.firstChild) {
    element.removeChild(element.firstChild);
  }
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

function isDropAction(action) {
  return action === "drop-symlink" || action === "drop-hardlink";
}

function isLightweightAction(action) {
  return (
    action === "drop-symlink" ||
    action === "drop-hardlink" ||
    action === "pick-source" ||
    action === "same-file" ||
    action === "link-count"
  );
}

function enterLightweightMode() {
  state.lightweightMode = true;
  state.fullWindowOpen = false;
  document.body.classList.add("drop-mode");
}

async function showLightweightWindow() {
  if (!state.lightweightMode || state.fullWindowOpen || !invoke) return;
  await invoke("show_drop_window");
}

async function closeLightweightWindow() {
  if (!invoke) return;
  try {
    await invoke("close_drop_window");
  } catch (_) {
    // The command can be interrupted by the window closing, which is the desired outcome.
  }
}

async function expandToFullWindow() {
  if (!state.lightweightMode || state.fullWindowOpen || !invoke) return;

  await invoke("expand_to_full_window");
  state.fullWindowOpen = true;
  state.lightweightMode = false;
  document.body.classList.remove("drop-mode");
  switchView("quick");

  const openFull = byId("modal-open-full");
  openFull.classList.add("hidden");
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

function messageBlock(message) {
  const block = document.createElement("div");
  block.className = "modal-message";
  block.textContent = message;
  return block;
}

function closeModal(value) {
  const resolver = state.modalResolver;
  if (!resolver) return;

  const backdrop = byId("modal-backdrop");
  const check = byId("modal-check");
  backdrop.classList.add("hidden");
  state.modalResolver = null;
  resolver({ value, checked: check.checked });
}

function openModal({
  title,
  content,
  actions,
  checkLabel = null,
  defaultValue = "cancel",
  tone = "",
}) {
  if (state.modalResolver) {
    closeModal(defaultValue);
  }

  const backdrop = byId("modal-backdrop");
  const modal = byId("modal-panel");
  const titleEl = byId("modal-title");
  const body = byId("modal-body");
  const actionsEl = byId("modal-actions");
  const checkRow = byId("modal-check-row");
  const check = byId("modal-check");
  const checkText = byId("modal-check-label");
  const openFull = byId("modal-open-full");

  titleEl.textContent = title;
  modal.className = `modal ${tone}`.trim();
  clearElement(body);
  clearElement(actionsEl);

  const nodes = Array.isArray(content) ? content : [content];
  nodes.forEach((node) => {
    if (typeof node === "string") {
      body.append(messageBlock(node));
    } else if (node) {
      body.append(node);
    }
  });

  check.checked = false;
  checkText.textContent = checkLabel || "";
  checkRow.classList.toggle("hidden", !checkLabel);
  openFull.classList.toggle("hidden", !(state.lightweightMode && !state.fullWindowOpen));
  openFull.onclick = async () => {
    try {
      await expandToFullWindow();
    } catch (error) {
      displayError(error);
    }
  };

  actions.forEach((action) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = action.className || "secondary-button";
    button.textContent = action.label;
    button.addEventListener("click", () => closeModal(action.value));
    actionsEl.append(button);
  });

  backdrop.classList.remove("hidden");

  return new Promise((resolve) => {
    state.modalResolver = resolve;
  });
}

async function showMessageModal(title, message, tone = "") {
  return openModal({
    title,
    content: messageBlock(message),
    tone,
    defaultValue: "close",
    actions: [{ label: "Close", value: "close", className: "primary-button" }],
  });
}

async function showConflictModal(path, allowApplyToRemaining) {
  const wrapper = document.createElement("div");
  wrapper.className = "conflict-body";

  const text = document.createElement("p");
  text.textContent = "The target already exists.";
  wrapper.append(text);

  const code = document.createElement("code");
  code.className = "path-code";
  code.textContent = path;
  wrapper.append(code);

  const result = await openModal({
    title: "Resolve Conflict",
    content: wrapper,
    checkLabel: allowApplyToRemaining ? "Apply this choice to remaining conflicts" : null,
    defaultValue: "cancel",
    actions: [
      { label: "Rename", value: "rename", className: "primary-button" },
      { label: "Overwrite", value: "overwrite", className: "secondary-button" },
      { label: "Skip", value: "skip", className: "secondary-button" },
      { label: "Cancel", value: "cancel", className: "ghost-button" },
    ],
  });

  return {
    choice: result.value || "cancel",
    applyToRemaining: Boolean(result.checked && result.value !== "cancel"),
  };
}

function hasPreflightFindings(preflight) {
  return Boolean(
    preflight?.problems?.length ||
      preflight?.conflicts?.length ||
      preflight?.warnings?.length,
  );
}

function appendPreflightSection(wrapper, title, items, className, renderItem) {
  if (!items?.length) return;

  const section = document.createElement("section");
  section.className = `preflight-section ${className}`;

  const heading = document.createElement("h4");
  heading.textContent = title;
  section.append(heading);

  const list = document.createElement("ul");
  items.forEach((item) => {
    const row = document.createElement("li");
    renderItem(row, item);
    list.append(row);
  });
  section.append(list);
  wrapper.append(section);
}

function appendPreflightPath(row, path) {
  if (!path) return;
  const code = document.createElement("code");
  code.className = "path-code";
  code.textContent = path;
  row.append(code);
}

async function showPreflightModal(preflight) {
  const wrapper = document.createElement("div");
  wrapper.className = "preflight-body";

  appendPreflightSection(
    wrapper,
    "Problems",
    preflight.problems,
    "problem",
    (row, item) => {
      const text = document.createElement("p");
      text.textContent = item.message;
      row.append(text);
      appendPreflightPath(row, item.source);
    },
  );

  appendPreflightSection(
    wrapper,
    "Conflicts",
    preflight.conflicts,
    "conflict",
    (row, item) => {
      const text = document.createElement("p");
      text.textContent = item.message;
      row.append(text);
      appendPreflightPath(row, item.link);
    },
  );

  appendPreflightSection(
    wrapper,
    "Warnings",
    preflight.warnings,
    "warning",
    (row, item) => {
      const text = document.createElement("p");
      text.textContent = item.message;
      row.append(text);
      appendPreflightPath(row, item.source);
    },
  );

  const hasProblems = Boolean(preflight.problems?.length);
  const hasConflicts = Boolean(preflight.conflicts?.length);
  let actions;
  if (hasProblems) {
    actions = [{ label: "Cancel", value: "cancel", className: "primary-button" }];
  } else if (hasConflicts) {
    actions = [
      { label: "Overwrite Existing", value: "overwrite-conflicts", className: "primary-button" },
      { label: "Rename / Review Each", value: "continue", className: "secondary-button" },
      { label: "Skip Existing", value: "skip-conflicts", className: "secondary-button" },
      { label: "Cancel", value: "cancel", className: "ghost-button" },
    ];
  } else {
    actions = [
      { label: "Continue", value: "continue", className: "primary-button" },
      { label: "Cancel", value: "cancel", className: "ghost-button" },
    ];
  }

  const result = await openModal({
    title: hasProblems ? "Preflight Failed" : "Review Batch",
    content: wrapper,
    defaultValue: "cancel",
    tone: hasProblems ? "error" : "",
    actions,
  });

  return result.value || "cancel";
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

async function runSameFileComparison(pathA, pathB) {
  await runCommand("same_file", { pathA, pathB }, (result) => resultMessage(result.message));
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
  if (!requireTauri()) return;

  try {
    const currentValue = inputValue(target);
    const value = options.save
      ? await dialog.save({ defaultPath: currentValue || undefined })
      : await dialog.open({
          directory: Boolean(options.directory),
          multiple: false,
          defaultPath: currentValue || undefined,
        });
    if (typeof value === "string" && value) {
      setInput(target, value);
    }
  } catch (error) {
    displayError(error);
  }
}

function directKindFromAction(action) {
  return action === "drop-hardlink" ? "hardlink" : "symlink";
}

function directKindNoun(kind) {
  return kind === "hardlink" ? "hard link" : "symlink";
}

function createDirectSummary() {
  return {
    created: 0,
    renamed: 0,
    skipped: 0,
    failed: 0,
    cancelled: 0,
    skippedDetails: [],
    failedDetails: [],
  };
}

function recordDirectStep(summary, result) {
  if (result.status === "created") {
    summary.created += 1;
    return;
  }
  if (result.status === "renamed") {
    summary.created += 1;
    summary.renamed += 1;
    return;
  }
  if (result.status === "skipped") {
    summary.skipped += 1;
    summary.skippedDetails.push(result.message);
    return;
  }
  if (result.status === "failed") {
    summary.failed += 1;
    summary.failedDetails.push(result.message);
    return;
  }
  if (result.status === "cancelled") {
    summary.cancelled += 1;
  }
}

function directSummaryMessage(summary, kind) {
  const lines = [
    `Batch ${directKindNoun(kind)} operation complete.`,
    `Created: ${summary.created}`,
    `Renamed: ${summary.renamed}`,
    `Skipped: ${summary.skipped}`,
    `Failed: ${summary.failed}`,
    `Cancelled: ${summary.cancelled}`,
  ];

  if (summary.skippedDetails.length) {
    lines.push("Skipped items:");
    lines.push(...summary.skippedDetails);
  }
  if (summary.failedDetails.length) {
    lines.push("Failed items:");
    lines.push(...summary.failedDetails);
  }

  return lines.join("\n");
}

function isCleanDirectSummary(summary) {
  return (
    summary.created > 0 &&
    summary.renamed === 0 &&
    summary.skipped === 0 &&
    summary.failed === 0 &&
    summary.cancelled === 0
  );
}

async function runDirectDrop(context) {
  if (!requireTauri()) return;

  const kind = directKindFromAction(context.action);
  const summary = createDirectSummary();
  let appliedChoice = null;
  let overwriteSources = new Set();

  setResults([]);
  setStatus("Preparing batch link operation...", "idle");

  try {
    const drop = await invoke("prepare_direct_drop", {
      targets: context.paths || [],
      backgroundTarget: Boolean(context.backgroundTarget),
      kind,
    });

    byId("context-label").textContent =
      `${context.action}: ${drop.sources.length} source(s) -> ${drop.targetDir}`;

    let sources = drop.sources;
    if (hasPreflightFindings(drop.preflight)) {
      await showLightweightWindow();
      const preflightChoice = await showPreflightModal(drop.preflight);
      if (preflightChoice === "cancel" || drop.preflight.problems?.length) {
        summary.cancelled = drop.sources.length;
        const message = directSummaryMessage(summary, kind);
        resultMessage(message);
        setStatus("Cancelled.", "idle");
        if (!state.fullWindowOpen) {
          await closeLightweightWindow();
        }
        return;
      }

      if (preflightChoice === "skip-conflicts") {
        const conflictSources = new Set(drop.preflight.conflicts.map((item) => item.source));
        drop.preflight.conflicts.forEach((item) => {
          summary.skipped += 1;
          summary.skippedDetails.push(`${item.link}: target already exists`);
        });
        sources = drop.sources.filter((source) => !conflictSources.has(source));
      } else if (preflightChoice === "overwrite-conflicts") {
        overwriteSources = new Set(drop.preflight.conflicts.map((item) => item.source));
      }
    }

    for (let index = 0; index < sources.length; index += 1) {
      const source = sources[index];
      const conflictChoice = overwriteSources.has(source) ? "overwrite" : appliedChoice;
      let result = await invoke("create_direct_link_step", {
        source,
        targetDir: drop.targetDir,
        kind,
        conflictChoice,
      });

      if (result.status === "needsConflict") {
        await showLightweightWindow();
        const answer = await showConflictModal(result.link, index < sources.length - 1);
        if (answer.choice === "cancel") {
          summary.cancelled += sources.length - index;
          break;
        }
        if (answer.applyToRemaining) {
          appliedChoice = answer.choice;
        }
        result = await invoke("create_direct_link_step", {
          source,
          targetDir: drop.targetDir,
          kind,
          conflictChoice: answer.choice,
        });
      }

      recordDirectStep(summary, result);
      setStatus(`Processed ${index + 1} of ${sources.length}.`, "idle");
    }

    const message = directSummaryMessage(summary, kind);
    resultMessage(message);
    setStatus("Done.", summary.failed ? "error" : "ok");
    if (isCleanDirectSummary(summary) && !state.fullWindowOpen) {
      await closeLightweightWindow();
      return;
    }

    await showLightweightWindow();
    await showMessageModal("Batch Complete", message, summary.failed ? "error" : "ok");
    if (!state.fullWindowOpen) {
      await closeLightweightWindow();
    }
  } catch (error) {
    const message = error?.message || String(error);
    displayError(error);
    await showLightweightWindow();
    await showMessageModal("LinkForge", message, "error");
    if (!state.fullWindowOpen) {
      await closeLightweightWindow();
    }
  }
}

async function finishLightweightWithModal(title, message, tone = "") {
  resultMessage(message);
  setStatus(tone === "error" ? message : "Done.", tone === "error" ? "error" : "ok");
  await showLightweightWindow();
  await showMessageModal(title, message, tone);
  if (!state.fullWindowOpen) {
    await closeLightweightWindow();
  }
}

async function runPickSource(context) {
  if (!requireTauri()) return;

  try {
    await invoke("pick_context_sources", { paths: context.paths || [] });
    if (!state.fullWindowOpen) {
      await closeLightweightWindow();
    }
  } catch (error) {
    const message = error?.message || String(error);
    displayError(error);
    await finishLightweightWithModal("Pick Link Source Failed", message, "error");
  }
}

async function runLightweightSameFile(context) {
  if (!requireTauri()) return;

  try {
    if (context.paths?.length !== 2) {
      throw new Error("Same-file context action requires exactly two paths.");
    }
    const result = await invoke("same_file", {
      pathA: context.paths[0],
      pathB: context.paths[1],
    });
    await finishLightweightWithModal("Same File", result.message, result.same ? "ok" : "");
  } catch (error) {
    const message = error?.message || String(error);
    displayError(error);
    await finishLightweightWithModal("Same File Failed", message, "error");
  }
}

async function runLightweightLinkCount(context) {
  if (!requireTauri()) return;

  try {
    if (context.paths?.length !== 1) {
      throw new Error("Link-count context action requires exactly one path.");
    }
    const result = await invoke("link_count", { path: context.paths[0] });
    await finishLightweightWithModal("Link Count", result.message, "ok");
  } catch (error) {
    const message = error?.message || String(error);
    displayError(error);
    await finishLightweightWithModal("Link Count Failed", message, "error");
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

    if (isLightweightAction(context.action)) {
      enterLightweightMode();
    }

    if (isDropAction(context.action)) {
      await runDirectDrop(context);
      return;
    }

    if (context.action === "pick-source") {
      await runPickSource(context);
      return;
    }

    if (context.action === "same-file") {
      await runLightweightSameFile(context);
      return;
    }

    if (context.action === "link-count") {
      await runLightweightLinkCount(context);
      return;
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
      await runSameFileComparison(
        requireField("same-a", "Path A"),
        requireField("same-b", "Path B"),
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
