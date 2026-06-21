# Commitments Follow Selected Month — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `store.commitments` follow the selected month so切月后 Commitments 弹层显示目标月真实数据（新月=空），并消除「在错月保存写错文件」的数据正确性 bug。

**Architecture:** 方案 A —— `store.commitments` 语义从「启动时的真实日历月」改为「当前选中月」，与 `store.commitmentProgress` 同生命周期。`loadMonth` 成为月级状态（entries/progress/commitments）的唯一加载点；`App.initApp` 不再写 commitments；`commitments-changed` 文件事件改为重读选中月，不再经 `initApp` 冲回当前月。后端零改动。

**Tech Stack:** Vue 3 (`<script setup>`) + TypeScript + Tauri 2 IPC（`invoke`）；测试 vitest + @vue/test-utils。

参考 spec：`docs/superpowers/specs/2026-06-21-commitments-follow-selected-month-design.md`

---

## 文件结构

涉及文件（全部已存在，无新建）：

- 修改 `src/components/MonthView.vue` —— `loadMonth` 增加按选中月加载 commitments；新增 `loadCommitments` 辅助函数（镜像现有 `loadCommitments` 同级的 `loadCommitmentProgress`）。
- 修改 `src/App.vue` —— `initApp` 的 `Ready` 分支删除 `store.commitments` 写入；`commitments-changed` 监听器改为重读选中月 commitments + progress；新增 import。
- 修改测试 `src/__tests__/components/MonthView.test.ts` —— `beforeEach` 路由 `get_commitments`；新增「切月后 store.commitments 跟随目标月」测试。
- 修改测试 `src/__tests__/components/App.test.ts` —— 重写既有「commitments-changed event calls initApp」测试（该断言在本次改动后失效）；新增「不冲掉选中月」「initApp 不再覆盖 commitments」两条。

后端 `get_commitments(root_path, year, month)`（`src-tauri/src/commands.rs:490`）已正确按月读取、月文件缺失返回 `vec![]`，**不改**。

---

## Task 1: MonthView — `loadMonth` 按选中月加载 commitments

**Files:**
- Test: `src/__tests__/components/MonthView.test.ts`（修改 `beforeEach` + 新增一条 `it`）
- Modify: `src/components/MonthView.vue:42-73`

- [ ] **Step 1: 改测试 mock 路由 + 写失败测试**

先在 `src/__tests__/components/MonthView.test.ts` 的 `beforeEach`（当前 43-50 行）里给 `get_commitments` 加路由，避免它落到默认分支返回 `{note,entries}` 对象污染 `store.commitments`。把 `beforeEach` 改成：

```ts
beforeEach(() => {
  invokeMock.mockReset();
  // Route by command so progress/commitments always return arrays
  invokeMock.mockImplementation(async (cmd: string) => {
    if (cmd === "get_commitment_progress") return [];
    if (cmd === "get_commitments") return [];
    return { note: null, entries: [] };
  });
});
```

然后在 `describe("MonthView", ...)` 内新增测试（放在文件末尾 `⌘T` 测试之后）：

```ts
it("loadMonth loads commitments for the SELECTED month (not the launch month)", async () => {
  const store = makeStore(); // currentDate === today, commitments = [一个 commitment]
  const curYM = todayDateStr().slice(0, 7); // 当前 YYYY-MM

  // 当前月有 commitments；其它任何月为空（模拟「目标月还没建 commitments」）
  invokeMock.mockImplementation(async (cmd: string, args: { year: number; month: number }) => {
    if (cmd === "get_commitment_progress") return [];
    if (cmd === "get_commitments") {
      const ym = `${args.year}-${String(args.month).padStart(2, "0")}`;
      return ym === curYM ? [makeCommitment({ role: "Dev", goals: ["G"] })] : [];
    }
    return { note: null, entries: [] };
  });

  const wrapper = mountView(store);
  await flushPromises(); // 等 onMounted 的 loadMonth(当前月) 跑完
  expect(store.commitments).toHaveLength(1); // 当前月：有数据

  // 切到上一个月（⌘⇧[）—— 该月没有 commitments
  window.dispatchEvent(new KeyboardEvent("keydown", { key: "[", metaKey: true, shiftKey: true }));
  await flushPromises();

  expect(store.commitments).toEqual([]); // 跟随目标月：空，而非停留在当前月数据
});
```

并在文件顶部 import 里加入 `flushPromises`（与 `mount` 同一行）：

```ts
import { mount, flushPromises } from "@vue/test-utils";
```

- [ ] **Step 2: 跑测试，确认失败**

Run: `pnpm vitest run src/__tests__/components/MonthView.test.ts -t "loadMonth loads commitments"`
Expected: FAIL —— 改动前 `loadMonth` 不读 commitments，切月后 `store.commitments` 仍为初始的 `[makeCommitment]`，断言 `toEqual([])` 不通过。

- [ ] **Step 3: 实现 —— 新增 `loadCommitments` 并在 `loadMonth` 调用**

在 `src/components/MonthView.vue` 中，于 `loadCommitmentProgress`（当前 69-73 行）之后新增 `loadCommitments`：

```ts
async function loadCommitments(year: number, month: number) {
  try {
    store.commitments = (await invoke("get_commitments", { rootPath: store.rootPath, year, month })) as Commitment[];
  } catch (e) { logError("MonthView.loadCommitments", e); store.commitments = []; }
}
```

在 `loadMonth` 里，紧跟 `await loadCommitmentProgress(year, month);`（当前第 62 行）之后加一行：

```ts
  store.monthEntries = map;
  await loadCommitmentProgress(year, month);
  await loadCommitments(year, month);
  if (store.currentDate in map) {
    store.today = { note: null, entries: map[store.currentDate] };
    loadDayNote(store.currentDate);
  }
```

`Commitment` 类型已在 MonthView 顶部 import（第 11 行），无需新增 import。

- [ ] **Step 4: 跑测试，确认通过**

Run: `pnpm vitest run src/__tests__/components/MonthView.test.ts`
Expected: PASS（新测试通过，且原有 MonthView 测试因 `beforeEach` 已路由 `get_commitments` 不受影响）。

- [ ] **Step 5: Commit**

```bash
git add src/components/MonthView.vue src/__tests__/components/MonthView.test.ts
git commit -m "fix(commitments): loadMonth loads commitments for selected month"
```

---

## Task 2: App.vue — initApp 不再写 commitments；commitments-changed 重读选中月

**Files:**
- Test: `src/__tests__/components/App.test.ts`（重写 1 条，新增 2 条）
- Modify: `src/App.vue`（imports；`commitments-changed` 监听器约 47-49 行；`Ready` 分支约 97 行）

- [ ] **Step 1: 重写失效测试 + 写新失败测试**

在 `src/__tests__/components/App.test.ts` 中，**删除**既有的（221-235 行）：

```ts
  it("commitments-changed event calls initApp", async () => {
    mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    vi.clearAllMocks();
    mockInvoke.mockResolvedValue({ status: "NeedsSetup" });

    if (commitmentsChangedCallback) {
      commitmentsChangedCallback();
      await vi.runAllTimersAsync();
    }

    expect(mockInvoke).toHaveBeenCalledWith("init");
  });
```

替换为下面三条测试：

```ts
  it("commitments-changed reloads the SELECTED month's commitments and does NOT call init", async () => {
    vi.setSystemTime(new Date(2026, 5, 20, 10, 0, 0)); // 当前月 = 2026-06
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", config: makeConfig(), today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    store.currentDate = "2026-07-15"; // 用户切到 7 月（无 commitments）
    store.status = "ready";
    store.commitments = [makeCommitment()]; // 模拟此刻残留的「当前月」数据
    vi.clearAllMocks();

    // 按月路由：7 月空，其它月有数据
    mockInvoke.mockImplementation(async (cmd: string, args: { month: number }) => {
      if (cmd === "get_commitments") return args.month === 7 ? [] : [makeCommitment()];
      if (cmd === "get_commitment_progress") return [];
      return undefined;
    });

    commitmentsChangedCallback?.();
    await vi.runAllTimersAsync();

    expect(mockInvoke).not.toHaveBeenCalledWith("init");
    expect(mockInvoke).toHaveBeenCalledWith("get_commitments", expect.objectContaining({ year: 2026, month: 7 }));
    expect(store.commitments).toEqual([]); // 跟随 7 月，未被冲回当前月数据
  });

  it("commitments-changed is a no-op when status is not ready", async () => {
    mockInvoke.mockResolvedValue({ status: "NeedsSetup" });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    store.status = "setup";
    vi.clearAllMocks();

    commitmentsChangedCallback?.();
    await vi.runAllTimersAsync();

    expect(mockInvoke).not.toHaveBeenCalledWith("get_commitments");
  });

  it("initApp (via config-changed) does NOT overwrite the selected month's commitments", async () => {
    vi.setSystemTime(new Date(2026, 5, 20, 10, 0, 0));
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", config: makeConfig(), today: makeDayFile(), commitments: [makeCommitment()], scan_warnings: [] },
    });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    store.currentDate = "2026-07-15";
    const sentinel = [makeCommitment({ role: "JulyOnly" })];
    store.commitments = sentinel; // 选中月（7 月）当前持有的数据
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", config: makeConfig(), today: makeDayFile(), commitments: [makeCommitment()], scan_warnings: [] },
    });

    configChangedCallback?.({ payload: [] }); // 触发 initApp
    await vi.runAllTimersAsync();

    expect(store.commitments).toBe(sentinel); // initApp 不再写 commitments，引用未变
  });
```

- [ ] **Step 2: 跑测试，确认失败**

Run: `pnpm vitest run src/__tests__/components/App.test.ts -t "commitments-changed"`
Expected: FAIL —— 改动前 `commitments-changed` 调 `initApp`，会 `invoke("init")` 而非 `get_commitments`，且不会按 7 月重读；「initApp 不覆盖 commitments」一条也会失败（改动前 initApp 写 `data.commitments`）。

- [ ] **Step 3: 实现 App.vue 三处改动**

1）顶部 import（当前 2-11 行附近）新增。在 `import { useStore } ...` 上下补一行 dates import，并把类型 import 行补上 `Commitment, CommitmentProgress`：

```ts
import { useStore } from "./stores/useStore";
import { yearMonthFromDate } from "./utils/dates";
```

```ts
import type { InitResult, ConfigErrorDetail, ScanWarning, Commitment, CommitmentProgress } from "./types";
```

2）`commitments-changed` 监听器（当前 47-49 行）替换为重读选中月：

```ts
    unlistenCommitments = await listen<ConfigErrorDetail[]>("commitments-changed", async () => {
      if (store.status !== "ready") return;
      const { year, month } = yearMonthFromDate(store.currentDate);
      try {
        store.commitments = (await invoke("get_commitments", { rootPath: store.rootPath, year, month })) as Commitment[];
        store.commitmentProgress = (await invoke("get_commitment_progress", { rootPath: store.rootPath, year, month })) as CommitmentProgress[];
      } catch (e) {
        logError("App.commitmentsChanged", e);
      }
    });
```

3）`initApp` 的 `Ready` 分支（当前 93-103 行）删除写 commitments 的那一行：

```ts
      case "Ready":
        store.rootPath = result.data.root_path;
        store.config = result.data.config;
        store.today = result.data.today;
        store.status = "ready";
        if (result.data.scan_warnings.length > 0) {
          scanWarnings.value = result.data.scan_warnings;
          showScanWarning.value = true;
        }
        break;
```

（即删掉原 `store.commitments = result.data.commitments;` 一行。启动时的 commitments 由 `MonthView.onMounted → loadMonth` 统一加载。）

- [ ] **Step 4: 跑测试，确认通过**

Run: `pnpm vitest run src/__tests__/components/App.test.ts`
Expected: PASS（三条新测试通过；其余 App 测试不受影响——它们未断言 `store.commitments`，midnight-follow 仍调 `init`）。

- [ ] **Step 5: Commit**

```bash
git add src/App.vue src/__tests__/components/App.test.ts
git commit -m "fix(commitments): commitments-changed reloads selected month; initApp drops commitments write"
```

---

## Task 3: 全量回归 + 类型护栏

**Files:** 无（仅运行验证）

- [ ] **Step 1: 前端全量测试**

Run: `pnpm test`
Expected: 全绿。基线 26 files / 296 tests，本次新增 4 条（MonthView +1，App +3，净 +3 因删 1 旧测试）→ 约 299 pass，0 fail。

- [ ] **Step 2: 类型 + 构建护栏**

Run: `pnpm run build`
Expected: 成功。`vue-tsc` 会对测试文件做严格类型检查（`noUnusedLocals`）——确认新增 import（`flushPromises`、`yearMonthFromDate`、`Commitment`、`CommitmentProgress`）全部被使用，无未用变量。（vitest 绿 ≠ build 绿，此步不可省。）

- [ ] **Step 3: 后端零回归 sanity**

Run: `cd src-tauri && cargo test`
Expected: 153 pass，与基线一致（后端未改，应零变化）。

- [ ] **Step 4: 手动冒烟（可选，若环境允许 GUI）**

Run: `pnpm tauri dev`（仓库根目录）
验证：当前月配好 1 个 role+goal → ⌘⇧[ 切到无 commitments 的月 → 打开 CommitmentsPanel 的 Edit → 弹层应为空（非当前月数据）→ 保存空 → 检查目标月 `_monthly.md` 未被写入当前月数据。

- [ ] **Step 5: Commit（如有遗留改动）**

```bash
git status # 预期 clean；若有格式化等改动则提交
```

---

## Self-Review

**Spec coverage：**
- §2 决策 A（store.commitments 跟随选中月）→ Task 1（loadMonth）+ Task 2（initApp/listener）覆盖。
- §3 改动 1（loadMonth）→ Task 1；改动 2（initApp 不写）→ Task 2 Step 3.3 + 「initApp 不覆盖」测试；改动 3（commitments-changed 重读选中月）→ Task 2 Step 3.2 + 两条 listener 测试。
- §4 错误处理（try/catch + logError + `[]`）→ Task 1 `loadCommitments`、Task 2 监听器 catch。状态守卫（`status === "ready"`）→ Task 2「no-op when not ready」测试。
- §5 测试 1（种子按选中月）→ Task 1 测试；测试 2（commitments-changed 不冲掉）→ Task 2 第一条测试。回归基线 → Task 3。
- §6 非目标（继承、后端、文档漂移）→ 计划未触及，符合。

**Placeholder scan：** 无 TBD/TODO；每个改代码的 step 均含完整代码块与确切命令。

**Type consistency：** `loadCommitments(year, month)` 签名与 `loadCommitmentProgress(year, month)` 一致；`store.commitments: Commitment[]`、`store.commitmentProgress: CommitmentProgress[]`（见 `useStore.ts:16-17`）与监听器断言类型一致；fixture `makeCommitment` 返回 `Commitment`（`fixtures.ts:59`）。`yearMonthFromDate` 返回 `{ year, month }`（MonthView 已同样用法）。
