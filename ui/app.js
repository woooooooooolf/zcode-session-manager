/* ZCode Session Manager — frontend logic. All dynamic content is inserted
   via textContent (never innerHTML) so session data cannot inject markup. */

const $ = (id) => document.getElementById(id);

// surface unexpected JS errors visibly instead of a silently broken UI
window.addEventListener("error", (e) => {
  const t = document.getElementById("toast");
  if (t) {
    t.textContent = `JS Error: ${e.message}\n${e.filename}:${e.lineno}:${e.colno}`;
    t.hidden = false;
  }
});
window.addEventListener("unhandledrejection", (e) => {
  const t = document.getElementById("toast");
  if (t) {
    t.textContent = `Promise Error: ${e.reason && e.reason.message ? e.reason.message : e.reason}`;
    t.hidden = false;
  }
});

let STATE = null; // app_state payload (settings, compat, integrity, zcode_running)
let SESSIONS = [];
let SELECTED = new Set();
let LAST_RESULT = null; // backup dir of the last delete, for "open folder"
let SCAN_ERROR = null;
let FILTER = new Set(); // categories: active/archived/pinned/child/ghost (empty = all)
let SORT = { key: "updated", dir: "desc" };
const DEFAULT_DIR = { title: "asc", project: "asc", updated: "desc", messages: "desc", size: "desc" };

const GITHUB_URL = "https://github.com/woooooooooolf/zcode-session-manager";

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
    "zcode_running", "limited_mode", "compat", "corruption", "invalid_dir", "db_not_found",
    "detect_failed", "io", "sqlite", "other", "no_dir", "no_backend",
  ];
  const key = known.includes(code) ? `err.${code}` : "err.other";
  const detail = typeof e === "object" && e && e.message ? e.message : String(e);
  const problems = typeof e === "object" && e && Array.isArray(e.problems) && e.problems.length
    ? "\n" + e.problems.join("\n")
    : "";
  return `${t(key)}${problems}\n${detail}`;
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

function fmtDuration(minutes) {
  const m = Number(minutes) || 60;
  const sp = CURRENT_LANG === "zh" ? "" : " ";
  const singular = CURRENT_LANG === "en" ? ["unit.minute", "unit.hour", "unit.day"] : null;
  let value, key;
  if (m % 1440 === 0) {
    value = m / 1440;
    key = singular && value === 1 ? singular[2] : "unit.days";
  } else if (m % 60 === 0) {
    value = m / 60;
    key = singular && value === 1 ? singular[1] : "unit.hours";
  } else {
    value = m;
    key = singular && value === 1 ? singular[0] : "unit.minutes";
  }
  return `${value}${sp}${t(key)}`;
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
  applyTheme(STATE && STATE.settings ? STATE.settings.theme : null);
  applyI18n();
  applyWindowTitle();
  renderAbout();
  renderHelp();
  renderFilterMenu();

  bindHeader();
  bindTabs();
  bindToolbar();
  bindSortHeaders();
  bindSettings();
  bindDialogs();
  startPolling();

  await refresh();
});

// ---------- popovers ----------

function closePopovers() {
  document.querySelectorAll(".menu").forEach((m) => (m.hidden = true));
}

function togglePopover(menu, anchor) {
  const wasHidden = menu.hidden;
  closePopovers();
  if (!wasHidden) return;
  menu.hidden = false;
  const r = anchor.getBoundingClientRect();
  menu.style.top = `${Math.round(r.bottom + 4)}px`;
  const width = Math.min(280, menu.offsetWidth || 220);
  menu.style.left = `${Math.max(8, Math.round(Math.min(r.left, window.innerWidth - width - 8)))}px`;
}

document.addEventListener("click", (e) => {
  if (e.target.closest(".menu") || e.target.closest("#filterBtn") || e.target.closest("#themeBtn")) return;
  closePopovers();
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") closePopovers();
});

function bindHeader() {
  // language: two options only, so the button simply toggles
  $("langBtn").addEventListener("click", () => switchLang(CURRENT_LANG === "zh" ? "en" : "zh"));

  const themeMenu = el("div", "menu");
  themeMenu.id = "themeMenu";
  themeMenu.hidden = true;
  document.body.appendChild(themeMenu);
  themeMenu.addEventListener("click", (e) => e.stopPropagation());
  $("themeBtn").addEventListener("click", (e) => {
    e.stopPropagation();
    renderThemeMenu(themeMenu);
    togglePopover(themeMenu, $("themeBtn"));
  });

  $("helpBtn").addEventListener("click", () => $("helpDlg").showModal());
  $("aboutBtn").addEventListener("click", () => $("aboutDlg").showModal());
}

function renderThemeMenu(menu) {
  menu.textContent = "";
  for (const value of ["light", "dark", "hc"]) {
    const item = el("button", "menu-item", "");
    const label = el("span", "", t(`theme.${value}`));
    item.appendChild(label);
    if (document.documentElement.dataset.theme === value) {
      item.appendChild(el("span", "menu-check", "✓"));
    }
    item.addEventListener("click", () => {
      document.documentElement.dataset.theme = value;
      invoke("set_prefs", { language: null, theme: value, idleMinutes: null }).catch(() => {});
      closePopovers();
      renderThemeMenu(menu);
    });
    menu.appendChild(item);
  }
}

async function switchLang(lang) {
  CURRENT_LANG = lang;
  applyI18n();
  applyWindowTitle();
  renderAbout();
  renderHelp();
  renderFilterMenu();
  renderTable();
  // dynamically generated chrome must follow the language too
  renderBanners();
  renderFooter();
  await invoke("set_prefs", { language: CURRENT_LANG, theme: null, idleMinutes: null }).catch(() => {});
}

/// Keep the native window title in sync with the UI language.
function applyWindowTitle() {
  const title = t("app.windowTitle");
  document.title = title;
  invoke("set_window_title", { title }).catch(() => {});
}

// ---------- live ZCode state ----------

function startPolling() {
  // the authoritative guard lives in delete_execute; this keeps the UI honest
  setInterval(async () => {
    if (!STATE) return;
    try {
      const running = await invoke("is_zcode_running");
      if (running !== STATE.zcodeRunning) {
        STATE.zcodeRunning = running;
        renderBanners();
        renderTable();
      }
    } catch {
      /* transient probe failure — keep previous state */
    }
  }, 4000);
}

// ---------- tabs ----------

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
    $("langBtn").title = t("lang.label");
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

// ---------- gating ----------

function coreOpsAllowed() {
  return !!(STATE && STATE.compat && STATE.compat.ok &&
    (STATE.integrity || []).every((d) => d.state === "ok"));
}

/// Limited mode while ZCode runs: archived + idle beyond the threshold.
/// The backend enforces this authoritatively (incl. automation references).
function rowDeletable(s) {
  if (!coreOpsAllowed()) return false;
  if (!STATE.zcodeRunning) return true;
  const idleMin = (STATE.settings && STATE.settings.idleMinutes) || 60;
  const updated = s.updatedMs || 0;
  return !!s.archived && Date.now() - updated >= idleMin * 60000;
}

function renderAll() {
  renderBanners();
  renderTable();
  renderFooter();
  renderSettings();
}

// ---------- banners / footer ----------

function renderBanners() {
  const box = $("banners");
  box.textContent = "";
  const add = (cls, msg) => {
    const b = el("div", `banner ${cls}`, msg);
    box.appendChild(b);
  };
  if (!STATE) return;
  if (STATE.zcodeRunning) {
    add("warn", t("banner.limited", { n: fmtDuration((STATE.settings && STATE.settings.idleMinutes) || 60) }));
  }
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

// ---------- filter / sort ----------

function matchFilter(s) {
  if (!FILTER.size) return true; // nothing selected = show everything
  if (FILTER.has("active") && !s.archived) return true;
  if (FILTER.has("archived") && s.archived) return true;
  if (FILTER.has("pinned") && s.pinned) return true;
  if (FILTER.has("child") && s.parentId) return true;
  if (FILTER.has("ghost") && s.ghost) return true;
  return false;
}

function renderFilterMenu() {
  const menu = $("filterMenu");
  if (!menu) return;
  menu.textContent = "";
  for (const cat of ["active", "archived", "pinned", "child", "ghost"]) {
    const item = el("label", "menu-item check-item");
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = FILTER.has(cat);
    cb.addEventListener("change", () => {
      if (cb.checked) FILTER.add(cat);
      else FILTER.delete(cat);
      $("filterCount").textContent = FILTER.size ? ` (${FILTER.size})` : "";
      renderTable();
    });
    item.appendChild(cb);
    item.appendChild(el("span", "", t(`cat.${cat}`)));
    menu.appendChild(item);
  }
  menu.appendChild(el("div", "menu-sep"));
  const clear = el("button", "menu-item", t("filter.clear"));
  clear.addEventListener("click", () => {
    FILTER.clear();
    $("filterCount").textContent = "";
    renderFilterMenu();
    renderTable();
  });
  menu.appendChild(clear);
}

function sortKeyFn() {
  const key = SORT.key;
  return (s) => {
    switch (key) {
      case "title": return (s.title || "").toLowerCase();
      case "project": return (s.project || "").toLowerCase();
      case "messages": return s.messageCount === null || s.messageCount === undefined ? -1 : s.messageCount;
      case "size": return s.diskBytes;
      default: return s.updatedMs;
    }
  };
}

function bindSortHeaders() {
  document.querySelectorAll("th.sortable").forEach((th) => {
    th.addEventListener("click", () => {
      const k = th.dataset.sort;
      if (SORT.key === k) SORT.dir = SORT.dir === "asc" ? "desc" : "asc";
      else SORT = { key: k, dir: DEFAULT_DIR[k] || "desc" };
      updateSortHeaders();
      renderTable();
    });
  });
  updateSortHeaders();
}

function updateSortHeaders() {
  document.querySelectorAll("th.sortable").forEach((th) => {
    const arrow = th.querySelector(".arrow");
    if (!arrow) return;
    arrow.textContent = th.dataset.sort === SORT.key ? (SORT.dir === "asc" ? "▲" : "▼") : "";
  });
}

/// Filtered by search + categories, then ordered as a tree: children follow
/// their parent (indented); roots and siblings follow the active sort.
function visibleSessions() {
  const q = $("search").value.trim().toLowerCase();
  let rows = SESSIONS.slice();
  if (q) rows = rows.filter((s) => (s.title || "").toLowerCase().includes(q));
  rows = rows.filter(matchFilter);

  const byId = new Map(rows.map((r) => [r.id, r]));
  const kids = new Map();
  const roots = [];
  for (const r of rows) {
    const pid = r.parentId && byId.has(r.parentId) ? r.parentId : null;
    if (pid) {
      if (!kids.has(pid)) kids.set(pid, []);
      kids.get(pid).push(r);
    } else {
      roots.push(r);
    }
  }
  const key = sortKeyFn();
  const cmp = (a, b) => {
    const ka = key(a); const kb = key(b);
    const c = ka < kb ? -1 : ka > kb ? 1 : 0;
    return SORT.dir === "asc" ? c || a.id.localeCompare(b.id) : -c || a.id.localeCompare(b.id);
  };
  roots.sort(cmp);
  for (const arr of kids.values()) arr.sort(cmp);

  const out = [];
  const walk = (list, depth) => {
    for (const r of list) {
      r._depth = depth;
      out.push(r);
      const k = kids.get(r.id);
      if (k && k.length) walk(k, depth + 1);
    }
  };
  walk(roots, 0);
  return out;
}

// ---------- table ----------

function renderTable() {
  if (SCAN_ERROR) {
    showTableState("error");
    return;
  }
  const rows = visibleSessions();
  const body = $("sessBody");
  body.textContent = "";
  showTableState(rows.length ? "ok" : "empty");

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
    tdTitle.style.paddingLeft = `${10 + (s._depth || 0) * 20}px`;
    if (s._depth > 0) tdTitle.appendChild(el("span", "tree-line", "└ "));
    tdTitle.appendChild(el("span", "title-text", s.ghost ? t("badge.ghost") : s.title || s.id));
    tdTitle.title = s.id;
    tr.appendChild(tdTitle);

    const tdFlags = el("td", "cell-flags");
    const chip = (cls, key) => tdFlags.appendChild(el("span", `badge ${cls}`, t(key)));
    if (s.ghost) chip("badge-ghost", "chip.ghost");
    if (s.archived) chip("badge-a", "chip.archived");
    if (s.pinned) chip("badge-p", "chip.pinned");
    if (s.parentId) chip("badge-c", "chip.child");
    if (!tdFlags.childNodes.length) tdFlags.appendChild(el("span", "muted", "—"));
    tr.appendChild(tdFlags);

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
    const deletable = rowDeletable(s);
    const delBtn = el("button", "btn-small danger", t("btn.delete"));
    delBtn.disabled = !deletable;
    if (!deletable && coreOpsAllowed() && STATE.zcodeRunning) {
      delBtn.title = t("banner.limited", {
        n: fmtDuration((STATE.settings && STATE.settings.idleMinutes) || 60),
      });
    }
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
  $("deleteSelBtn").disabled = n === 0 || !coreOpsAllowed();
}

function bindToolbar() {
  $("search").addEventListener("input", renderTable);
  $("refreshBtn").addEventListener("click", refresh);
  $("filterBtn").addEventListener("click", (e) => {
    e.stopPropagation();
    togglePopover($("filterMenu"), $("filterBtn"));
  });
  $("filterMenu").addEventListener("click", (e) => e.stopPropagation());
  $("checkAll").addEventListener("change", () => {
    const rows = visibleSessions().filter((s) => rowDeletable(s));
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
  if (!coreOpsAllowed()) {
    toast(STATE && STATE.zcodeRunning ? t("banner.limited", { n: fmtDuration(60) }) : t("banner.compat"));
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
  if (STATE && STATE.zcodeRunning) {
    body.appendChild(el("p", "backup-note", t("del.limitedNote")));
  }

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
  renderIdleSetting();
}

/// Threshold display: pick the largest unit that divides the stored minutes
/// evenly (days → hours → minutes); switching units re-derives the value,
/// saving converts back to minutes.
function renderIdleSetting() {
  const m = (STATE && STATE.settings && STATE.settings.idleMinutes) || 60;
  const unit = m % 1440 === 0 ? "1440" : m % 60 === 0 ? "60" : "1";
  $("idleUnit").value = unit;
  $("idleValue").value = String(m / Number(unit));
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
  $("idleUnit").addEventListener("change", renderIdleSetting);
  $("saveIdleBtn").addEventListener("click", async () => {
    const factor = Number($("idleUnit").value);
    const raw = Number($("idleValue").value);
    if (!Number.isFinite(raw) || raw <= 0) {
      $("idleMsg").textContent = t("set.limitInvalid");
      return;
    }
    const minutes = Math.round(raw * factor);
    if (minutes < 1 || minutes > 525600) {
      $("idleMsg").textContent = t("set.limitRange");
      return;
    }
    try {
      const settings = await invoke("set_prefs", { language: null, theme: null, idleMinutes: minutes });
      STATE.settings = settings;
      renderIdleSetting();
      $("idleMsg").textContent = t("set.limitSaved", { n: fmtDuration(STATE.settings.idleMinutes) });
      renderBanners();
      renderTable();
    } catch (e) {
      $("idleMsg").textContent = errText(e);
    }
  });
  $("openBackupsBtn").addEventListener("click", () => {
    if (STATE && STATE.backupsDir) invoke("reveal_path", { path: STATE.backupsDir }).catch(() => {});
  });
}

// ---------- about card + changelog + licenses ----------

function renderAbout() {
  const body = $("aboutBody");
  if (!body) return;
  body.textContent = "";
  const card = el("div", "about-card");

  card.appendChild(el("div", "about-name", t("about.appName")));
  card.appendChild(el("div", "about-version", `v${(STATE && STATE.appVersion) || "1.0.0"}`));

  const author = el("div", "about-line");
  author.appendChild(el("span", "about-k", t("about.author")));
  author.appendChild(el("span", "about-v", t("about.authorValue")));
  card.appendChild(author);

  const gh = el("div", "about-line");
  gh.appendChild(el("span", "about-k", t("about.repo")));
  const link = el("a", "link", t("about.repoUrl"));
  link.href = "#";
  link.addEventListener("click", (e) => {
    e.preventDefault();
    invoke("open_external", { url: GITHUB_URL }).catch(() => {});
  });
  gh.appendChild(link);
  card.appendChild(gh);

  const btns = el("div", "about-btns");
  const notes = el("button", "", t("about.changelogBtn"));
  notes.addEventListener("click", openChangelog);
  const comps = el("button", "", t("about.licensesBtn"));
  comps.addEventListener("click", openLicenses);
  btns.appendChild(notes);
  btns.appendChild(comps);
  card.appendChild(btns);

  card.appendChild(el("p", "muted small", t("about.complianceNote")));
  body.appendChild(card);
}

function parseChangelog(text) {
  const sections = [];
  let cur = null;
  for (const line of text.split(/\r?\n/)) {
    const m = line.match(/^##\s+\[(.+?)\]\s*(?:-\s*(.*))?$/);
    if (m) {
      cur = { version: m[1], date: (m[2] || "").trim(), lines: [] };
      sections.push(cur);
      continue;
    }
    if (cur && line.trim()) cur.lines.push(line.trim());
  }
  return sections;
}

async function openChangelog() {
  let text;
  try {
    text = await invoke("changelog_text");
  } catch (e) {
    toast(errText(e));
    return;
  }
  const body = $("changelogBody");
  body.textContent = "";
  const sections = parseChangelog(text);
  if (!sections.length) body.appendChild(el("p", "muted", "—"));
  sections.forEach((sec, i) => {
    if (i > 0) body.appendChild(el("div", "section-divider"));
    const head = el("div", "cl-head");
    head.appendChild(el("span", "cl-version", sec.version));
    if (sec.date) head.appendChild(el("span", "muted small", ` ${sec.date}`));
    body.appendChild(head);
    let list = null;
    for (const line of sec.lines) {
      if (line.startsWith("###")) {
        body.appendChild(el("h5", "cl-sub", line.replace(/^#+\s*/, "")));
        list = null;
      } else if (line.startsWith("- ") || line.startsWith("* ")) {
        if (!list) {
          list = el("ul", "about-list");
          body.appendChild(list);
        }
        list.appendChild(el("li", "", line.replace(/^[-*]\s*/, "")));
      } else if (!line.startsWith("#")) {
        body.appendChild(el("p", "small", line));
        list = null;
      }
    }
  });
  $("changelogDlg").showModal();
}

async function openLicenses() {
  let table;
  let generated = true;
  try {
    const [rows, ok] = await Promise.all([
      invoke("third_party"),
      invoke("third_party_generated"),
    ]);
    table = rows;
    generated = ok;
  } catch (e) {
    toast(errText(e));
    return;
  }
  const body = $("licensesBody");
  body.textContent = "";
  if (!generated || !table.length) {
    body.appendChild(el("p", "muted", t("licenses.unavailable")));
  } else {
    const tbl = el("table", "license-table");
    const thead = el("thead");
    const hr = el("tr");
    for (const k of ["licenses.component", "licenses.version", "licenses.license"]) {
      hr.appendChild(el("th", "", t(k)));
    }
    thead.appendChild(hr);
    tbl.appendChild(thead);
    const tbody = el("tbody");
    for (const [name, version, license] of table) {
      const tr = el("tr");
      tr.appendChild(el("td", "", name));
      tr.appendChild(el("td", "", version));
      tr.appendChild(el("td", "", license));
      tbody.appendChild(tr);
    }
    tbl.appendChild(tbody);
    body.appendChild(tbl);
  }
  $("licensesDlg").showModal();
}

// ---------- help ----------

function renderHelp() {
  const body = $("helpBody");
  if (!body) return;
  body.textContent = "";
  body.appendChild(el("p", "", t("help.intro")));
  body.appendChild(el("h4", "", t("help.usageTitle")));
  const ul = el("ul", "about-list");
  for (const k of ["help.usage1", "help.usage2", "help.usage3"]) ul.appendChild(el("li", "", t(k)));
  body.appendChild(ul);
  body.appendChild(el("h4", "", t("help.badgesTitle")));
  const ul2 = el("ul", "about-list");
  for (const k of ["help.badgeA", "help.badgeP", "help.badgeC", "help.badgeG"]) {
    ul2.appendChild(el("li", "", t(k)));
  }
  body.appendChild(ul2);
  body.appendChild(el("h4", "", t("help.dataTitle")));
  const ul3 = el("ul", "about-list");
  for (const k of ["help.data1", "help.data2"]) ul3.appendChild(el("li", "", t(k)));
  body.appendChild(ul3);
  body.appendChild(el("h4", "", t("help.layoutTitle")));
  const ul4 = el("ul", "about-list");
  for (const k of ["about.l1", "about.l2", "about.l3", "about.l4", "about.l5"]) {
    ul4.appendChild(el("li", "", t(k)));
  }
  body.appendChild(ul4);
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
}
