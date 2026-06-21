# 行高标准化（line-height standardization）「1b」Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 给四个字号档各挂默认行高，删除 7 处数值 `leading-*` 让其继承档默认，保留 8 处 `leading-none` 图标覆盖，并扩展护栏禁止散装行高。

**Architecture:** Tailwind v4 `@theme` 里 `--text-<tier>--line-height` 让 `text-<tier>` 工具类同时输出 font-size + line-height；元素上显式的 `leading-*`（生成顺序在后）仍胜出。迁移后除 8 处 `leading-none` 外，所有文本元素继承档行高。护栏沿用仓库已有的「allowlist 收缩到空」范式，保证每个 commit 都绿。

**Tech Stack:** Vue 3 + TypeScript + Tailwind CSS v4 + Vitest。`npm run verify` = `vitest run && vue-tsc --noEmit && vite build`。

**Spec:** `docs/superpowers/specs/2026-06-21-line-height-standardization-design.md`

**与 spec 的一处偏离**：spec 写「每处 `leading-none` 补注释」。本计划改为：`leading-none` 是护栏放行的合法紧排覆盖（非破例），8 处均为自明单字形，故不逐处加 HTML 注释，仅在 CLAUDE.md 记一次约定（Task 7）。如需逐处注释，执行时告知。

---

## File Structure

| 文件 | 责任 | 改动 |
|---|---|---|
| `src/assets/main.css` | 字号档定义（`@theme`）| 四档各加 `--text-<tier>--line-height`；更新注释 |
| `src/__tests__/tailwind-token-usage.test.ts` | token 护栏 | 加 `leadingViolations` + 两条测试 + `LEADING_ALLOWLIST` |
| `src/components/EntryComposer.vue` | 录入器 | 删 3 处数值 leading（保留 2 处 none）|
| `src/components/composite/EntryRow.vue` | entry 行 | 删 2 处数值 leading（保留 1 处 none）|
| `src/components/MonthView.vue` | 月视图 | 删 1 处数值 leading |
| `src/components/composite/CommitmentsModal.vue` | commitments 弹窗 | 删 1 处数值 leading |
| `CLAUDE.md` | 项目规则 | 「设计 token」段加行高约定 |

`leading-none` 不动的文件（仅供核对，本计划不改）：`DimensionPopover.vue:202`、`DayHeader.vue:33,40`、`composite/EntryRowEdit.vue:166`、`composite/EntryRow.vue:80`、`base/Toast.vue:34`、`EntryComposer.vue:156,180`。

---

## Task 1: 给字号档挂默认行高

**Files:**
- Modify: `src/assets/main.css:10-18`

- [ ] **Step 1: 改 `@theme` 注释**

把第 10-14 行的块注释中这句：

```
     Font-size only (line-height stays with each element's leading-* class, matching prior behavior). */
```

改为：

```
     Each tier carries a default line-height (1b round). Elements inherit it via text-<tier>;
     a tight override uses leading-none (the only sanctioned explicit leading). */
```

- [ ] **Step 2: 给四档各加 line-height**

把：

```css
  --text-title: 20px;
  --text-body: 14px;
  --text-secondary: 12px;
  --text-micro: 10px;
```

改为：

```css
  --text-title: 20px;
  --text-title--line-height: 1.2;
  --text-body: 14px;
  --text-body--line-height: 1.4;
  --text-secondary: 12px;
  --text-secondary--line-height: 1.5;
  --text-micro: 10px;
  --text-micro--line-height: 1.6;
```

- [ ] **Step 3: build 确认编译通过**

Run: `npm run build`
Expected: PASS（vue-tsc + vite build 均成功，无 CSS 报错）。

- [ ] **Step 4: 坐实「leading-none 仍胜出」假设（关键）**

Run: `npm run dev`，浏览器打开 day view。
检查 DayHeader 的 `←` / `→` 导航键、entry 行的 `⋯` 编辑键（均为 `leading-none text-body`）。
Expected: 这些图标的行盒**没有被撑高**、垂直位置不变 —— 证明显式 `leading-none` 覆盖了 body 档的 1.4。
若被撑高（假设不成立）：停止，改用更高特异度的紧排方案，并回报。完成后 `Ctrl-C` 退出 dev。

- [ ] **Step 5: Commit**

```bash
git add src/assets/main.css
git commit -m "feat(type): add default line-height to the four type tiers (1b)"
```

---

## Task 2: 扩展护栏 —— 行高检查（红→绿 via allowlist）

**Files:**
- Modify: `src/__tests__/tailwind-token-usage.test.ts`

- [ ] **Step 1: 加 `leadingViolations` 函数**

在 `fontViolations` 函数之后（约第 73 行后）插入：

```ts
function leadingViolations(src: string): string[] {
  const out: string[] = [];
  // 任意行高: leading-[1.4] / leading-[1.8] / leading-[2rem] …（leading-none 不在此列）
  const arb = /\bleading-\[[^\]]+\]/g;
  for (const m of src.matchAll(arb)) {
    out.push(`${m[0]} → 行高跟随字号档；需紧排用 leading-none（破例需注释 + 显式豁免）`);
  }
  // Tailwind 数字档: leading-6 / leading-7
  const num = /\bleading-\d+\b/g;
  for (const m of src.matchAll(num)) {
    out.push(`${m[0]} → 行高跟随字号档；需紧排用 leading-none`);
  }
  // 具名非 none 档: leading-tight/snug/normal/relaxed/loose
  const named = /\bleading-(tight|snug|normal|relaxed|loose)\b/g;
  for (const m of src.matchAll(named)) {
    out.push(`${m[0]} → 行高跟随字号档；需紧排用 leading-none`);
  }
  return out;
}
```

- [ ] **Step 2: 加测试（先不加 allowlist，制造红态以捕获精确路径 key）**

在 `describe` 块内、`"has no stale allowlist entries"` 测试之前插入：

```ts
  it("uses only the sanctioned leading utility (leading-none); shrinking to empty", () => {
    const offenders: string[] = [];
    for (const [path, src] of Object.entries(allFiles)) {
      if (LEADING_ALLOWLIST.has(path)) continue;
      const v = leadingViolations(src);
      if (v.length) offenders.push(`${path}:\n  ${v.join("\n  ")}`);
    }
    expect(offenders).toEqual([]);
  });
```

并在文件顶部 `ALLOWLIST` 定义（约第 13 行）之后加一个**空**的 leading allowlist：

```ts
const LEADING_ALLOWLIST = new Set<string>([]);
```

- [ ] **Step 3: 跑测试，确认红，并记录精确路径 key**

Run: `npx vitest run src/__tests__/tailwind-token-usage.test.ts`
Expected: FAIL。offenders 里会列出 4 个文件的精确 glob key（形如 `../components/EntryComposer.vue`、`../components/composite/EntryRow.vue`、`../components/MonthView.vue`、`../components/composite/CommitmentsModal.vue`）。**照抄这 4 个 key**用于下一步。

- [ ] **Step 4: 用上一步的精确 key 填充 allowlist**

把 `LEADING_ALLOWLIST` 改为（key 以 Step 3 实际输出为准）：

```ts
const LEADING_ALLOWLIST = new Set<string>([
  "../components/EntryComposer.vue",
  "../components/composite/EntryRow.vue",
  "../components/MonthView.vue",
  "../components/composite/CommitmentsModal.vue",
]);
```

- [ ] **Step 5: 让「stale allowlist」检查也覆盖 leading**

把现有 `"has no stale allowlist entries"` 测试（约第 106-115 行）替换为同时检查两个 allowlist：

```ts
  it("has no stale allowlist entries (migrated files must be removed)", () => {
    const stale: string[] = [];
    for (const path of ALLOWLIST) {
      const src = allFiles[path];
      if (src && spacingViolations(src).length === 0 && fontViolations(src).length === 0) {
        stale.push(path);
      }
    }
    for (const path of LEADING_ALLOWLIST) {
      const src = allFiles[path];
      if (src && leadingViolations(src).length === 0) {
        stale.push(`${path} (leading)`);
      }
    }
    expect(stale).toEqual([]);
  });
```

- [ ] **Step 6: 跑测试，确认绿**

Run: `npx vitest run src/__tests__/tailwind-token-usage.test.ts`
Expected: PASS（4 个文件被 allowlist 豁免，stale 检查通过——它们仍含违规故不算 stale）。

- [ ] **Step 7: Commit**

```bash
git add src/__tests__/tailwind-token-usage.test.ts
git commit -m "test(guard): forbid arbitrary line-height; only leading-none allowed (allowlist seeded)"
```

---

## Task 3: 迁移 EntryComposer（删 3 处数值 leading）

**Files:**
- Modify: `src/components/EntryComposer.vue:163,176,186`
- Modify: `src/__tests__/tailwind-token-usage.test.ts`（移除 allowlist 项）

保留不动：`:156` `+` 字形、`:180` `×` 删除（均 `leading-none`）。

- [ ] **Step 1: 删单行输入的 leading-[1.5]（→ 继承 body 1.4）**

`:161-164` 的 class 中，把：

```
                 caret-[var(--color-brand-solid)] leading-[1.5] py-2xs"
```

改为：

```
                 caret-[var(--color-brand-solid)] py-2xs"
```

- [ ] **Step 2: 删 dim-token 的 leading-[1.6]（→ 继承 micro 1.6）**

`:176` 把：

```
          class="text-micro font-medium px-sm py-2xs rounded-[var(--radius-sm)] inline-flex items-center gap-xs leading-[1.6]"
```

改为：

```
          class="text-micro font-medium px-sm py-2xs rounded-[var(--radius-sm)] inline-flex items-center gap-xs"
```

- [ ] **Step 3: 删 dur-token 的 leading-[1.6]（→ 继承 micro 1.6）**

`:186` 把：

```
          class="mono text-micro font-medium px-sm py-2xs rounded-[var(--radius-sm)] inline-flex items-center gap-xs leading-[1.6]
```

改为：

```
          class="mono text-micro font-medium px-sm py-2xs rounded-[var(--radius-sm)] inline-flex items-center gap-xs
```

- [ ] **Step 4: 从 LEADING_ALLOWLIST 移除 EntryComposer**

在 `src/__tests__/tailwind-token-usage.test.ts` 的 `LEADING_ALLOWLIST` 中删除 `"../components/EntryComposer.vue"` 那一行。

- [ ] **Step 5: 跑测试，确认绿**

Run: `npx vitest run src/__tests__/tailwind-token-usage.test.ts`
Expected: PASS（EntryComposer 已无数值 leading，移出 allowlist 后既不违规也不 stale）。

- [ ] **Step 6: Commit**

```bash
git add src/components/EntryComposer.vue src/__tests__/tailwind-token-usage.test.ts
git commit -m "refactor(composer): drop arbitrary leading, inherit tier defaults (1b)"
```

---

## Task 4: 迁移 EntryRow（删 2 处数值 leading）

**Files:**
- Modify: `src/components/composite/EntryRow.vue:63,69`
- Modify: `src/__tests__/tailwind-token-usage.test.ts`

保留不动：`:80` `⋯` 编辑键（`leading-none`）。

- [ ] **Step 1: 删正文的 leading-[1.4]（→ 继承 body 1.4，逐像素相同）**

`:63` 把：

```
        class="text-body font-medium text-[var(--color-text-primary)] leading-[1.4] break-words overflow-hidden [display:-webkit-box] [-webkit-line-clamp:2] [-webkit-box-orient:vertical]"
```

改为：

```
        class="text-body font-medium text-[var(--color-text-primary)] break-words overflow-hidden [display:-webkit-box] [-webkit-line-clamp:2] [-webkit-box-orient:vertical]"
```

- [ ] **Step 2: 删 chip 的 leading-[1.7]（→ 继承 micro 1.6）**

`:69` 把：

```
          class="text-micro font-[450] px-sm rounded-[var(--radius-sm)] leading-[1.7] max-w-[100px] overflow-hidden text-ellipsis whitespace-nowrap"
```

改为：

```
          class="text-micro font-[450] px-sm rounded-[var(--radius-sm)] max-w-[100px] overflow-hidden text-ellipsis whitespace-nowrap"
```

- [ ] **Step 3: 从 LEADING_ALLOWLIST 移除 EntryRow**

删除 `"../components/composite/EntryRow.vue"` 那一行。

- [ ] **Step 4: 跑测试，确认绿**

Run: `npx vitest run src/__tests__/tailwind-token-usage.test.ts`
Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/EntryRow.vue src/__tests__/tailwind-token-usage.test.ts
git commit -m "refactor(entry-row): drop arbitrary leading, inherit tier defaults (1b)"
```

---

## Task 5: 迁移 MonthView（删 1 处数值 leading）

**Files:**
- Modify: `src/components/MonthView.vue:348`
- Modify: `src/__tests__/tailwind-token-usage.test.ts`

- [ ] **Step 1: 删备注框的 leading-[1.5]（→ 继承 secondary 1.5，逐像素相同）**

`:348` 把：

```
          class="text-secondary italic text-[var(--color-text-secondary)] leading-[1.5] cursor-text px-sm py-sm rounded-[var(--radius-form-lg)] outline-none hover:bg-[var(--color-page-bg)]"
```

改为：

```
          class="text-secondary italic text-[var(--color-text-secondary)] cursor-text px-sm py-sm rounded-[var(--radius-form-lg)] outline-none hover:bg-[var(--color-page-bg)]"
```

- [ ] **Step 2: 从 LEADING_ALLOWLIST 移除 MonthView**

删除 `"../components/MonthView.vue"` 那一行。

- [ ] **Step 3: 跑测试，确认绿**

Run: `npx vitest run src/__tests__/tailwind-token-usage.test.ts`
Expected: PASS。

- [ ] **Step 4: Commit**

```bash
git add src/components/MonthView.vue src/__tests__/tailwind-token-usage.test.ts
git commit -m "refactor(month-view): drop arbitrary leading, inherit tier default (1b)"
```

---

## Task 6: 迁移 CommitmentsModal（删 1 处数值 leading，allowlist 清空）

**Files:**
- Modify: `src/components/composite/CommitmentsModal.vue:155`
- Modify: `src/__tests__/tailwind-token-usage.test.ts`

- [ ] **Step 1: 删摘要的 leading-[1.8]（→ 继承 secondary 1.5，两行收紧，已确认接受）**

`:155` 把：

```
          <div class="text-right text-secondary text-[var(--color-text-muted)] leading-[1.8]">
```

改为：

```
          <div class="text-right text-secondary text-[var(--color-text-muted)]">
```

- [ ] **Step 2: 从 LEADING_ALLOWLIST 移除 CommitmentsModal（此时 allowlist 应为空 `new Set<string>([])`）**

删除 `"../components/composite/CommitmentsModal.vue"` 那一行。`LEADING_ALLOWLIST` 现应回到空集。

- [ ] **Step 3: 跑测试，确认绿**

Run: `npx vitest run src/__tests__/tailwind-token-usage.test.ts`
Expected: PASS（leading allowlist 已空，全仓数值 leading 清零，stale 检查通过）。

- [ ] **Step 4: Commit**

```bash
git add src/components/composite/CommitmentsModal.vue src/__tests__/tailwind-token-usage.test.ts
git commit -m "refactor(commitments-modal): drop arbitrary leading, inherit tier default (1b)"
```

---

## Task 7: CLAUDE.md 约定 + 全量验证

**Files:**
- Modify: `CLAUDE.md`（「设计 token」段）

- [ ] **Step 1: 在「设计 token」段补行高约定**

找到「设计 token」段中描述字号的句子（`字号走 text-title/body/secondary/micro …`）之后，补一句：

```
行高跟随字号档（@theme 里 --text-<tier>--line-height）：元素继承档行高，需紧排时用 leading-none（唯一合法显式覆盖）；禁止散装 leading-[...] / leading-<number> / leading-tight 等。guard 同步在 tailwind-token-usage.test.ts 强制。
```

- [ ] **Step 2: 全仓核对 —— 只剩 8 处 leading-none**

Run: `grep -rnE "leading-" src/components src/App.vue`
Expected: 恰好 8 行，全部是 `leading-none`（`DimensionPopover:202`、`EntryComposer:156,180`、`DayHeader:33,40`、`EntryRowEdit:166`、`EntryRow:80`、`Toast:34`）。无任何 `leading-[...]` / `leading-<number>`。

- [ ] **Step 3: 全量 verify**

Run: `npm run verify`
Expected: PASS（vitest 全绿含行高 guard；vue-tsc 无类型错误；vite build 成功）。

- [ ] **Step 4: 关键面人工比对**

Run: `npm run dev`，逐面核对与本轮 mockup 一致：
- day view：标题/Today 徽章/count·total/时长/commitments 行行高自然，图标未被撑高。
- commitments 弹窗头部：右侧 Committed/Logged 两行较此前收紧（1.8→1.5），可接受。
- popover 底部提示、error banner 多行说明：行距与 mockup 一致。
- 热力图格子：固定高度，文字未溢出。
完成后 `Ctrl-C` 退出。

- [ ] **Step 5: Commit**

```bash
git add CLAUDE.md
git commit -m "docs(rules): line-height follows type tier; only leading-none allowed (1b)"
```

---

## Self-Review（已对照 spec）

- **Spec §1 @theme** → Task 1。✓
- **Spec §2 覆盖处置（留 8 / 折 7）** → Task 3-6 删 7 处数值 leading；8 处 leading-none 全程不动（Task 7 Step 2 核对）。✓
- **Spec §3 护栏** → Task 2 加 leadingViolations + 测试，禁任意/数字/具名行高，仅放行 leading-none。✓
- **Spec §4 CLAUDE.md** → Task 7 Step 1。✓
- **Spec 验证（verify / build 假设 / 人工比对）** → Task 1 Step 4（leading-none 假设）、Task 7 Step 3-4。✓
- **Spec 验收（仅余 8 处 leading-none）** → Task 7 Step 2。✓
- **类型/命名一致性**：`leadingViolations` / `LEADING_ALLOWLIST` 在 Task 2 定义，Task 3-6 一致引用。✓
- **占位符**：无 TBD/TODO；每个改动步骤均含精确 old→new 文本。✓
