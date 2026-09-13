<!-- 提交 PR 前请确认以下各项；PR 将被 squash 合并到 main，且需要 CI（build-test）通过。 -->

## 变更说明

<!-- 简要描述本次变更的内容与动机 -->

## 变更类型

- [ ] 新功能（feature）
- [ ] 缺陷修复（fix）
- [ ] 文档（docs）
- [ ] 重构 / 杂项（chore/refactor）

## 自查清单

- [ ] `cargo test -p zsm-core` 全部通过
- [ ] `cargo build --release -p zsm-gui` 构建成功，并已实际启动验证界面
- [ ] 涉及 UI 的改动已在浅色 / 深色 / 高对比度三种主题下检查
- [ ] 涉及中英文的改动已切换语言检查
- [ ] 破坏性变更（如有）已在描述中明确标注
