# CLAUDE.md

Logbook — 个人工作时间记录工具。Tauri 2.x + Vue 3 + TypeScript。

**设计文档**：Vault `1_Projects/Logbook/README.md`（产品设计中心，不做代码实现）
**技术规格**：`SPEC.md`
**后端约定**：`src-tauri/CLAUDE.md`（Rust 测试规范、模块结构）
**交互原则**：`docs/interaction-principles.md`（不丢输入、消解一致性、快捷键按频率分配、尊重输入上下文——治理所有前端交互）
**命名约定**：`docs/naming-conventions.md`（组件按职责命名、DTO 用 `*Input`、落盘格式与标识符解耦——治理命名）

## 命令字典

用户指令 → 具体命令的直映射。**不猜测，查到即执行。** 新增命令时同步更新此表。

| 用户说 | 执行命令 | 产物 |
|--------|---------|------|
| 打包正式版 / 生产版本 / production build | `pnpm tauri:build` | `Logbook.app`（`com.boxcounter.logbook`），含 CLI |
| 打包开发版 / dev build | `pnpm tauri:build:dev` | `Logbook Dev.app`（`com.boxcounter.logbook.dev`） |
| 启动 / run / dev | `pnpm tauri dev` | 开发模式热重载 |
| 测试 / test | `pnpm test` | vitest + cargo test |

## 项目级规则

### Phase checkpoint

每完成一个独立 phase（如 Integration tests、Phase 2）停下确认，**不连续推进多个 phase 不征求同意**。HANDOFF.md 的「下一步」是选项清单，不是空头支票。

### 文档一致性检查

何时该做：写 HANDOFF.md 之前、Phase 结束时，应完成一次文档一致性检查。

`/check-consistency` skill 已设为仅手动调用（其 frontmatter 含 `disable-model-invocation: true`），不会被自动触发。因此在上述时机，主动提醒用户运行 `/check-consistency`，由用户显式发起；用户说「检查一致性」/「文档同步」时同样运行。检查项目（文档 ↔ 文档 + 文档 ↔ 代码）已固化在 skill 定义中，不在此重复。

### 前端交互

新增或修改弹层、编辑器、输入控件、焦点/键盘/取消行为时，遵循 `docs/interaction-principles.md`，不逐组件另起一套。评审时按该文件逐条核对。

### 设计 token

间距、字号必须用语义 token：间距走 `--spacing-*` 命名档（`gap-sm`/`p-md`，禁止裸 px 与 Tailwind 数字默认档如 `p-4`）；字号走 `text-title/body/secondary/micro`（默认 `text-sm` 等已用 `--text-*: initial` 清除，不可用）；行高跟随字号档（`@theme` 的 `--text-<tier>--line-height`），元素继承档行高，需紧排时用 `leading-none`（唯一合法显式覆盖），禁止散装 `leading-[...]`/`leading-<number>`/`leading-tight` 等。组件尺寸（`w-`/`h-`/`min-`/`max-`）不在此约束内，可用任意 px。新增或调整阶梯走 PR 说明理由；破例需一行注释 + 显式豁免 + 人工签字。`src/__tests__/tailwind-token-usage.test.ts` 是可执行护栏（报错含合法替代），接入 `npm run verify` + pre-commit + CI。详见 `docs/superpowers/specs/2026-06-21-design-system-consolidation-design.md` §2–3。

### 其他

- 诊断先于计划：handoff 标记了 bug → 先写测试确认 bug 还存在 → 再计划修复
