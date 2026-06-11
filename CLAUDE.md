# CLAUDE.md

Logbook — 个人工作时间记录工具。Tauri 2.x + Vue 3 + TypeScript。

**设计文档**：Vault `1_Projects/Logbook/README.md`（产品设计中心，不做代码实现）
**技术规格**：`SPEC.md`
**后端约定**：`src-tauri/CLAUDE.md`（Rust 测试规范、模块结构）

## 项目级规则

### Phase checkpoint

每完成一个独立 phase（如 Integration tests、Phase 2）停下确认，**不连续推进多个 phase 不征求同意**。HANDOFF.md 的「下一步」是选项清单，不是空头支票。

### 文档一致性检查

写 HANDOFF.md 之前，dispatch subagent 做全量交叉比对。范围：

- **文档 ↔ 文档**：Vault `1_Projects/Logbook/README.md` ↔ `SPEC.md` ↔ `HANDOFF.md` ↔ `src-tauri/CLAUDE.md`
- **文档 ↔ 代码**：上述文档 vs 实际 Rust 模块、Vue 组件、命令签名、数据结构

Subagent 读全部文档和代码，报告不一致项。不要裁剪——命令、数据结构、组件树、约定、Phase 进度全部比。

### 其他

- 诊断先于计划：handoff 标记了 bug → 先写测试确认 bug 还存在 → 再计划修复
- 后端只读：`~/.claude/projects/.../memory/` 里标记的只读仓库不在此项目，但注意 Slax Reader backend 不在本仓库
