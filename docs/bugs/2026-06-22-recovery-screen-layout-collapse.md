# Bug: RecoveryScreen 文本居中塌缩成「一字一行」

- **状态**：未解决（已知问题，不阻塞功能）
- **发现**：2026-06-22，recovery-screen 功能手动验证时
- **严重度**：低（纯视觉；恢复操作功能正常）
- **组件**：`src/components/RecoveryScreen.vue`
- **已落地分支**：已合并入 `main`，相关提交 `0faa7cb`

## 症状

进入恢复屏幕（如 Tier 2 `config_missing` / Tier 3 `root_missing`）时，居中文本列被压得极窄：

- 标题、正文 `<p>` 每行只剩一个**单词**（如 "Your" / "data" / "folder" 各占一行）。
- 路径 `<code>`（带 `break-all`）被压成每行一个**字符**。
- 内容列整体偏窄（~80–270px），靠 `justify-center`/`mx-auto` 居中，左侧留大片空白。

两张截图存于本地图片缓存（会话内），未附到仓库。

## 复现

`pnpm tauri dev` → 制造 `config_missing`（删除数据目录里的 `template.yaml`，保留目录）或 `root_missing`（删整个数据目录）→ 启动 → 观察恢复屏幕。

## 已试方案（均未解决）

1. **flex-col items-center + 给文本元素加宽度**（commit 已被 amend 覆盖）：原始就是 `flex flex-col items-center`，文本无显式宽度 → 塌缩。
2. **改成 flex 行 + `w-full max-w-md` 子项**：更糟，路径变成一字符一行（flex 行的单一子项 `flex-shrink:1` + `min-width:auto` 在窗口窄于 max-content 时塌到 min-content）。
3. **最终（`0faa7cb`，当前 main 上的版本）**：彻底弃用 flex 容纳内容，改用规范居中块 `<div class="mx-auto max-w-md text-center">`（普通块流，子元素填满 28rem、按文本流换行）。**理论上 bulletproof，但用户报告仍未解决。**

## 已排除 / 已确认的证据

- **不是全局 CSS**：`src/assets/main.css`、`tokens.css` 里 `body`/`#app` 无 `display:flex`、无 `place-items`、无宽度约束（普通块，全宽）。
- **不是工具类没生成**：grep 构建产物 `dist/assets/index-*.css` 确认 `mx-auto` / `max-w-md`(=28rem) / `break-all` / `flex-wrap` / `whitespace-nowrap` / `text-center` **均已生成**。
- **测试抓不到**：vitest 用 jsdom，无布局引擎，组件测试只验 data-testid/文本，无法发现塌缩。
- **功能正常**：Recreate / Reveal / Choose folder / Retry / Start-fresh 两步确认均工作。

## 当前假设（下次从这里查）

规范的 `mx-auto max-w-md` 普通块**不可能**自己塌成 min-content——除非它的**某个祖先节点**被 shrink-wrap 成 `width: min-content` / `fit-content`，或某祖先是 `display:flex`/`inline-block` 让 RecoveryScreen 不再撑满。静态读代码看不出来，必须看运行时 computed styles。

**下一步（需 devtools，新会话）**：
1. `pnpm tauri dev`，进恢复屏幕，开 devtools。
2. 选中那个一字一行的 `<p>`，沿 DOM 往上逐层看 computed `width`：
   `<p>` → `.mx-auto.max-w-md` 包裹块 → RecoveryScreen 根 `.min-h-screen.p-2xl` → App.vue 的 `<div class="min-h-screen">` → `#app` → `body`。
3. 找到第一个 `width` 异常（min-content/fit-content/极窄）的层 = 真凶。重点怀疑 App.vue 根容器或 `#app` 是否在某处被设成 inline/flex/收缩宽度。
4. 也确认一下当时**窗口本身**是不是异常窄（截图疑似 retina，CSS 宽度可能只有 ~337px）——若窗口极窄是叠加因素，需确认正常宽度下是否仍塌缩。

## 参考

- 组件：`src/components/RecoveryScreen.vue`
- 可工作的对照：`src/components/SetupScreen.vue`（普通块流 + `max-w-md`，渲染正常）
- 记忆笔记：`centered-text-screens-use-block-not-flex`（已记录 flex-item 塌缩陷阱）
