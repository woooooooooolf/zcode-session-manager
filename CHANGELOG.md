# Changelog

All notable changes to this project are documented in this file. 所有显著变更记录于此。
The format is based on [Keep a Changelog](https://keepachangelog.com/); tags are `V`-prefixed (`V1.0.0`).

## [Unreleased] - 2026-09-13
<!-- zh -->
**界面与信息架构二次改版。**

### 新增
- 子会话在列表中树形缩进展示，任何排序下都保持父子相邻。
- 新增独立的"标记"列，以彩色文字芯片显示会话类型（已归档 / 置顶 / 子会话 / 残影），随界面语言切换。
- 列头点击排序（正序 / 逆序切换，带方向箭头），取代排序下拉框。
- 筛选扁平化为可组合的多选类别：使用中 / 已归档 / 置顶 / 子会话 / 残影，全不选即显示全部。
- 主题改为图标弹层菜单，语言一键切换；关于页改为"名片"式布局，"版本说明"与"开源组件"（构建时经 cargo metadata 自动生成名称 / 版本 / 许可证表）以二级对话框呈现；GitHub 链接可点击跳转浏览器。
- 实时探测 ZCode 进程的启动 / 退出，受限模式横幅与删除门控自动同步，探测间隔可在设置中调整（1 ~ 30 秒）。

### 调整
- 设置页随窗口自适应缩放；会话表格填满视口，横向滚动条始终可见。
- 搜索仅匹配标题；所有输入框禁用 WebView2 表单自动填充。
- 帮助内容与当前界面同步更新。
<!-- en -->
**Second major UI and information-architecture overhaul.**

### Added
- Child sessions are indented under their parent in a tree that survives any sorting.
- Dedicated badges column with full-text chips (archived / pinned / child / ghost) that follow the UI language.
- Column-header sorting with direction arrows replaces the sort dropdown.
- The filter is now a flat, combinable multi-select: in use / archived / pinned / child / ghost; nothing selected shows everything.
- Theme picker as a popover menu; one-click language toggle; the About page is a centered card whose "Release notes" and "Open-source components" (a name / version / license table generated at build time via cargo metadata) open as sub-dialogs; the GitHub link opens the repository in the browser.
- ZCode process start / exit is detected live and the limited-mode banner and delete gating follow automatically; the polling interval is configurable in Settings (1 – 30 seconds).

### Changed
- Settings panels scale with the window; the sessions table fills the viewport so the horizontal scrollbar is always visible.
- Search matches titles only; WebView2 form autofill is disabled on all inputs.
- The Help content is synced with the current UI.

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
