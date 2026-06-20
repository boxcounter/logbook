# CLAUDE.md

Logbook — 个人工作时间记录工具。Tauri 2.x + Vue 3 + TypeScript。

**设计文档**：Vault `1_Projects/Logbook/README.md`（产品设计中心，不做代码实现）
**技术规格**：`SPEC.md`
**后端约定**：`src-tauri/CLAUDE.md`（Rust 测试规范、模块结构）
**交互原则**：`docs/interaction-principles.md`（不丢输入、消解一致性、快捷键按频率分配、尊重输入上下文——治理所有前端交互）

## 项目级规则

### Phase checkpoint

每完成一个独立 phase（如 Integration tests、Phase 2）停下确认，**不连续推进多个 phase 不征求同意**。HANDOFF.md 的「下一步」是选项清单，不是空头支票。

### 文档一致性检查

触发条件：写 HANDOFF.md 之前、Phase 结束时、用户说「检查一致性」/「文档同步」时。

调用 `/check-consistency` skill。检查项目（文档 ↔ 文档 + 文档 ↔ 代码）已固化在 skill 定义中，不在此重复。

### 前端交互

新增或修改弹层、编辑器、输入控件、焦点/键盘/取消行为时，遵循 `docs/interaction-principles.md`，不逐组件另起一套。评审时按该文件逐条核对。

### 其他

- 诊断先于计划：handoff 标记了 bug → 先写测试确认 bug 还存在 → 再计划修复
