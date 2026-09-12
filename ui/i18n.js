// i18n dictionaries: zh (default) and en.
const I18N = {
  zh: {
    "app.title": "ZCode 会话管理器",
    "theme.label": "主题",
    "theme.light": "浅色",
    "theme.dark": "深色",
    "theme.hc": "高对比度",
    "lang.label": "语言",

    "tab.sessions": "会话列表",
    "tab.settings": "设置",

    "status.zcode": "ZCode 正在运行",
    "banner.zcode": "检测到 ZCode 正在运行。为避免数据库冲突，请先退出 ZCode 再执行删除操作。",
    "banner.compat": "ZCode 数据库格式与本工具的预期不符（可能因客户端更新导致）。所有操作已被禁用。",
    "banner.corrupt": "数据库完整性检查未通过，为保护数据已禁用删除操作。可尝试从备份目录恢复。",
    "banner.indexWarn": "未找到会话索引（v2\\tasks-index.sqlite），归档标记不可用，其余功能不受影响。",

    "search.ph": "搜索标题 / ID / 项目",
    "filter.all": "全部",
    "filter.archived": "仅归档",
    "filter.pinned": "仅置顶",
    "sort.updated": "按更新时间",
    "sort.created": "按创建时间",
    "sort.size": "按占用大小",
    "sort.title": "按标题",
    "btn.refresh": "刷新",
    "btn.deleteSelected": "删除选中",
    "sel.count": "已选 {n} 项",
    "sel.none": "",

    "col.title": "标题",
    "col.project": "项目",
    "col.updated": "更新时间",
    "col.messages": "消息",
    "col.size": "占用",
    "col.actions": "操作",
    "badge.ghost": "索引残影（无正文）",
    "badge.archived": "归档",
    "badge.pinned": "置顶",
    "badge.child": "子会话",
    "btn.view": "查看",
    "btn.delete": "删除",

    "empty.title": "没有找到会话",
    "empty.sub": "可在设置中更换 ZCode 数据目录，或清除筛选条件。",

    "footer.sessions": "{n} 个会话",

    "detail.user": "用户",
    "detail.assistant": "助手",
    "detail.thinking": "思考过程",
    "detail.tool": "工具",
    "detail.output": "输出",
    "detail.empty": "（无可显示的消息）",
    "detail.more": "详细信息",

    "del.title": "删除会话",
    "del.intro": "以下内容将被永久删除：",
    "del.sessions": "会话 {n} 个（含级联子会话 {c} 个、索引残影 {g} 个）",
    "del.rows": "数据库行",
    "del.disk": "磁盘占用",
    "del.backupNote": "删除前会自动备份相关数据库与会话文件到：",
    "del.zcodeWarn": "ZCode 正在运行，拒绝执行。",
    "del.confirm": "永久删除",
    "del.cancel": "取消",
    "del.running": "正在删除…",

    "res.title": "删除完成",
    "res.failed": "删除失败",
    "res.rows": "已删除行",
    "res.disk": "已释放磁盘",
    "res.files": "已删除文件",
    "res.backup": "备份位置",
    "res.integrityOk": "数据库完整性校验通过。",
    "res.restored": "完整性校验未通过——已自动从备份恢复全部数据，请查看日志。",
    "res.restoreFailed": "完整性校验未通过，且自动恢复失败：",
    "res.warn": "警告",
    "res.openBackup": "打开备份目录",
    "res.close": "关闭",

    "set.title": "设置",
    "set.dirLabel": "ZCode 数据目录",
    "set.dirHint": "该目录应包含 cli\\db\\db.sqlite 与 v2\\tasks-index.sqlite，通常是 C:\\Users\\<用户名>\\.zcode。",
    "set.browse": "浏览…",
    "set.detect": "自动定位",
    "set.save": "保存并重新扫描",
    "set.saved": "已保存。",
    "set.invalid": "该目录看起来不是 ZCode 数据目录（缺少 cli/db/db.sqlite 或 v2/tasks-index.sqlite）。",
    "set.backups": "备份保存于：",
    "set.openBackups": "打开",

    "about.title": "关于",
    "about.p1": "本工具用于浏览、检查并彻底删除 ZCode 的历史会话（含归档会话），并清理磁盘上的关联数据。删除前自动备份，删除后校验数据库完整性，异常时自动恢复。",
    "about.layoutTitle": "ZCode 把会话数据存在哪里？",
    "about.l1": "v2\\tasks-index.sqlite — 桌面端会话索引（归档/置顶标记在这里）",
    "about.l2": "cli\\db\db.sqlite — 会话元数据与消息正文",
    "about.l3": "cli\\rollout\\model-io-<id>.jsonl — 模型输入输出原始记录",
    "about.l4": "cli\\{exec, artifacts, image-cache, agents}\\<id>\\ — 会话执行数据",
    "about.l5": "v2\\checkpoints\\ — 全局文件快照（按文件哈希组织，与本工具无关，不会改动）",
    "about.safetyTitle": "安全规则",
    "about.s1": "删除默认两步确认；先备份（简单拷贝，含时间戳目录名）再删除。",
    "about.s2": "删除完成后自动校验两个数据库的完整性；失败则从刚才的备份自动恢复并提示。",
    "about.s3": "ZCode 正在运行时拒绝删除与清理操作。",
    "about.s4": "数据库格式与预期不符时（如客户端更新），提示并禁止操作。",
    "about.close": "关闭",

    "err.zcode_running": "ZCode 正在运行，请先退出 ZCode 再重试。",
    "err.compat": "数据库格式不兼容，操作已被拒绝。",
    "err.corruption": "数据库完整性异常，操作已被拒绝。",
    "err.invalid_dir": "目录不是有效的 ZCode 数据目录。",
    "err.db_not_found": "找不到会话数据库。",
    "err.detect_failed": "在用户主目录下未能自动定位 ZCode 数据目录。",
    "err.io": "文件读写失败。",
    "err.sqlite": "数据库操作失败。",
    "err.other": "操作失败。",
    "err.scan": "扫描会话失败",
    "err.detail": "读取会话详情失败",
    "toast.deleted": "已删除 {n} 个会话",
  },

  en: {
    "app.title": "ZCode Session Manager",
    "theme.label": "Theme",
    "theme.light": "Light",
    "theme.dark": "Dark",
    "theme.hc": "High contrast",
    "lang.label": "Language",

    "tab.sessions": "Sessions",
    "tab.settings": "Settings",

    "status.zcode": "ZCode is running",
    "banner.zcode": "ZCode is currently running. Close it before deleting sessions to avoid database conflicts.",
    "banner.compat": "The ZCode database format does not match what this tool expects (possibly due to a client update). All operations are disabled.",
    "banner.corrupt": "Database integrity check failed. Deleting is disabled to protect your data. You may restore from the backup directory.",
    "banner.indexWarn": "Session index (v2\\tasks-index.sqlite) not found — archive flags unavailable. Everything else still works.",

    "search.ph": "Search title / ID / project",
    "filter.all": "All",
    "filter.archived": "Archived",
    "filter.pinned": "Pinned",
    "sort.updated": "By updated",
    "sort.created": "By created",
    "sort.size": "By size",
    "sort.title": "By title",
    "btn.refresh": "Refresh",
    "btn.deleteSelected": "Delete selected",
    "sel.count": "{n} selected",
    "sel.none": "",

    "col.title": "Title",
    "col.project": "Project",
    "col.updated": "Updated",
    "col.messages": "Msgs",
    "col.size": "Size",
    "col.actions": "Actions",
    "badge.ghost": "Index ghost (no content)",
    "badge.archived": "Archived",
    "badge.pinned": "Pinned",
    "badge.child": "Child session",
    "btn.view": "View",
    "btn.delete": "Delete",

    "empty.title": "No sessions found",
    "empty.sub": "Try a different directory in Settings, or clear the filters.",

    "footer.sessions": "{n} sessions",

    "detail.user": "User",
    "detail.assistant": "Assistant",
    "detail.thinking": "Thinking",
    "detail.tool": "Tool",
    "detail.output": "Output",
    "detail.empty": "(no messages to show)",
    "detail.more": "Details",

    "del.title": "Delete sessions",
    "del.intro": "The following will be deleted permanently:",
    "del.sessions": "{n} session(s) — including {c} cascaded children and {g} index ghosts",
    "del.rows": "Database rows",
    "del.disk": "Disk space",
    "del.backupNote": "A backup (simple copy, timestamped) is created first at:",
    "del.zcodeWarn": "ZCode is running — refusing to proceed.",
    "del.confirm": "Delete permanently",
    "del.cancel": "Cancel",
    "del.running": "Deleting…",

    "res.title": "Deletion complete",
    "res.failed": "Deletion failed",
    "res.rows": "Rows deleted",
    "res.disk": "Disk freed",
    "res.files": "Files removed",
    "res.backup": "Backup location",
    "res.integrityOk": "Database integrity check passed.",
    "res.restored": "Integrity check failed — everything was automatically restored from the backup. See the log below.",
    "res.restoreFailed": "Integrity check failed and automatic restore failed:",
    "res.warn": "Warnings",
    "res.openBackup": "Open backup folder",
    "res.close": "Close",

    "set.title": "Settings",
    "set.dirLabel": "ZCode data directory",
    "set.dirHint": "The folder that contains cli\\db\\db.sqlite and v2\\tasks-index.sqlite — usually C:\\Users\\<you>\\.zcode.",
    "set.browse": "Browse…",
    "set.detect": "Auto-detect",
    "set.save": "Save & rescan",
    "set.saved": "Saved.",
    "set.invalid": "This directory does not look like a ZCode data directory (missing cli/db/db.sqlite or v2/tasks-index.sqlite).",
    "set.backups": "Backups are stored in:",
    "set.openBackups": "Open",

    "about.title": "About",
    "about.p1": "Browse, inspect and truly delete ZCode history sessions (including archived ones) together with their on-disk data. Every deletion is backed up first, verified afterwards, and automatically restored if the verification fails.",
    "about.layoutTitle": "Where does ZCode keep session data?",
    "about.l1": "v2\\tasks-index.sqlite — desktop session index (archive/pin flags live here)",
    "about.l2": "cli\\db\\db.sqlite — session metadata and message content",
    "about.l3": "cli\\rollout\\model-io-<id>.jsonl — raw model I/O records",
    "about.l4": "cli\\{exec, artifacts, image-cache, agents}\\<id>\\ — session execution data",
    "about.l5": "v2\\checkpoints\\ — global file snapshots (keyed by file hash; not touched by this tool)",
    "about.safetyTitle": "Safety rules",
    "about.s1": "Two-step delete confirmation; a simple-copy backup (timestamped folder) is created before anything is removed.",
    "about.s2": "Both databases are integrity-checked after deletion; on failure everything is restored from the backup and you are notified.",
    "about.s3": "Deleting is refused while ZCode is running.",
    "about.s4": "If the database format does not match expectations (e.g. after a client update), operations are blocked with a warning.",
    "about.close": "Close",

    "err.zcode_running": "ZCode is running — close it first and retry.",
    "err.compat": "Database format is incompatible — operation refused.",
    "err.corruption": "Database integrity is abnormal — operation refused.",
    "err.invalid_dir": "Not a valid ZCode data directory.",
    "err.db_not_found": "Session database not found.",
    "err.detect_failed": "Could not locate a ZCode data directory under the user home.",
    "err.io": "File I/O failed.",
    "err.sqlite": "Database operation failed.",
    "err.other": "Operation failed.",
    "err.scan": "Failed to scan sessions",
    "err.detail": "Failed to load session detail",
    "toast.deleted": "Deleted {n} session(s)",
  },
};

let CURRENT_LANG = "zh";

function t(key, params) {
  let s = (I18N[CURRENT_LANG] && I18N[CURRENT_LANG][key]) ?? I18N.zh[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      s = s.replaceAll(`{${k}}`, String(v));
    }
  }
  return s;
}

function applyI18n() {
  document.documentElement.lang = CURRENT_LANG === "zh" ? "zh" : "en";
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    el.textContent = t(el.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-ph]").forEach((el) => {
    el.placeholder = t(el.dataset.i18nPh);
  });
  document.querySelectorAll("[data-i18n-title]").forEach((el) => {
    el.title = t(el.dataset.i18nTitle);
  });
}

function detectLang(saved) {
  if (saved === "zh" || saved === "en") return saved;
  return (navigator.language || "en").toLowerCase().startsWith("zh") ? "zh" : "en";
}
