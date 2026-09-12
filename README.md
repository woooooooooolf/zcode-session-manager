# ZCode Session Manager (zsm)

浏览、检查并**彻底删除** ZCode 的历史会话（含归档会话）的桌面工具。Tauri 2 + Rust 实现，
安装包小、无运行时依赖。中英双语界面，浅色 / 深色 / 高对比度三主题。

## 功能

- **自动定位** ZCode 数据目录（默认 `~/.zcode`），可在设置中手动指定
- **扫描全部会话**：标题、项目、消息数、磁盘占用、归档/置顶/子会话/索引残影标记
- **搜索 / 筛选 / 排序**：按标题、ID、项目搜索；归档、置顶筛选；多种排序
- **会话详情**：消息流（用户 / 助手 / 思考过程 / 工具调用及输出），只读
- **独立删除 / 多选删除**：两步确认，显示级联范围（子会话、索引残影、行数、磁盘占用）
- **删除前自动备份**：简单拷贝到 `<zcode>/zsm-backups/<时间戳>/`，含两个数据库与被删会话的磁盘文件
- **删除后完整性校验**：对两个数据库执行 `PRAGMA integrity_check`；失败时自动从备份恢复并提示
- **兼容性检查**：校验数据库表结构与预期一致（ZCode 客户端更新导致格式变化时，提示并禁止操作）
- **受限模式**：ZCode 运行中无需退出即可清理——仅可删除已归档、闲置超过阈值
  （默认 1 小时，可设 1 分钟 ~ 365 天，支持分钟/小时/天单位自动换算）且未被启用中的
  定时自动化引用的会话
- **主题 / 语言**：内置浅色、深色、高对比度主题与中英文界面，偏好持久化，窗口标题随语言切换

## ZCode 会话数据的存储位置

本工具针对以下经过验证的布局（ZCode 桌面版）：

| 位置 | 内容 |
| --- | --- |
| `~/.zcode/v2/tasks-index.sqlite` | 桌面端会话索引，归档/置顶标记在这里 |
| `~/.zcode/cli/db/db.sqlite` | 会话元数据与消息正文 |
| `~/.zcode/cli/rollout/model-io-<id>.jsonl` | 模型输入/输出原始记录 |
| `~/.zcode/cli/{exec,artifacts,image-cache,agents}/<id>/` | 会话执行数据 |
| `~/.zcode/v2/checkpoints/` | 全局文件快照（按文件哈希组织，与本工具无关，不会改动） |

## 删除了什么

对每个选中会话（含其全部子孙会话）：

1. **db.sqlite**（单事务，失败整体回滚）：`message`、`part`、`session_entry`、`session_input`、
   `session_target`、`todo`、`tool_usage`、`turn_usage`、`model_usage`、`input_history`（按 `session_id`）；
   `session_task_link`（双向）；`workflow_run`/`workflow_event`/`workflow_activity`；最后删 `session` 行。
2. **磁盘**：`rollout` 记录与 `exec`、`artifacts`、`image-cache`、`agents` 下该会话的目录。
3. **tasks-index.sqlite**（最后删）：`tasks` 行、`task_group_members`、`automation_runs`、
   `off_peak_tasks` 中对这些会话的引用。索引最后删——中途失败只会留下无害的孤儿数据，
   不会让界面指向已删除的内容。

保留不动：`permission`、`local_setting`、`memories/`、`v2/checkpoints/`、`exec/shell-snapshots/` 等共享数据。

## 开发

```powershell
# 前置：Rust (stable-msvc) + Node 可选 + tauri-cli
cargo test -p zsm-core      # 核心逻辑单元测试（临时伪库，不碰真实数据）
cargo tauri dev             # 开发运行
cargo build --release -p zsm-gui   # 直接产出 target/release/zsm-gui.exe
cargo tauri build           # 产出 NSIS 安装包
```

**关于前端资源打包**：release 构建时 `ui/`（frontendDist）下的全部文件会在编译期
压缩嵌入 exe（Tauri v2 codegen 机制），单文件即可分发，无需随身携带静态资源；
debug 构建则从磁盘读取 `ui/`（改前端即时生效）。已验证：把 release 版 exe 单独拷贝到
空目录运行，界面（样式/文案/图标/数据）完整无缺；NSIS 安装包约 2.8 MB。
注意：若在 `ui/` 里新增了被 `index.html` 引用的文件，直接重新构建即可自动包含；
但若引用了不存在的文件，release 下只会在运行时表现为功能缺失——构建不会报错。

结构：

```
crates/zsm-core   # 核心逻辑：扫描 / 兼容性 / 备份 / 级联删除 / 完整性校验+恢复（UI 无关）
src-tauri         # Tauri 后端：命令、设置持久化、安全拦截
ui                # 前端：原生 HTML/CSS/JS，i18n 与主题无构建依赖
tools/make_icon.py  # 图标生成脚本
```

## 安全设计

- 删除需两步确认；确认框列出将删除的每个会话、行数与磁盘占用
- 备份（简单拷贝、时间戳目录）先于任何删除；备份包含恢复所需的全部文件
- 删除后自动 `integrity_check`；未通过则自动恢复刚创建的备份并在界面提示
- 数据库结构不符合预期（如客户端更新）时禁止一切操作
- **受限模式**（ZCode 运行中）：计划内每个会话都必须闲置超过阈值（新鲜度取
  `session.time_updated` 与 tasks-index `updated_at` 的较大值）且未被启用的自动化引用；
  根会话还必须已归档（级联子会话豁免归档条件——ZCode 不向 fork 传播归档标记）。
  任一条件不满足即整体拒绝，逐条给出原因；ZCode 关闭时无此限制
- 所有动态内容以纯文本渲染，杜绝注入

## 已知边界

- 不清理 `memories/`（按项目组织）与 `v2/checkpoints/`（全局快照，验证不含会话 ID）
- 自动化配置（`automations` 表）若仍引用被删会话会给出警告，但不改动配置本体
