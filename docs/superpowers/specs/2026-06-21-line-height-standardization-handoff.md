# HANDOFF — 行高标准化（line-height standardization）「1b」

**状态**：未开始。这是设计规范梳理的**后续可选轮次**，单独走 mockup 评审，不在已完成的字号/间距 consolidation 内。
**前置**：分支 `worktree-ux-design-discussion` 的设计规范梳理已完成（spec `docs/superpowers/specs/2026-06-21-design-system-consolidation-design.md`，plan `docs/superpowers/plans/2026-06-21-design-system-consolidation.md`，截至 commit `20d1833`）。

---

## 为什么单独成一轮

字号 consolidation 这轮的承诺是「≤1px、肉眼看不出」，且是**纯字号**的 1:1 迁移。期间一度给 `@theme` 的四个字号档挂了 line-height（1.2/1.4/1.5/1.6），最终评审发现这会改变所有「有 `text-*` 但无就近 `leading-*`」元素的行高——属于真实可见的视觉改动，与本轮承诺不符——因此**删除了那些 line-height，回退为纯字号**（commit `59be9cf`）。

行高标准化本身是合理的（一个完整的 type scale 通常包含行高），但它是一次**有意的视觉改动**，该用 mockup 评审、不该混进「看不出」的那轮。故拆出 1b。

## 目标

把 line-height 纳入字号语义层：每个 `text-*` 档配一个标准行高，消除「有的元素行高靠默认、有的靠手写 `leading-*`」的不一致，同时不破坏现有刻意的紧排版（如单行标签、热力图格子）。

## 现状诊断（起点数据，截至 `20d1833`）

- 字号档（`src/assets/main.css` 的 `@theme`）：`text-title`(20) `text-body`(14) `text-secondary`(12) `text-micro`(10)，**当前均为纯字号、无 line-height**。
- 现有显式 `leading-*` 共 15 处：`leading-none`×8、`leading-[1.6]`×2、`leading-[1.5]`×2、`leading-[1.8]`×1、`leading-[1.7]`×1、`leading-[1.4]`×1。
- 其余有 `text-*` 但无 `leading-*` 的元素，行高来自浏览器默认（约 normal ≈ 1.2–1.5，取决于字体）。
- 重扫命令：`grep -rlE "text-(title|body|secondary|micro)" src/components src/App.vue` 找用字号的文件；`grep -rnE "leading-" src/components src/App.vue` 找已显式设行高的位置。

## 建议方向（待 mockup 验证，非定论）

1. **给每个字号档定一个默认行高**（起点候选，曾用值）：title 1.2、body 1.4、secondary 1.5、micro 1.6。在 `@theme` 里以 `--text-<tier>--line-height` 形式声明即可（Tailwind v4 支持，会随 `text-<tier>` 一起应用）。
2. **保留例外**：单行/紧排场景（`leading-none`、徽章、热力图固定高度格子）仍显式覆盖；定一条规则——「行高跟随字号档；需要紧排时显式 `leading-none`/`leading-[...]` 覆盖，并注释原因」。
3. 评审完后，逐元素决定：哪些去掉手写 `leading-*` 改吃档默认，哪些保留覆盖。

## 过程要求

- 走 `brainstorming` skill + **visual companion**（用户偏好可视化 mockup，且 mockup 要用**真实数据密度**：多行 entry、填充的 commitments 卡，别用单元素——见 memory `prefers-visual-mockups-for-ui`）。
- 重点对比面：day view（标题 + 多行 entry + chips）、commitments 卡、popover 列表、热力图。逐面看「改前/改后」行高。
- 落地仍走本仓库已有治理：迁移后 `npm run verify`；行高若也要强制，考虑扩展 `src/__tests__/tailwind-token-usage.test.ts`（目前 guard 不管行高）。

## 风险

- **与现有 `leading-*` 冲突**：档默认行高与元素显式 `leading-*` 同时存在时，按 Tailwind 生成顺序决胜；需逐处确认覆盖关系符合预期（这正是要 mockup 的原因）。
- **多行文本被撑高**：secondary/micro 配较大行高（1.5/1.6）会让多行 chip/说明文字变高，密度下尤其明显。
- **固定高度容器**：热力图格子等用 flex 居中、高度固定，行高基本无影响，但要确认不溢出。

## 验收（建议）

- 每个字号档有明确默认行高；元素要么吃默认、要么显式覆盖且有注释，无「靠浏览器默认」的隐式状态。
- 关键面 mockup 评审通过。
- `npm run verify` 绿；若行高纳入 guard，护栏更新并通过。
