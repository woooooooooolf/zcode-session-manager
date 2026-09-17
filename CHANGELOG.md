# Changelog

All notable changes to this project are documented in this file. 所有显著变更记录于此。
The format is based on [Keep a Changelog](https://keepachangelog.com/); tags are `V`-prefixed (`V1.3.0`).

## [1.3.1] - 2026-09-17
<!-- zh -->
### 修复
- 适配 ZCode 桌面版 3.11.2（内置引擎 0.16.5，数据库迁移 `0019_dwf_journal`）的本地数据结构变更：`db.sqlite` 新增了多智能体工作流日志表 `dwf_run` / `dwf_actor` / `dwf_node` / `dwf_event`，删除会话时现在会同步清理关联数据（经 `dwf_run.parent_session_id` 与 `dwf_actor.session_id` 级联），此前会残留孤儿行；兼容性校验也会检查这些表的关联列。
- tasks-index 中三张表追加的 `model_selection` 列为纯增量变化，读写不受影响，无需额外适配。
<!-- en -->
### Fixed
- Compatible with the ZCode 3.11.2 desktop release (bundled engine 0.16.5,
  db migration `0019_dwf_journal`): `db.sqlite` gained the workflow-journal
  tables `dwf_run` / `dwf_actor` / `dwf_node` / `dwf_event`.
  Deleting a session now cascades into them (via `dwf_run.parent_session_id` and
  `dwf_actor.session_id`); previously orphan rows would be left behind once the
  feature is used. The compatibility check now validates their link columns too.
- The appended `model_selection` columns in three tasks-index tables are additive
  and need no adaptation.

## [1.3.0] - 2026-09-15
<!-- zh -->
### 调整
- 状态横幅紧贴选项卡栏边界显示，且仅在会话列表页出现。
- 帮助改为独立选项卡（原弹窗）。
- 设置页去除冗余标题，卡片更紧凑。
- 会话表格改为混合列宽（纯 CSS）：勾选 / 标记 / 更新时间 / 消息 / 占用 / 操作列按内容固定宽度，标题与项目列平分剩余空间随窗口伸缩；表头居中、排序箭头固定于列右缘；任何窗口宽度下都不出现水平滚动条。
### 修复
- 滚动表格时整行表头固定在顶部（此前仅"标记""操作"两列残留、其余表头随内容滚走——排序箭头的 relative 定位覆盖了表头的 sticky）。
- 同时持有多个标记（如"置顶"+"子会话"）的会话现在显示全部标记（此前标记列过窄导致第二个标记被静默裁剪）。
- 表头列间分割线在首次打开时即显示（此前仅在点击排序后出现）。
- 版本说明支持渲染行内 markdown（粗体与行内代码）。
- 禁用窗口右键菜单（WebView2 默认菜单在本应用无实际用途）。
<!-- en -->
### Changed
- Banners render flush against the tab bar and only on the sessions view; Help became a
  full-window tab; the Settings page lost its redundant title and uses compact cards.
- The sessions table now uses hybrid column widths (pure CSS): checkbox / badges /
  updated / messages / size / actions are fixed to their content, while title and project
  share the remaining space and flex with the window; headers are centered with the sort
  arrow pinned to each column's right edge; no horizontal scrollbar at any window size.
### Fixed
- The whole header row stays pinned while scrolling (previously only the 标记/操作
  cells stuck — the sort arrow's relative positioning had overridden the headers'
  sticky on sortable columns).
- Sessions holding several badges (e.g. 置顶 + 子会话) show all of them (the badges
  column was too narrow, silently clipping the second chip).
- Header separators are visible immediately on launch (previously they only appeared
  after clicking a header).
- Release notes render inline markdown (bold and inline code).
- The WebView2 default context menu is disabled.

## [1.2.0] - 2026-09-14
<!-- zh -->
**安全体系全面上线，界面细节打磨，依赖全面升级。**

### 新增
- 安全扫描体系：CodeQL 静态分析（每次推送 / PR / 每周定时）、Secret scanning 与推送保护、依赖审查（PR 引入漏洞依赖时检查失败）。
- 依赖自动更新：Dependabot 每周检查 Cargo 与 GitHub Actions 依赖。
- PR / Issue 模板：PR 自查清单（测试、构建、三主题、双语）与标准化 Issue 表单。
- 帮助改为独立选项卡，全文窗口展示说明文字。

### 调整
- 状态横幅仅在会话列表页显示。
- 设置页去除冗余标题，卡片更紧凑；空状态提示不再占用纵向空间。
- 依赖升级：rusqlite 0.40（含新版 SQLite）、sysinfo 0.39、thiserror 2、dirs 6 及 GitHub Actions 组件更新。
- README：标题统一为中文名、透明底三主题层叠主图（中英双语页面各自配图）。

### 修复
- 适配 sysinfo 0.39 的 OsStr API（进程探测已经运行时验证）。
- 工作流补全 GITHUB_TOKEN 最小权限声明。
<!-- en -->
**Security tooling across the board, UI polish, and dependency upgrades.**

### Added
- Security scanning: CodeQL static analysis (every push / PR / weekly), secret scanning
  with push protection, and dependency review that fails PRs introducing vulnerable packages.
- Automatic dependency updates: Dependabot checks Cargo and GitHub Actions dependencies weekly.
- PR / Issue templates: a self-check list (tests, build, three themes, both languages) and
  structured issue forms.
- Help moved to a dedicated full-window tab.

### Changed
- Status banners render on the sessions view only.
- The Settings page dropped its redundant title and uses compact cards; empty status lines
  no longer reserve vertical space.
- Dependency upgrades: rusqlite 0.40 (with a newer SQLite), sysinfo 0.39, thiserror 2,
  dirs 6, plus GitHub Actions component updates.
- README: unified Chinese naming and a transparent three-theme cascade hero image
  (per-language screenshots).

### Fixed
- Adapted to the sysinfo 0.39 OsStr API (process probe verified at runtime).
- CI workflow now declares minimal GITHUB_TOKEN permissions.

## [1.1.0] - 2026-09-13
<!-- zh -->
**界面与信息架构二次改版，删除流程的可控性进一步增强。**

### 新增
- 备份保存位置可在设置中修改（或一键恢复默认的 `zsm-backups\`）。
- 子会话在列表中树形缩进展示，任何排序下都保持父子相邻。
- 新增独立的"标记"列，以彩色文字芯片显示会话类型（已归档 / 置顶 / 子会话 / 残影），随界面语言切换。
- 列头点击排序（正序 / 逆序切换，带方向箭头），取代排序下拉框。
- 筛选扁平化为可组合的多选类别：使用中 / 已归档 / 置顶 / 子会话 / 残影，全不选即显示全部。
- 实时探测 ZCode 进程的启动 / 退出，受限模式横幅与删除门控自动同步，探测间隔可在设置中调整（1 ~ 30 秒）。
- 关于页改为"名片"式布局："版本说明"（解析内置 CHANGELOG，随界面语言切换）与"开源组件"（构建时经 cargo metadata 自动生成名称 / 版本 / 许可证表）以二级对话框呈现；GitHub 链接可点击跳转浏览器。

### 调整
- 顶栏按钮统一为文字样式并重排（中/EN、主题、帮助、关于）；主题改为弹层菜单，语言一键切换。
- 设置页随窗口自适应缩放、独立成页；会话表格填满视口，横向滚动条始终可见。
- 搜索仅匹配标题；所有输入框禁用 WebView2 表单自动填充。
- 帮助内容与当前界面同步更新；窗口标题随界面语言切换。
- README 全面改版：中英双语（中文默认）、虚构演示数据截图、三主题层叠主图。

### 修复
- 修复时间戳列以 REAL 类型存储时会话列表静默变为空的问题（现兼容整数 / 浮点并记录无法映射的行）。
- 修复切换语言后提示横幅与页脚计数不跟随的问题。
- 修复设置页未独立显示（被会话列表挤至下方）的问题。
- 修复保存删除阈值后前端状态错误导致操作失败的问题。
- 修复顶栏重复渲染与英文单位单复数（1 hour）等细节。
<!-- en -->
**Second major UI and information-architecture overhaul; deletion is even more controllable.**

### Added
- Configurable backup location: the backup directory can be changed in Settings
  (or reset to the default `zsm-backups\` next to the ZCode data directory).
- Child sessions are indented under their parent in a tree that survives any sorting.
- Dedicated badges column with full-text chips (archived / pinned / child / ghost) that follow the UI language.
- Column-header sorting with direction arrows replaces the sort dropdown.
- The filter is now a flat, combinable multi-select: in use / archived / pinned / child / ghost; nothing selected shows everything.
- ZCode process start / exit is detected live and the limited-mode banner and delete gating follow automatically; the polling interval is configurable in Settings (1 – 30 seconds).
- The About page is a centered card: "Release notes" (parsed from the bundled CHANGELOG, following the UI language) and "Open-source components" (a name / version / license table generated at build time via cargo metadata) open as sub-dialogs; the GitHub link opens the repository in the browser.

### Changed
- Uniform text buttons in the tab bar, reordered (中/EN, Theme, Help, About); the theme picker is a popover and the language toggles with one click.
- Settings panels scale with the window as a real full page; the sessions table fills the viewport so the horizontal scrollbar is always visible.
- Search matches titles only; WebView2 form autofill is disabled on all inputs.
- The Help content and the window title follow the current UI language.
- README overhaul: bilingual (Chinese default), fictional-data screenshots, a three-theme cascade hero image.

### Fixed
- Session lists silently emptying when timestamp columns were stored as REAL (integer / float are now both accepted and unmappable rows are logged).
- Banners and the footer count not following a language switch.
- The Settings page rendering below the session list instead of replacing it.
- A frontend state error after saving the delete threshold.
- A duplicated tab bar, English unit plurals ("1 hour") and other polish.

## [1.0.0] - 2026-09-13
<!-- zh -->
**首个完整版本。**

### 新增
- 会话扫描：自动定位 ZCode 数据目录（`~/.zcode`，可在设置中修改），列出全部会话的标题、项目、消息数、磁盘占用与归档 / 置顶 / 子会话 / 残影标记。
- 会话详情：只读消息流（用户 / 助手、思考过程、工具调用及输出）。
- 删除：单删与多选批量删除，两步确认并完整预览级联范围（子会话、索引残影、逐表行数、磁盘占用、备份位置）。
- 安全：删除前对两个数据库与全部会话文件做时间戳目录的简单拷贝备份；删除后执行 `PRAGMA integrity_check`，失败自动从本次备份恢复；数据库格式不符合预期时兼容性检查禁止一切操作。
- 受限模式：ZCode 运行时仅可删除已归档、闲置超过可配置阈值（默认 1 小时；1 分钟 ~ 365 天，支持分钟 / 小时 / 天）且未被启用中的自动化引用的会话。
- 关于（版本、发布说明、合规、作者）与帮助（用法、标记说明、恢复指引）对话框；窗口标题随语言切换；浅色 / 深色 / 高对比度三主题；中英双语界面。
<!-- en -->
First complete release.

### Added
- Session scanning: auto-locates the ZCode data directory (`~/.zcode`, overridable in Settings), lists all sessions with title, project, message count, disk usage and archive/pin/child/ghost badges.
- Session detail viewer: read-only message stream (user/assistant, reasoning, tool calls and outputs).
- Deletion: single or multi-select, two-step confirmation with a full cascade preview (children, index ghosts, per-table row counts, disk size, backup location).
- Safety: timestamped simple-copy backup of both databases and all session files before every deletion; `PRAGMA integrity_check` afterwards with automatic restore from the fresh backup on failure; a schema compatibility check blocks all operations on unexpected database formats.
- Limited mode: while ZCode is running, only archived sessions idle beyond a configurable threshold (default 1 hour; 1 minute – 365 days, minutes/hours/days units) and unreferenced by enabled automations can be deleted.
- About (version, release notes, compliance, author) and Help (usage, badge glossary, restore guide) dialogs; localized window title; light / dark / high-contrast themes; Chinese & English UI.
