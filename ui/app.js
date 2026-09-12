/* ZCode Session Manager — frontend logic. All dynamic content is inserted
   via textContent (never innerHTML) so session data cannot inject markup. */

const $ = (id) => document.getElementById(id);

let STATE = null; // app_state payload (settings, compat, integrity, zcode_running)
let SESSIONS = [];
let SELECTED = new Set();
let LAST_RESULT = null; // backup dir of the last delete, for "open folder"
let SCAN_ERROR = null;

// ---------- invoke helpers ----------

function invoke(cmd, args) {
  if (!window.__TAURI__ || !window.__TAURI__.core) {
    return Promise.reject({ code: "no_backend", message: "Tauri backend not available" });
  }
  return window.__TAURI__.core.invoke(cmd, args);
}

function errText(e) {
  const code = typeof e === "object" && e && e.code ? e.code : "other";
  const known = [
    "zcode_running", "compat", "corruption", "invalid_dir", "db_not_found",
    "detect_failed", "io", "sqlite", "other", "no_dir", "no_backend",
  ];
  const key = known.includes(code) ? `err.${code}` : "err.other";
  const detail = typeof e === "object" && e && e.message ? e.message : String(e);
  return `${t(key)}\n${detail}`;
}

// ---------- formatting ----------

function fmtBytes(n) {
  n = Number(n || 0);
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB"];
  let u = -1;
  do { n /= 1024; u++; } while (n >= 1024 && u < units.length - 1);
  return `${n.toFixed(1)} ${units[u]}`;
}

function fmtTime(ms) {
  if (!ms) return "-";
  return new Date(ms).toLocaleString();
}

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined && text !== null) e.textContent = text;
  return e;
}

// ---------- boot ----------

document.addEventListener("DOMContentLoaded", async () => {
  try {
    STATE = await invoke("app_state");
  } catch (e) {
    STATE = null;
    console.error(e);
  }
  CURRENT_LANG = detectLang(STATE && STATE.settings ? STATE.settings.language : null);
  $("langSel").value = CURRENT_LANG;
  applyTheme(STATE && STATE.settings ? STATE.settings.theme : null);
  applyI18n();
  renderAbout();

  bindHeader();
  bindTabs();
  bindToolbar();
  bindSettings();
  bindDialogs();

  await refresh();
});

function bindHeader() {
  $("langSel").addEventListener("change", async () => {
    CURRENT_LANG = $("langSel").value;
    applyI18n();
    renderAbout();
    renderTable();
    await invoke("set_prefs", { language: CURRENT_LANG, theme: null }).catch(() => {});
  });
  $("themeSel").addEventListener("change", async () => {
    document.documentElement.dataset.theme = $("themeSel").value;
    await invoke("set_prefs", { language: null, theme: $("themeSel").value }).catch(() => {});
  });
  $("aboutBtn").addEventListener("click", () => $("aboutDlg").showModal());
}

function bindTabs() {
  $("tab-sessions").addEventListener("click", () => switchTab("sessions"));
  $("tab-settings").addEventListener("click", () => switchTab("settings"));
}

function switchTab(name) {
  $("view-sessions").hidden = name !== "sessions";
  $("view-settings").hidden = name !== "settings";
  $("tab-sessions").classList.toggle("active", name === "sessions");
  $("tab-settings").classList.toggle("active", name === "settings");
}

// ---------- data refresh ----------

async function refresh() {
  SCAN_ERROR = null;
  showTableState("loading");
  try {
    const [state, sessions] = await Promise.all([
      invoke("app_state"),
      invoke("list_sessions"),
    ]);
    STATE = state;
    SESSIONS = sessions;
    SELECTED.clear();
    $("langSel").value = CURRENT_LANG;
    renderAll();
  } catch (e) {
    SCAN_ERROR = e;
    SESSIONS = [];
    $("errorStateMsg").textContent = `${t("err.scan")}\n${errText(e)}`;
    showTableState("error");
    renderBanners();
    renderFooter();
  }
}

function showTableState(which) {
  $("sessTable").hidden = which !== "ok";
  $("emptyState").hidden = which !== "empty";
  $("errorState").hidden = which !== "error";
}

// ---------- banners / footer ----------

function deleteAllowed() {
  if (!STATE || !STATE.compat || !STATE.compat.ok) return false;
  if (STATE.zcodeRunning) return false;
  return (STATE.integrity || []).every((d) => d.state === "ok");
}

function renderAll() {
  renderBanners();
  renderTable();
  renderFooter();
  renderSettings();
}

function renderBanners() {
  const box = $("banners");
  box.textContent = "";
  const add = (cls, msg) => {
    const b = el("div", `banner ${cls}`, msg);
    box.appendChild(b);
  };
  if (!STATE) return;
  if (STATE.zcodeRunning) add("warn", t("banner.zcode"));
  if (STATE.compat && !STATE.compat.ok) {
    const b = el("div", "banner error", t("banner.compat"));
    const ul = el("ul", "banner-detail");
    for (const p of STATE.compat.problems) ul.appendChild(el("li", "", p));
    b.appendChild(ul);
    box.appendChild(b);
  }
  const corrupt = (STATE.integrity || []).filter((d) => d.state !== "ok");
  if (corrupt.length) {
    const b = el("div", "banner error", t("banner.corrupt"));
    const ul = el("ul", "banner-detail");
    for (const c of corrupt) ul.appendChild(el("li", "", `${c.db}: ${c.state} ${c.detail}`.trim()));
    b.appendChild(ul);
    box.appendChild(b);
  }
  if (STATE.compat && STATE.compat.ok) {
    for (const w of STATE.compat.warnings || []) add("warn", w);
  }
}

function renderFooter() {
  $("footerPath").textContent = STATE && STATE.dbPath ? STATE.dbPath : "";
  $("footerCount").textContent = STATE ? t("footer.sessions", { n: SESSIONS.length }) : "";
}

// ---------- table ----------

function visibleSessions() {
  const q = $("search").value.trim().toLowerCase();
  const filter = $("filterSel").value;
  const sort = $("sortSel").value;
  let rows = SESSIONS.slice();
  if (q) {
    rows = rows.filter(
      (s) =>
        (s.title || "").toLowerCase().includes(q) ||
        s.id.toLowerCase().includes(q) ||
        (s.project || "").toLowerCase().includes(q)
    );
  }
  if (filter === "archived") rows = rows.filter((s) => s.archived);
  if (filter === "pinned") rows = rows.filter((s) => s.pinned);
  const key = {
    updated: (s) => s.updatedMs,
    created: (s) => s.createdMs,
    size: (s) => s.diskBytes,
    title: (s) => (s.title || s.id).toLowerCase(),
  }[sort];
  if (sort === "title") rows.sort((a, b) => key(a).localeCompare(key(b)));
  else rows.sort((a, b) => key(b) - key(a));
  return rows;
}

function renderTable() {
  if (SCAN_ERROR) {
    showTableState("error");
    return;
  }
  const rows = visibleSessions();
  const body = $("sessBody");
  body.textContent = "";
  showTableState(rows.length ? "ok" : "empty");

  const canDelete = deleteAllowed();
  for (const s of rows) {
    const tr = el("tr");

    const tdCheck = el("td", "col-check");
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = SELECTED.has(s.id);
    cb.addEventListener("change", () => {
      if (cb.checked) SELECTED.add(s.id);
      else SELECTED.delete(s.id);
      updateSelInfo();
    });
    tdCheck.appendChild(cb);
    tr.appendChild(tdCheck);

    const tdTitle = el("td", "cell-title");
    const title = s.ghost ? t("badge.ghost") : s.title || s.id;
    tdTitle.appendChild(el("span", "title-text", title));
    tdTitle.title = s.id;
    const flags = el("span", "flags");
    if (s.ghost) flags.appendChild(el("span", "badge badge-ghost", "G"));
    if (s.archived) flags.appendChild(el("span", "badge badge-a", "A"));
    if (s.pinned) flags.appendChild(el("span", "badge badge-p", "P"));
    if (s.parentId) flags.appendChild(el("span", "badge badge-c", "c"));
    tdTitle.appendChild(flags);
    tr.appendChild(tdTitle);

    tr.appendChild(el("td", "cell-project", s.project || "-"));

    const ghostTime = s.ghost && !s.updatedMs;
    tr.appendChild(el("td", "", ghostTime ? "-" : fmtTime(s.updatedMs)));
    tr.appendChild(el("td", "num", s.messageCount === null || s.messageCount === undefined ? "-" : String(s.messageCount)));
    tr.appendChild(el("td", "num", fmtBytes(s.diskBytes)));

    const tdAct = el("td", "col-actions");
    const viewBtn = el("button", "btn-small", t("btn.view"));
    viewBtn.disabled = !!s.ghost;
    viewBtn.addEventListener("click", () => openDetail(s.id));
    tdAct.appendChild(viewBtn);
    const delBtn = el("button", "btn-small danger", t("btn.delete"));
    delBtn.disabled = !canDelete;
    delBtn.addEventListener("click", () => openDeleteDialog([s.id]));
    tdAct.appendChild(delBtn);
    tr.appendChild(tdAct);

    body.appendChild(tr);
  }
  $("checkAll").checked = rows.length > 0 && rows.every((s) => SELECTED.has(s.id));
  updateSelInfo();
}

function updateSelInfo() {
  const n = SELECTED.size;
  $("selInfo").textContent = n ? t("sel.count", { n }) : "";
  $("deleteSelBtn").disabled = n === 0 || !deleteAllowed();
}

function bindToolbar() {
  $("search").addEventListener("input", renderTable);
  $("filterSel").addEventListener("change", renderTable);
  $("sortSel").addEventListener("change", renderTable);
  $("refreshBtn").addEventListener("click", refresh);
  $("checkAll").addEventListener("change", () => {
    const rows = visibleSessions();
    if ($("checkAll").checked) rows.forEach((s) => SELECTED.add(s.id));
    else rows.forEach((s) => SELECTED.delete(s.id));
    renderTable();
  });
  $("deleteSelBtn").addEventListener("click", () => openDeleteDialog([...SELECTED]));
}

// ---------- detail dialog ----------

async function openDetail(id) {
  let detail;
  try {
    detail = await invoke("session_detail", { id });
  } catch (e) {
    toast(errText(e));
    return;
  }
  if (!detail) return;
  $("detailTitle").textContent = detail.title || detail.id;
  $("detailMeta").textContent = `${detail.id} · ${detail.directory || "-"} · ${fmtTime(detail.createdMs)}`;
  const body = $("detailBody");
  body.textContent = "";

  const visible = detail.messages.filter((m) => m.visible);
  if (!visible.length) {
    body.appendChild(el("p", "muted", t("detail.empty")));
  }
  for (const m of visible) {
    const block = el("div", `msg msg-${m.role || "other"}`);
    block.appendChild(
      el("div", "msg-role", m.role === "user" ? t("detail.user") : m.role === "assistant" ? t("detail.assistant") : m.role || "?")
    );
    let rendered = false;
    for (const p of m.parts) {
      if (p.type === "text" && p.text) {
        block.appendChild(el("p", "msg-text", p.text));
        rendered = true;
      } else if (p.type === "reasoning" && p.text) {
        const d = el("details", "msg-reasoning");
        d.appendChild(el("summary", "", t("detail.thinking")));
        d.appendChild(el("div", "reasoning-text", p.text));
        block.appendChild(d);
        rendered = true;
      } else if (p.type === "tool") {
        const state = p.state || {};
        const input = state.input || {};
        const keys = ["url", "prompt", "command", "file_path", "pattern", "query", "skill", "to"];
        const summary = keys
          .filter((k) => typeof input[k] === "string" && input[k])
          .map((k) => `${k}=${input[k]}`)
          .join(" | ");
        const line = el("div", "msg-tool");
        line.appendChild(
          el("span", "tool-head", `[${t("detail.tool")}] ${p.tool || ""} ${state.status || ""} ${summary}`.trim())
        );
        let output = state.output;
        if (output !== undefined && output !== null) {
          if (typeof output !== "string") output = JSON.stringify(output, null, 1);
          const d = el("details", "tool-output");
          d.appendChild(el("summary", "", t("detail.output")));
          d.appendChild(el("div", "output-text", output));
          line.appendChild(d);
        }
        block.appendChild(line);
        rendered = true;
      }
    }
    if (!rendered && m.parts.length) {
      const d = el("details", "msg-other");
      d.appendChild(el("summary", "", t("detail.more")));
      d.appendChild(el("pre", "", JSON.stringify(m.parts, null, 1)));
      block.appendChild(d);
    }
    body.appendChild(block);
  }
  $("detailDlg").showModal();
}

// ---------- delete flow ----------

async function openDeleteDialog(ids) {
  if (!deleteAllowed()) {
    toast(STATE && STATE.zcodeRunning ? t("banner.zcode") : t("banner.compat"));
    return;
  }
  let plan;
  try {
    plan = await invoke("delete_plan", { ids });
  } catch (e) {
    toast(errText(e));
    return;
  }
  const body = $("confirmBody");
  body.textContent = "";

  body.appendChild(el("p", "", t("del.intro")));

  const nRoots = plan.roots.length;
  const nChildren = plan.allIds.length - nRoots;
  const nGhosts = plan.metas.filter((m) => m.ghost).length;
  body.appendChild(
    el("p", "strong", t("del.sessions", { n: nRoots, c: Math.max(0, nChildren), g: nGhosts }))
  );

  const list = el("ul", "plan-list");
  for (const m of plan.metas) {
    const li = el("li", "", `${m.archived ? "[A] " : ""}${m.ghost ? t("badge.ghost") : m.title || m.id}`);
    li.appendChild(el("span", "muted small", ` ${m.id}`));
    list.appendChild(li);
  }
  body.appendChild(list);

  const rowsTxt = plan.counts
    .filter((c) => c.rows > 0)
    .map((c) => `${c.table}=${c.rows}`)
    .join(", ");
  const kv = el("div", "plan-kv");
  const addKv = (k, v) => {
    kv.appendChild(el("span", "k", k));
    kv.appendChild(el("span", "v", v));
  };
  addKv(t("del.rows"), rowsTxt || "-");
  addKv(t("del.disk"), fmtBytes(plan.diskBytes));
  body.appendChild(kv);

  body.appendChild(el("p", "backup-note", t("del.backupNote")));
  body.appendChild(el("p", "backup-path", plan.backupsDir));

  $("confirmGo").disabled = false;
  $("confirmGo").textContent = t("del.confirm");
  $("confirmGo").onclick = async () => {
    $("confirmGo").disabled = true;
    $("confirmGo").textContent = t("del.running");
    $("confirmDlg").close();
    await executeDelete(ids);
  };
  $("confirmDlg").showModal();
}

async function executeDelete(ids) {
  let res;
  try {
    res = await invoke("delete_execute", { ids });
  } catch (e) {
    toast(errText(e));
    return;
  }
  showResult(res);
  await refresh();
}

function showResult(res) {
  LAST_RESULT = res.backupDir;
  const body = $("resultBody");
  body.textContent = "";
  const restored = res.integrity && res.integrity.restored;
  const failed = res.integrity && !res.integrity.passed;

  $("resultTitle").textContent = failed ? t("res.failed") : t("res.title");
  if (failed) $("resultTitle").classList.add("danger");
  else $("resultTitle").classList.remove("danger");

  const kv = el("div", "plan-kv");
  const addKv = (k, v) => {
    kv.appendChild(el("span", "k", k));
    kv.appendChild(el("span", "v", v));
  };
  const rows = res.deleted.map((c) => `${c.table}=${c.rows}`).filter((s) => !s.endsWith("=0")).join(", ");
  addKv(t("res.rows"), rows || "-");
  addKv(t("res.files"), String(res.disk.files));
  addKv(t("res.disk"), fmtBytes(res.disk.bytes));
  body.appendChild(kv);

  body.appendChild(el("p", "", `${t("res.backup")}: `));
  body.appendChild(el("p", "backup-path", res.backupDir));

  if (res.integrity) {
    const p = el("p", restored ? "restore-warn" : "integrity-ok");
    p.textContent = failed ? (restored ? t("res.restored") : t("res.restoreFailed")) : t("res.integrityOk");
    body.appendChild(p);
    if (failed && !restored) {
      for (const e of res.integrity.restoreErrors) body.appendChild(el("p", "restore-warn", e));
    }
    if (restored) {
      const d = el("details", "restore-log");
      d.appendChild(el("summary", "", t("detail.more")));
      for (const l of res.integrity.restoreLog) d.appendChild(el("div", "small", l));
      body.appendChild(d);
    }
  }

  for (const w of res.warnings || []) body.appendChild(el("p", "restore-warn", `${t("res.warn")}: ${w}`));
  for (const e of res.disk.errors || []) body.appendChild(el("p", "restore-warn", e));

  $("resultOpenBackup").hidden = !res.backupDir;
  $("resultDlg").showModal();
}

// ---------- settings ----------

function renderSettings() {
  if (!STATE) return;
  $("dirInput").value = STATE.settings.zcodeDir || STATE.detectedDefault || "";
  $("backupsPath").textContent = STATE.backupsDir || "-";
}

function bindSettings() {
  $("browseBtn").addEventListener("click", async () => {
    try {
      const picked = await invoke("pick_folder");
      if (picked) $("dirInput").value = picked;
    } catch (e) {
      toast(errText(e));
    }
  });
  $("detectBtn").addEventListener("click", async () => {
    try {
      STATE = await invoke("detect_zcode_dir");
      renderAll();
      await refresh();
      $("dirMsg").textContent = t("set.saved");
    } catch (e) {
      $("dirMsg").textContent = errText(e);
    }
  });
  $("saveDirBtn").addEventListener("click", async () => {
    const path = $("dirInput").value.trim();
    try {
      STATE = await invoke("set_zcode_dir", { path });
      $("dirMsg").textContent = t("set.saved");
      renderAll();
      await refresh();
      switchTab("sessions");
    } catch (e) {
      $("dirMsg").textContent = errText(e);
    }
  });
  $("openBackupsBtn").addEventListener("click", () => {
    if (STATE && STATE.backupsDir) invoke("reveal_path", { path: STATE.backupsDir }).catch(() => {});
  });
}

// ---------- about ----------

function renderAbout() {
  const body = $("aboutBody");
  body.textContent = "";
  body.appendChild(el("p", "muted small", `v${(STATE && STATE.appVersion) || "0.1.0"}`));
  body.appendChild(el("p", "", t("about.p1")));
  body.appendChild(el("h4", "", t("about.layoutTitle")));
  const ul = el("ul", "about-list");
  for (const k of ["about.l1", "about.l2", "about.l3", "about.l4", "about.l5"]) {
    ul.appendChild(el("li", "", t(k)));
  }
  body.appendChild(ul);
  body.appendChild(el("h4", "", t("about.safetyTitle")));
  const ul2 = el("ul", "about-list");
  for (const k of ["about.s1", "about.s2", "about.s3", "about.s4"]) {
    ul2.appendChild(el("li", "", t(k)));
  }
  body.appendChild(ul2);
}

// ---------- misc ----------

function bindDialogs() {
  document.querySelectorAll("[data-close]").forEach((btn) => {
    btn.addEventListener("click", () => $(btn.dataset.close).close());
  });
  $("confirmCancel").addEventListener("click", () => $("confirmDlg").close());
  $("resultClose").addEventListener("click", () => $("resultDlg").close());
  $("resultOpenBackup").addEventListener("click", () => {
    if (LAST_RESULT) invoke("reveal_path", { path: LAST_RESULT }).catch(() => {});
  });
}

let toastTimer = null;
function toast(msg) {
  const elx = $("toast");
  elx.textContent = msg;
  elx.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    elx.hidden = true;
  }, 6000);
}

function applyTheme(saved) {
  let theme = saved;
  if (!theme) {
    theme = window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  document.documentElement.dataset.theme = theme;
  $("themeSel").value = theme;
}
