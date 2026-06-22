# Bug: RecoveryScreen 文本居中塌缩成「一字一行」

- **状态**：已解决（2026-06-22）
- **发现**：2026-06-22，recovery-screen 功能手动验证时
- **严重度**：低（纯视觉；恢复操作功能正常）。但根因是全局陷阱，殃及所有用 `max-w-<档>` 的屏幕。
- **组件**：`src/components/RecoveryScreen.vue`、`src/components/SetupScreen.vue`
- **修复提交**：`max-w-md` → `max-w-[28rem]`（两处）+ 回归护栏

## 症状

进入恢复屏幕（如 Tier 2 `config_missing` / Tier 3 `root_missing`）时，居中文本列被压成 ~12px 宽：

- 标题、正文 `<p>` 每行只剩一个**单词**（"Your" / "template.yaml" / "is" / "missing" 各占一行，且因不可断词而向右溢出）。
- 路径 `<code>`（带 `break-all`）被压成每行一个**字符**。
- 内容列被 `mx-auto` 居中在极宽窗口里，于是整列偏到右侧，左边一大片空白。

## 根因（已证实）

**`max-w-md` 编译产物是 `max-width: var(--spacing-md)` = 12px，不是预期的 28rem。**

- `src/assets/main.css` 的 `@theme` 定义了具名 `--spacing-*` 档（`--spacing-md: 12px` …），但**没有定义 `--container-*` 档**。
- Tailwind v4 缺少 container 档时，把 `max-w-md` 的 `md` 当作 spacing key 解析 → `max-width: var(--spacing-md)` = 12px。更大的 t-shirt 档（`3xl`+）则不生成任何规则。
- 12px 宽的块里，正常文本按词换行、`break-all` 路径按字符换行；`mx-auto` 再把这条 12px 细缝居中。

证据（用**真实编译产物** `dist/assets/index-*.css` + headless Chrome 实测，viewport 1200）：

| 写法 | 计算出的 `max-width` | 列宽 | `<h1>` | `<code>` 路径 |
|---|---|---|---|---|
| `max-w-md`（旧，已上线） | **12px** | 12px | 4 行 | **65 行**（一字符一行） |
| `max-w-[28rem]`（修复） | 448px | 448px | 1 行 | 2 行 |

**与 flex/block 无关。** flex-item 收缩 min-content 是之前的**误诊**：`0faa7cb` 把容器从 flex 改成普通块流，但两种布局在 `max-w-md` = 12px 下塌缩方式相同。真正变量始终是那 12px。

## 修复

1. `RecoveryScreen.vue:38`、`SetupScreen.vue:12`：`max-w-md` → `max-w-[28rem]`（=448px，即 `md` 本意）。全仓库其他 `max-w-*` 早已用任意值（`max-w-[300px]` / `max-w-[92vw]`）；`max-*` 尺寸不受 `--spacing-*` token 规则约束（根 CLAUDE.md）。
2. 回归护栏：`src/__tests__/tailwind-token-usage.test.ts` 新增「never uses t-shirt sizes on width/height utilities」——禁止 `(max-w|min-w|w|max-h|min-h|h|size|basis)-(t-shirt)`，旧代码会红、修复后绿。

## 为什么之前没抓到

- **vitest 抓不到**：jsdom 无布局引擎，组件测试只验 data-testid/文本。
- **静态读代码抓不到**：源码里写的是 `max-w-md`，看不出它编译成 12px——必须看**编译产物**。第一次修复时用「手写等价 CSS」(把 `max-w-md` 当成 28rem) 在 headless Chrome 复现，于是假阴性、误判已修好。教训：构建管线会改写源码时，**对着编译产物验证，不要对着自己脑补的 CSS**。

## 参考

- 组件：`src/components/RecoveryScreen.vue`、`src/components/SetupScreen.vue`
- 护栏：`src/__tests__/tailwind-token-usage.test.ts`
- 记忆笔记：`tailwind-tshirt-sizes-collapse`（取代了被证伪的 `centered-text-screens-use-block-not-flex`）
