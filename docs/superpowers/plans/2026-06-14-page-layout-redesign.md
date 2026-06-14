# 页面布局改版 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将主窗口布局从 Day/Week/Month 三粒度模式改为固定月视图模式——左栏月概览 + 右栏日内详情。

**Architecture:** MonthView 替代 TodayView 作为主视图容器。左栏 MonthNavigator + CommitmentsPanel，右栏 DayStrip + DayNote（内联）+ QuickEntry + EntryList。granularity 概念完全移除，EntryList 始终以 day 模式渲染。新增 Rust 命令 `get_available_months` 支持懒加载月份列表。

**Tech Stack:** Vue 3 + TypeScript + Tailwind CSS + Tauri 2.x (Rust)

---

## File Structure

### New Files
| File | Responsibility |
|------|---------------|
| `src/components/MonthView.vue` | 主视图容器，左右两栏布局，协调数据加载 |
| `src/components/MonthNavigator.vue` | 月份切换 ← → 箭头 + 快速跳转双下拉 |
| `src/components/DayStrip.vue` | 横向滚动 1-31 日期条，圆点标记、周分组间距、未来灰显 |
| `src/__tests__/components/MonthView.test.ts` | MonthView 集成测试 |
| `src/__tests__/components/MonthNavigator.test.ts` | MonthNavigator 单元测试 |
| `src/__tests__/components/DayStrip.test.ts` | DayStrip 单元测试 |

### Modified Files
| File | Change |
|------|--------|
| `src/App.vue` | 导入 MonthView 替代 TodayView，移除 loadCommitmentProgress |
| `src/stores/useStore.ts` | 移除 `granularity`/`periodEntries`，新增 `monthEntries`/`availableMonths` |
| `src/types.ts` | 移除 `Granularity` |
| `src/utils/dates.ts` | 移除 `weekLabel`，简化 `datesInPeriod` 为仅 month 分支 |
| `src/components/EntryList.vue` | 移除 `granularity`/`periodEntries` props，始终 day 模式，底部内联合计行 |
| `src-tauri/src/commands.rs` | 新增 `get_available_months` 命令 |
| `src-tauri/src/lib.rs` | 注册 `get_available_months` |
| `src-tauri/src/models.rs` | 新增 `AvailableMonth` 结构体 |
| `src/__tests__/components/App.test.ts` | TodayView → MonthView |
| `src/__tests__/components/EntryList.test.ts` | 移除 granularity 相关测试，改用新 props，验证内联合计行 |
| `src/__tests__/dates.test.ts` | 移除 week/month/weekLabel 测试 |

### Deleted Files
| File | Reason |
|------|--------|
| `src/components/TodayView.vue` | 由 MonthView.vue 替代 |
| `src/components/DateNavigator.vue` | 拆分为 MonthNavigator + DayStrip + DayNote |
| `src/components/SummaryBar.vue` | 由 EntryList 内联合计行替代 |
| `src/__tests__/components/TodayView.test.ts` | 由 MonthView.test.ts 替代 |
| `src/__tests__/components/DateNavigator.test.ts` | 由 MonthNavigator.test.ts + DayStrip.test.ts 替代 |
| `src/__tests__/components/SummaryBar.test.ts` | 功能合并到 EntryList.test.ts |

---

### Task 1: Rust — 新增 AvailableMonth 模型和 get_available_months 命令

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 models.rs 中添加 AvailableMonth 结构体**

`src-tauri/src/models.rs` 文件末尾添加：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableMonth {
    pub year: i32,
    pub month: u32,
}
```

- [ ] **Step 2: 在 commands.rs 添加 get_available_months 命令**

`src-tauri/src/commands.rs` 的 `open_in_editor` 命令之后（`log_error` 之前）添加：

```rust
#[tauri::command]
pub fn get_available_months(root_path: String) -> Result<Vec<AvailableMonth>, String> {
    use crate::models::AvailableMonth;
    let root = std::path::Path::new(&root_path);
    if !root.exists() {
        return Ok(vec![]);
    }

    let mut months: Vec<AvailableMonth> = Vec::new();

    let year_entries = std::fs::read_dir(root)
        .map_err(|e| format!("Failed to read root dir: {}", e))?;

    for year_entry in year_entries.flatten() {
        if !year_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let year_name = year_entry.file_name();
        let year_str = year_name.to_string_lossy();
        let year: i32 = match year_str.parse() {
            Ok(y) if y >= 2000 && y <= 2100 => y,
            _ => continue,
        };

        let month_entries = match std::fs::read_dir(year_entry.path()) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for month_entry in month_entries.flatten() {
            if !month_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let month_name = month_entry.file_name();
            let month_str = month_name.to_string_lossy();
            let month: u32 = match month_str.parse() {
                Ok(m) if m >= 1 && m <= 12 => m,
                _ => continue,
            };

            // Check if this month directory contains at least one .md file
            let has_md = match std::fs::read_dir(month_entry.path()) {
                Ok(entries) => entries.flatten().any(|e| {
                    e.file_name().to_string_lossy().ends_with(".md")
                }),
                Err(_) => false,
            };

            if has_md {
                months.push(AvailableMonth { year, month });
            }
        }
    }

    // Sort descending (newest first)
    months.sort_by(|a, b| b.year.cmp(&a.year).then(b.month.cmp(&a.month)));

    Ok(months)
}
```

- [ ] **Step 3: 在 lib.rs 注册命令**

`src-tauri/src/lib.rs` 的 `invoke_handler` 中添加 `commands::get_available_months,`（放在 `commands::get_commitment_progress,` 之后）：

```rust
.invoke_handler(tauri::generate_handler![
    commands::init,
    commands::set_root_path,
    commands::get_entries,
    commands::append_entry,
    commands::update_entry,
    commands::delete_entry,
    commands::set_day_note,
    commands::get_commitments,
    commands::get_commitment_progress,
    commands::get_available_months,
    commands::open_in_editor,
    commands::create_starter_files,
    commands::log_error,
    commands::log_info,
])
```

- [ ] **Step 4: 编译验证**

```bash
cd src-tauri && cargo check
```

Expected: `Finished` with no errors.

- [ ] **Step 5: 运行 Rust 测试确保无回归**

```bash
cd src-tauri && cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add get_available_months backend command"
```

---

### Task 2: 清理 types.ts 和 dates.ts

**Files:**
- Modify: `src/types.ts`
- Modify: `src/utils/dates.ts`

- [ ] **Step 1: 从 types.ts 移除 Granularity**

`src/types.ts` 文件末尾，删除：

```typescript
export type Granularity = "day" | "week" | "month";
```

- [ ] **Step 2: 简化 dates.ts —— 移除 weekLabel，简化 datesInPeriod**

将 `src/utils/dates.ts` 替换为：

```typescript
export function formatDate(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export function parseDate(dateStr: string): Date {
  return new Date(dateStr + "T00:00:00");
}

/** Return all dates (YYYY-MM-DD) in the month containing dateStr. */
export function datesInMonth(dateStr: string): string[] {
  const d = parseDate(dateStr);
  const year = d.getFullYear();
  const month = d.getMonth();
  const lastDay = new Date(year, month + 1, 0).getDate();
  const dates: string[] = [];
  for (let day = 1; day <= lastDay; day++) {
    dates.push(`${year}-${String(month + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`);
  }
  return dates;
}

/** Return the year and month from a YYYY-MM-DD date string. */
export function yearMonthFromDate(dateStr: string): { year: number; month: number } {
  const parts = dateStr.split("-");
  return { year: parseInt(parts[0], 10), month: parseInt(parts[1], 10) };
}
```

- [ ] **Step 3: 更新 dates.test.ts —— 只保留 formatDate/parseDate/datesInMonth**

将 `src/__tests__/dates.test.ts` 替换为：

```typescript
import { describe, it, expect } from "vitest";
import { formatDate, datesInMonth, parseDate } from "../utils/dates";

describe("formatDate", () => {
  it("formats correctly", () => {
    expect(formatDate(new Date(2026, 5, 12))).toBe("2026-06-12");
  });
});

describe("datesInMonth", () => {
  it("returns correct count for 30-day month", () => {
    expect(datesInMonth("2026-06-12").length).toBe(30);
  });
  it("returns correct count for 31-day month", () => {
    expect(datesInMonth("2026-07-15").length).toBe(31);
  });
  it("returns correct count for February", () => {
    expect(datesInMonth("2026-02-10").length).toBe(28);
  });
  it("first date is the 1st", () => {
    expect(datesInMonth("2026-06-15")[0]).toBe("2026-06-01");
  });
  it("last date is the last day of month", () => {
    const dates = datesInMonth("2026-06-15");
    expect(dates[dates.length - 1]).toBe("2026-06-30");
  });
});

describe("parseDate", () => {
  it("parses ISO date", () => {
    expect(parseDate("2026-06-12").getDate()).toBe(12);
  });
});
```

- [ ] **Step 4: 运行 dates 测试**

```bash
pnpm test -- src/__tests__/dates.test.ts
```

Expected: all 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/types.ts src/utils/dates.ts src/__tests__/dates.test.ts
git commit -m "refactor: remove Granularity type, simplify datesInMonth"
```

---

### Task 3: 清理 Store —— 移除 granularity/periodEntries，新增 monthEntries/availableMonths

**Files:**
- Modify: `src/stores/useStore.ts`

- [ ] **Step 1: 更新 useStore.ts**

将 `src/stores/useStore.ts` 替换为：

```typescript
import { reactive, inject, provide, type InjectionKey } from "vue";
import type { Config, DayFile, Commitment, CommitmentProgress, ConfigErrorDetail, Screen, Entry } from "../types";

export interface AvailableMonth {
  year: number;
  month: number;
}

export interface AppStore {
  screen: Screen;
  rootPath: string;
  config: Config | null;
  configErrors: ConfigErrorDetail[];
  today: DayFile | null;
  commitments: Commitment[];
  commitmentProgress: CommitmentProgress[];
  lastDimensions: Record<string, string>;
  currentDate: string;
  monthEntries: Record<string, Entry[]>;
  availableMonths: AvailableMonth[] | null; // null = not yet loaded
}

export const STORE_KEY: InjectionKey<AppStore> = Symbol("AppStore");

export function createStore(): AppStore {
  const now = new Date();
  const dateStr = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;

  return reactive<AppStore>({
    screen: "loading",
    rootPath: "",
    config: null,
    configErrors: [],
    today: null,
    commitments: [],
    commitmentProgress: [],
    lastDimensions: {},
    currentDate: dateStr,
    monthEntries: {},
    availableMonths: null,
  });
}

export function provideStore(store: AppStore): void {
  provide(STORE_KEY, store);
}

export function useStore(): AppStore {
  const store = inject(STORE_KEY);
  if (!store) throw new Error("AppStore not provided. Call provideStore() in root component.");
  return store;
}
```

- [ ] **Step 2: 更新 store mock**

将 `src/__tests__/mocks/store.ts` 替换为：

```typescript
import { createStore, type AppStore } from "../../stores/useStore";

export function createTestStore(overrides?: Partial<AppStore>): AppStore {
  const store = createStore();
  if (overrides) {
    Object.assign(store, overrides);
  }
  return store;
}
```

内容不变（因为 `createStore` 已经更新了默认值，mock 不需要改）。但需要确认导入仍然正确：

```typescript
import { createStore, type AppStore } from "../../stores/useStore";

export function createTestStore(overrides?: Partial<AppStore>): AppStore {
  const store = createStore();
  if (overrides) {
    Object.assign(store, overrides);
  }
  return store;
}
```

- [ ] **Step 3: 运行 useStore 测试确认无回归**

```bash
pnpm test -- src/__tests__/useStore.test.ts
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/stores/useStore.ts
git commit -m "refactor: remove granularity/periodEntries, add monthEntries/availableMonths to store"
```

---

### Task 4: 创建 DayStrip.vue（含测试）

**Files:**
- Create: `src/components/DayStrip.vue`
- Create: `src/__tests__/components/DayStrip.test.ts`

- [ ] **Step 1: 编写 DayStrip 测试**

`src/__tests__/components/DayStrip.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount } from "@vue/test-utils";
import DayStrip from "../../components/DayStrip.vue";

describe("DayStrip", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // Set "today" to 2026-06-14
    vi.setSystemTime(new Date(2026, 5, 14));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function mountStrip(props: {
    dates: string[];
    selectedDate: string;
    monthEntries: Record<string, unknown[]>;
  }) {
    return mount(DayStrip, { props });
  }

  it("renders correct number of day cells", () => {
    const dates = Array.from({ length: 30 }, (_, i) => `2026-06-${String(i + 1).padStart(2, "0")}`);
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-14", monthEntries: {} });
    const cells = wrapper.findAll("[data-day]");
    expect(cells).toHaveLength(30);
  });

  it("selected date has highlight class", () => {
    const dates = ["2026-06-01", "2026-06-02", "2026-06-03"];
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-02", monthEntries: {} });
    const cells = wrapper.findAll("[data-day]");
    const selected = cells.find(c => c.attributes("data-day") === "2026-06-02");
    expect(selected?.classes()).toContain("bg-blue-600");
  });

  it("today has distinct indicator when not selected", () => {
    const dates = ["2026-06-13", "2026-06-14", "2026-06-15"];
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-13", monthEntries: {} });
    const todayCell = wrapper.find('[data-day="2026-06-14"]');
    // Should have underline or bold class but not the selected blue bg
    expect(todayCell.classes()).not.toContain("bg-blue-600");
    // Should have a "today" class or similar marker
    expect(todayCell.classes()).toContain("font-semibold");
  });

  it("future dates are grey and not clickable", () => {
    const dates = ["2026-06-13", "2026-06-14", "2026-06-15", "2026-06-16"];
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-13", monthEntries: {} });
    const cell15 = wrapper.find('[data-day="2026-06-15"]');
    const cell16 = wrapper.find('[data-day="2026-06-16"]');
    expect(cell15.classes()).toContain("text-gray-300");
    expect(cell16.classes()).toContain("text-gray-300");
  });

  it("future dates emit no event on click", async () => {
    const dates = ["2026-06-14", "2026-06-15"];
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-14", monthEntries: {} });
    const cell15 = wrapper.find('[data-day="2026-06-15"]');
    await cell15.trigger("click");
    expect(wrapper.emitted("selectDay")).toBeFalsy();
  });

  it("clicking a past date emits selectDay", async () => {
    const dates = ["2026-06-10", "2026-06-11"];
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-10", monthEntries: {} });
    const cell = wrapper.find('[data-day="2026-06-11"]');
    await cell.trigger("click");
    expect(wrapper.emitted("selectDay")?.[0]).toEqual(["2026-06-11"]);
  });

  it("days with entries show a blue dot", () => {
    const dates = ["2026-06-01", "2026-06-02"];
    const monthEntries = { "2026-06-01": [{ id: "e1", item: "X", duration: 30, dimensions: {} }] };
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-01", monthEntries });
    const cellWithEntry = wrapper.find('[data-day="2026-06-01"]');
    const cellWithoutEntry = wrapper.find('[data-day="2026-06-02"]');
    // Cell with entry should contain a dot element
    expect(cellWithEntry.find('[data-dot]').exists()).toBe(true);
    // Cell without entry should not
    expect(cellWithoutEntry.find('[data-dot]').exists()).toBe(false);
  });

  it("every 7th cell has wider right margin", () => {
    const dates = Array.from({ length: 14 }, (_, i) => `2026-06-${String(i + 1).padStart(2, "0")}`);
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-01", monthEntries: {} });
    const cells = wrapper.findAll("[data-day]");
    // 7th cell (index 6) should have extra margin class
    expect(cells[6].classes()).toContain("mr-2");
    // 14th cell (index 13) should have extra margin class
    expect(cells[13].classes()).toContain("mr-2");
    // 8th cell (index 7) should NOT have it
    expect(cells[7].classes()).not.toContain("mr-2");
  });
});
```

- [ ] **Step 2: 运行测试验证失败**

```bash
pnpm test -- src/__tests__/components/DayStrip.test.ts
```

Expected: FAIL — no DayStrip component yet.

- [ ] **Step 3: 实现 DayStrip.vue**

`src/components/DayStrip.vue`:

```vue
<script setup lang="ts">
import { computed, ref, onMounted, nextTick } from "vue";
import type { Entry } from "../types";

const props = defineProps<{
  dates: string[];
  selectedDate: string;
  monthEntries: Record<string, Entry[]>;
}>();

const emit = defineEmits<{
  selectDay: [date: string];
}>();

const stripRef = ref<HTMLDivElement>();

function isToday(dateStr: string): boolean {
  const now = new Date();
  const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  return dateStr === today;
}

function isFuture(dateStr: string): boolean {
  const now = new Date();
  now.setHours(0, 0, 0, 0);
  const [y, m, d] = dateStr.split("-").map(Number);
  const target = new Date(y, m - 1, d);
  target.setHours(0, 0, 0, 0);
  return target > now;
}

function hasEntries(dateStr: string): boolean {
  const entries = props.monthEntries[dateStr];
  return entries !== undefined && entries.length > 0;
}

function dayNumber(dateStr: string): number {
  return parseInt(dateStr.split("-")[2], 10);
}

function handleClick(dateStr: string) {
  if (isFuture(dateStr)) return;
  emit("selectDay", dateStr);
}

// Scroll to make selected date visible on mount
onMounted(async () => {
  await nextTick();
  if (!stripRef.value) return;
  const selected = stripRef.value.querySelector(`[data-day="${props.selectedDate}"]`);
  if (selected) {
    selected.scrollIntoView({ inline: "center", block: "nearest", behavior: "instant" });
  }
});
</script>

<template>
  <div
    ref="stripRef"
    class="flex overflow-x-auto border border-gray-200 rounded-lg bg-white py-1.5 px-1"
  >
    <button
      v-for="(dateStr, idx) in dates"
      :key="dateStr"
      :data-day="dateStr"
      class="flex-shrink-0 w-9 h-11 flex flex-col items-center justify-center rounded text-xs transition-colors"
      :class="[
        dateStr === selectedDate
          ? 'bg-blue-600 text-white font-semibold'
          : isFuture(dateStr)
            ? 'text-gray-300 cursor-default'
            : isToday(dateStr)
              ? 'text-gray-700 font-semibold hover:bg-gray-100 cursor-pointer'
              : 'text-gray-600 hover:bg-gray-100 cursor-pointer',
        (idx + 1) % 7 === 0 ? 'mr-2' : '',
      ]"
      @click="handleClick(dateStr)"
    >
      <span>{{ dayNumber(dateStr) }}</span>
      <span
        v-if="hasEntries(dateStr)"
        data-dot
        class="inline-block w-1.5 h-1.5 rounded-full mt-0.5"
        :class="dateStr === selectedDate ? 'bg-white' : 'bg-blue-500'"
      ></span>
    </button>
  </div>
</template>
```

- [ ] **Step 4: 运行测试**

```bash
pnpm test -- src/__tests__/components/DayStrip.test.ts
```

Expected: all 8 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/DayStrip.vue src/__tests__/components/DayStrip.test.ts
git commit -m "feat: add DayStrip component with tests"
```

---

### Task 5: 创建 MonthNavigator.vue（含测试）

**Files:**
- Create: `src/components/MonthNavigator.vue`
- Create: `src/__tests__/components/MonthNavigator.test.ts`

- [ ] **Step 1: 编写 MonthNavigator 测试**

`src/__tests__/components/MonthNavigator.test.ts`:

```typescript
import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import MonthNavigator from "../../components/MonthNavigator.vue";
import type { AvailableMonth } from "../../stores/useStore";

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

function mountNav(props: {
  year: number;
  month: number;
  availableMonths: AvailableMonth[] | null;
}) {
  return mount(MonthNavigator, { props });
}

describe("MonthNavigator", () => {
  it("displays month name and year", () => {
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths: null });
    expect(wrapper.text()).toContain("June 2026");
  });

  it("emits navigate on left arrow click", async () => {
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths: null });
    const buttons = wrapper.findAll("button");
    await buttons[0].trigger("click"); // left arrow
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2026, month: 5 }]);
  });

  it("emits navigate on right arrow click", async () => {
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths: null });
    const buttons = wrapper.findAll("button");
    await buttons[1].trigger("click"); // right arrow
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2026, month: 7 }]);
  });

  it("January left wraps to December previous year", async () => {
    const wrapper = mountNav({ year: 2026, month: 1, availableMonths: null });
    const buttons = wrapper.findAll("button");
    await buttons[0].trigger("click");
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2025, month: 12 }]);
  });

  it("December right wraps to January next year", async () => {
    const wrapper = mountNav({ year: 2026, month: 12, availableMonths: null });
    const buttons = wrapper.findAll("button");
    await buttons[1].trigger("click");
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2027, month: 1 }]);
  });

  it("clicking month-year text toggles quick-jump popover", async () => {
    const availableMonths: AvailableMonth[] = [
      { year: 2026, month: 6 },
      { year: 2026, month: 5 },
    ];
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths });
    const label = wrapper.find(".cursor-pointer");
    await label.trigger("click");
    // Popover should now be visible
    expect(wrapper.find("select").exists()).toBe(true);
  });

  it("quick-jump popover: year select lists unique years from availableMonths", async () => {
    const availableMonths: AvailableMonth[] = [
      { year: 2026, month: 6 },
      { year: 2025, month: 3 },
      { year: 2025, month: 8 },
    ];
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths });
    await wrapper.find(".cursor-pointer").trigger("click");
    const yearSelect = wrapper.find("select");
    const options = yearSelect.findAll("option");
    const years = options.map(o => parseInt(o.element.value));
    expect(years).toContain(2026);
    expect(years).toContain(2025);
  });

  it("quick-jump popover: month select shows only months for selected year", async () => {
    const availableMonths: AvailableMonth[] = [
      { year: 2026, month: 6 },
      { year: 2026, month: 3 },
      { year: 2025, month: 8 },
    ];
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths });
    await wrapper.find(".cursor-pointer").trigger("click");
    const selects = wrapper.findAll("select");
    // First select is year, second is month
    const monthSelect = selects[1];
    const options = monthSelect.findAll("option");
    const monthValues = options.map(o => parseInt(o.element.value));
    // For year 2026 (selected), only months 3 and 6 should appear
    expect(monthValues).toEqual(expect.arrayContaining([3, 6]));
    expect(monthValues).not.toEqual(expect.arrayContaining([8]));
  });

  it("quick-jump: changing month select emits navigate", async () => {
    const availableMonths: AvailableMonth[] = [
      { year: 2026, month: 6 },
      { year: 2026, month: 3 },
    ];
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths });
    await wrapper.find(".cursor-pointer").trigger("click");
    const monthSelect = wrapper.findAll("select")[1];
    await monthSelect.setValue(3);
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2026, month: 3 }]);
  });

  it("quick-jump not shown when availableMonths is null (not yet loaded)", () => {
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths: null });
    const label = wrapper.find(".cursor-pointer");
    // When null, clicking should emit requestMonths and not open popover
    expect(wrapper.text()).toContain("June 2026");
    // The label should still be clickable (triggers requestMonths)
    expect(label.exists()).toBe(true);
  });
});
```

- [ ] **Step 2: 运行测试验证失败**

```bash
pnpm test -- src/__tests__/components/MonthNavigator.test.ts
```

Expected: FAIL — no MonthNavigator component yet.

- [ ] **Step 3: 实现 MonthNavigator.vue**

`src/components/MonthNavigator.vue`:

```vue
<script setup lang="ts">
import { ref, computed } from "vue";
import type { AvailableMonth } from "../stores/useStore";

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

const props = defineProps<{
  year: number;
  month: number; // 1-based
  availableMonths: AvailableMonth[] | null; // null = not yet loaded
}>();

const emit = defineEmits<{
  navigate: [{ year: number; month: number }];
  requestMonths: [];
}>();

const showPopover = ref(false);

function monthLabel(m: number): string {
  return MONTH_NAMES[m - 1];
}

function shiftMonth(delta: number) {
  let newMonth = props.month + delta;
  let newYear = props.year;
  if (newMonth < 1) {
    newMonth = 12;
    newYear--;
  } else if (newMonth > 12) {
    newMonth = 1;
    newYear++;
  }
  emit("navigate", { year: newYear, month: newMonth });
}

function handleLabelClick() {
  if (props.availableMonths === null) {
    emit("requestMonths");
    return;
  }
  showPopover.value = !showPopover.value;
}

// Unique years from availableMonths
const availableYears = computed(() => {
  if (!props.availableMonths) return [];
  const years = [...new Set(props.availableMonths.map(m => m.year))];
  years.sort((a, b) => b - a);
  return years;
});

// Selected year in the popover (defaults to current year)
const selectedYear = ref(props.year);

// Months for the year selected in the popover
const monthsForYear = computed(() => {
  if (!props.availableMonths) return [];
  return props.availableMonths
    .filter(m => m.year === selectedYear.value)
    .map(m => m.month)
    .sort((a, b) => a - b);
});

// Reset selectedYear when popover opens
function openPopover() {
  selectedYear.value = props.year;
}

function onYearChange() {
  // When year changes in dropdown, auto-select first available month
  const firstMonth = monthsForYear.value[0];
  // Don't emit yet, let user pick the month
}

function onMonthChange(month: number) {
  emit("navigate", { year: selectedYear.value, month });
  showPopover.value = false;
}
</script>

<template>
  <div class="relative bg-white rounded-lg border border-gray-200 p-3 text-center">
    <div class="flex items-center justify-center gap-3">
      <button
        class="text-gray-500 hover:text-gray-700 transition-colors text-base px-1"
        @click="shiftMonth(-1)"
      >←</button>
      <span
        class="text-base font-bold text-gray-800 cursor-pointer hover:text-blue-600 transition-colors select-none"
        @click="handleLabelClick(); openPopover()"
      >
        {{ monthLabel(month) }} {{ year }}
        <span v-if="availableMonths !== null" class="text-xs text-gray-400">▾</span>
      </span>
      <button
        class="text-gray-500 hover:text-gray-700 transition-colors text-base px-1"
        @click="shiftMonth(1)"
      >→</button>
    </div>

    <!-- Quick-jump popover -->
    <div
      v-if="showPopover && availableMonths !== null"
      class="absolute top-full left-1/2 -translate-x-1/2 mt-1 bg-white border border-gray-200 rounded-lg shadow-lg p-3 z-10 flex gap-2"
    >
      <select
        v-model="selectedYear"
        class="text-sm border border-gray-300 rounded px-2 py-1 focus:outline-none focus:ring-1 focus:ring-blue-500"
        @change="onYearChange"
      >
        <option
          v-for="y in availableYears"
          :key="y"
          :value="y"
        >{{ y }}</option>
      </select>
      <select
        class="text-sm border border-gray-300 rounded px-2 py-1 focus:outline-none focus:ring-1 focus:ring-blue-500"
        @change="onMonthChange(parseInt(($event.target as HTMLSelectElement).value, 10))"
      >
        <option
          v-for="m in monthsForYear"
          :key="m"
          :value="m"
          :selected="m === month && selectedYear === year"
        >{{ monthLabel(m) }}</option>
      </select>
    </div>
  </div>
</template>
```

- [ ] **Step 4: 运行测试**

```bash
pnpm test -- src/__tests__/components/MonthNavigator.test.ts
```

Expected: all 9 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/MonthNavigator.vue src/__tests__/components/MonthNavigator.test.ts
git commit -m "feat: add MonthNavigator component with tests"
```

---

### Task 6: 更新 EntryList.vue —— 移除 granularity，添加内联合计行

**Files:**
- Modify: `src/components/EntryList.vue`
- Modify: `src/__tests__/components/EntryList.test.ts`

- [ ] **Step 1: 更新 EntryList.test.ts**

将 `src/__tests__/components/EntryList.test.ts` 替换为：

```typescript
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import EntryList from "../../components/EntryList.vue";
import EntryItem from "../../components/EntryItem.vue";
import { makeEntry } from "../mocks/fixtures";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import { makeConfig } from "../mocks/fixtures";

const store = createTestStore({ config: makeConfig() });
const provide = { [STORE_KEY as symbol]: store };

describe("EntryList", () => {
  it("empty: shows empty state message", () => {
    const wrapper = mount(EntryList, {
      props: { entries: [] },
      global: { provide },
    });
    expect(wrapper.text()).toContain("No entries yet");
  });

  it("with entries: renders flat EntryItem list", () => {
    const entries = [makeEntry(), makeEntry(), makeEntry()];
    const wrapper = mount(EntryList, {
      props: { entries },
      global: { provide },
    });
    expect(wrapper.findAllComponents(EntryItem)).toHaveLength(3);
  });

  it("bubbles update event from child", async () => {
    const entries = [makeEntry({ id: "e1", item: "Test", duration: 30 })];
    const wrapper = mount(EntryList, {
      props: { entries },
      global: { provide },
    });
    const item = wrapper.findComponent(EntryItem);
    await item.vm.$emit("update", "e1", "Updated", 45);
    expect(wrapper.emitted("update")).toBeTruthy();
    expect(wrapper.emitted("update")![0]).toEqual(["e1", "Updated", 45]);
  });

  it("bubbles delete event from child", async () => {
    const entries = [makeEntry({ id: "e1" })];
    const wrapper = mount(EntryList, {
      props: { entries },
      global: { provide },
    });
    const item = wrapper.findComponent(EntryItem);
    await item.vm.$emit("delete", "e1");
    expect(wrapper.emitted("delete")).toBeTruthy();
    expect(wrapper.emitted("delete")![0]).toEqual(["e1"]);
  });

  it("bubbles updateDimensions event from child", async () => {
    const entries = [makeEntry({ id: "e1" })];
    const wrapper = mount(EntryList, {
      props: { entries },
      global: { provide },
    });
    const item = wrapper.findComponent(EntryItem);
    await item.vm.$emit("update-dimensions", "e1", { goal: "Code review" });
    expect(wrapper.emitted("updateDimensions")).toBeTruthy();
    expect(wrapper.emitted("updateDimensions")![0]).toEqual(["e1", { goal: "Code review" }]);
  });

  it("shows inline summary row when entries exist", () => {
    const entries = [makeEntry({ duration: 30 }), makeEntry({ duration: 45 })];
    const wrapper = mount(EntryList, {
      props: { entries },
      global: { provide },
    });
    const text = wrapper.text();
    expect(text).toContain("2 entries");
    expect(text).toContain("1h 15m");
  });

  it('singular "1 entry" in summary', () => {
    const entries = [makeEntry({ duration: 15 })];
    const wrapper = mount(EntryList, {
      props: { entries },
      global: { provide },
    });
    expect(wrapper.text()).toContain("1 entry");
  });

  it("no summary row when no entries", () => {
    const wrapper = mount(EntryList, {
      props: { entries: [] },
      global: { provide },
    });
    expect(wrapper.text()).not.toContain("entries");
  });
});
```

- [ ] **Step 2: 运行测试验证失败**

```bash
pnpm test -- src/__tests__/components/EntryList.test.ts
```

Expected: some tests FAIL — granularity prop no longer accepted.

- [ ] **Step 3: 更新 EntryList.vue**

将 `src/components/EntryList.vue` 替换为：

```vue
<script setup lang="ts">
import type { Entry } from "../types";
import { computed } from "vue";
import EntryItem from "./EntryItem.vue";
import { formatDuration } from "../utils/format";

const props = defineProps<{
  entries: Entry[];
}>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
  updateDimensions: [entryId: string, dimensions: Record<string, string>];
}>();

const totalMinutes = computed(() =>
  props.entries.reduce((s, e) => s + e.duration, 0)
);

const entryCount = computed(() => props.entries.length);
</script>

<template>
  <div class="bg-white rounded-lg shadow-sm">
    <div v-if="entries.length === 0" class="p-8 text-center text-gray-400 text-sm">
      No entries yet. Log your first work item above.
    </div>
    <div v-else class="px-4">
      <EntryItem
        v-for="(entry, index) in entries"
        :key="entry.id"
        :entry="entry"
        :index="index"
        @update="(entryId, item, dur) => emit('update', entryId, item, dur)"
        @delete="(entryId) => emit('delete', entryId)"
        @update-dimensions="(entryId, dims) => emit('updateDimensions', entryId, dims)"
      />
      <!-- Inline summary row -->
      <div class="flex justify-between text-xs text-gray-500 py-2 border-t border-gray-200 mt-2">
        <span>{{ entryCount }} {{ entryCount === 1 ? "entry" : "entries" }}</span>
        <span class="font-medium text-gray-700">{{ formatDuration(totalMinutes) }}</span>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 4: 运行测试**

```bash
pnpm test -- src/__tests__/components/EntryList.test.ts
```

Expected: all 8 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/EntryList.vue src/__tests__/components/EntryList.test.ts
git commit -m "refactor: remove granularity from EntryList, add inline summary row"
```

---

### Task 7: 创建 MonthView.vue（核心容器）

**Files:**
- Create: `src/components/MonthView.vue`

- [ ] **Step 1: 实现 MonthView.vue**

`src/components/MonthView.vue`:

```vue
<script setup lang="ts">
import { inject, computed, watch, ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../stores/useStore";
import MonthNavigator from "./MonthNavigator.vue";
import CommitmentsPanel from "./CommitmentsPanel.vue";
import DayStrip from "./DayStrip.vue";
import QuickEntry from "./QuickEntry.vue";
import EntryList from "./EntryList.vue";
import type { DayFile, Entry, CommitmentProgress } from "../types";
import { logError } from "../utils/errorLog";
import { datesInMonth, yearMonthFromDate } from "../utils/dates";

const store = useStore();

const selectedYear = computed(() => yearMonthFromDate(store.currentDate).year);
const selectedMonth = computed(() => yearMonthFromDate(store.currentDate).month);

// All dates in the current month
const monthDates = computed(() => datesInMonth(store.currentDate));

// Whether the selected date is today
const isSelectedToday = computed(() => {
  const now = new Date();
  const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  return store.currentDate === today;
});

// Inject undo toast trigger from App.vue
const triggerUndoToast = inject<(undoFn: () => void) => void>("triggerUndoToast", () => {});

// ---- Month loading ----

async function loadMonth(year: number, month: number, defaultDay?: number) {
  // Determine the date to select within the target month
  const now = new Date();
  const isCurrentMonth = year === now.getFullYear() && month === now.getMonth() + 1;

  let day: number;
  if (defaultDay !== undefined) {
    day = defaultDay;
  } else if (isCurrentMonth) {
    day = now.getDate();
  } else {
    day = new Date(year, month, 0).getDate(); // last day of past month
  }

  const dateStr = `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
  store.currentDate = dateStr;

  // Load all entries for the month
  const dates = datesInMonth(dateStr);
  const map: Record<string, Entry[]> = {};
  for (const date of dates) {
    try {
      const df = (await invoke("get_entries", { rootPath: store.rootPath, date })) as DayFile;
      map[date] = df.entries;
    } catch (e) {
      logError("MonthView.loadMonth", e);
      map[date] = [];
    }
  }
  store.monthEntries = map;

  // Load commitments
  await loadCommitmentProgress(year, month);

  // Set today from the loaded data
  if (store.currentDate in map) {
    store.today = { note: null, entries: map[store.currentDate] };
    // Also load the note for the selected day
    loadDayNote(store.currentDate);
  }
}

async function loadCommitmentProgress(year: number, month: number) {
  try {
    store.commitmentProgress = (await invoke("get_commitment_progress", {
      rootPath: store.rootPath,
      year,
      month,
    })) as CommitmentProgress[];
  } catch (e) {
    logError("MonthView.loadCommitmentProgress", e);
    store.commitmentProgress = [];
  }
}

async function loadDayNote(dateStr: string) {
  try {
    const df = (await invoke("get_entries", { rootPath: store.rootPath, date: dateStr })) as DayFile;
    if (store.today) {
      store.today.note = df.note;
    }
  } catch (e) {
    logError("MonthView.loadDayNote", e);
  }
}

// ---- Day selection (from DayStrip) ----

async function handleSelectDay(dateStr: string) {
  store.currentDate = dateStr;
  if (dateStr in store.monthEntries) {
    // Sync note from store before switching
    store.today = { note: null, entries: store.monthEntries[dateStr] };
    await loadDayNote(dateStr);
  }
}

// ---- Month navigation ----

async function handleNavigate({ year, month }: { year: number; month: number }) {
  await loadMonth(year, month);
}

// ---- Lazy load available months ----

async function handleRequestMonths() {
  if (store.availableMonths !== null) return;
  try {
    const months = (await invoke("get_available_months", { rootPath: store.rootPath })) as { year: number; month: number }[];
    store.availableMonths = months;
  } catch (e) {
    logError("MonthView.handleRequestMonths", e);
    store.availableMonths = [];
  }
}

// ---- Entry mutations ----

async function handleUpdateEntry(entryId: string, item: string, durationMinutes: number) {
  const entries = store.today?.entries;
  if (!entries) return;
  const entry = entries.find(e => e.id === entryId);
  if (!entry) return;

  const update: Record<string, unknown> = {};
  if (item !== entry.item) update.item = item;
  if (durationMinutes !== entry.duration) update.duration = String(durationMinutes);
  if (Object.keys(update).length === 0) return;

  try {
    const df = (await invoke("update_entry", {
      rootPath: store.rootPath,
      date: store.currentDate,
      entryId,
      update,
    })) as DayFile;
    store.today = df;
    store.monthEntries[store.currentDate] = df.entries;
    await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
  } catch (e) {
    logError("MonthView.handleUpdateEntry", e);
  }
}

async function handleUpdateDimensions(entryId: string, dimensions: Record<string, string>) {
  try {
    const df = (await invoke("update_entry", {
      rootPath: store.rootPath,
      date: store.currentDate,
      entryId,
      update: { dimensions },
    })) as DayFile;
    store.today = df;
    store.monthEntries[store.currentDate] = df.entries;
    await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
  } catch (e) {
    logError("MonthView.handleUpdateDimensions", e);
  }
}

let pendingDeleteTimer: ReturnType<typeof setTimeout> | null = null;

async function handleDeleteEntry(entryId: string) {
  const entries = store.today?.entries;
  if (!entries) return;

  const idx = entries.findIndex(e => e.id === entryId);
  if (idx === -1) return;
  const [removed] = entries.splice(idx, 1);

  let cancelled = false;
  pendingDeleteTimer = setTimeout(async () => {
    if (cancelled) return;
    try {
      await invoke("delete_entry", { rootPath: store.rootPath, date: store.currentDate, entryId });
      store.monthEntries[store.currentDate] = [...entries];
      await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
    } catch (e) {
      logError("MonthView.handleDeleteEntry", e);
      const currentIdx = entries.findIndex(e => e.id === entryId);
      if (currentIdx === -1) {
        entries.splice(idx, 0, removed);
      }
    }
  }, 5000);

  triggerUndoToast(() => {
    cancelled = true;
    if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
    const currentIdx = entries.findIndex(e => e.id === entryId);
    if (currentIdx === -1) {
      entries.splice(idx, 0, removed);
    }
  });
}

async function handleAppended() {
  // After QuickEntry appends: reload current day + refresh month + commitments
  await loadMonth(selectedYear.value, selectedMonth.value, parseInt(store.currentDate.split("-")[2], 10));
}

// ---- Day note ----

const noteRef = ref<HTMLDivElement>();

watch(
  () => store.today?.note,
  (n) => {
    if (noteRef.value && noteRef.value.textContent !== (n || "")) {
      noteRef.value.textContent = n || "";
    }
  },
  { immediate: true }
);

// Load month data on mount
onMounted(async () => {
  if (store.rootPath) {
    const { year, month } = yearMonthFromDate(store.currentDate);
    await loadMonth(year, month);
  }
});

function onNotePaste(e: ClipboardEvent) {
  e.preventDefault();
  const text = e.clipboardData?.getData("text/plain") || "";
  const selection = window.getSelection();
  if (selection && selection.rangeCount > 0) {
    const range = selection.getRangeAt(0);
    range.deleteContents();
    range.insertNode(document.createTextNode(text));
    range.collapse(false);
  }
}

function onNoteInput() {
  if (noteRef.value && noteRef.value.innerHTML !== noteRef.value.textContent) {
    noteRef.value.textContent = noteRef.value.textContent || "";
  }
}

async function saveNote() {
  const text = noteRef.value?.textContent || "";
  try {
    await invoke("set_day_note", { rootPath: store.rootPath, date: store.currentDate, note: text });
  } catch (e) {
    logError("MonthView.saveNote", e);
  }
}

// ---- File path ----

const dayFilePath = computed(() => {
  if (!store.rootPath) return "";
  const d = store.currentDate;
  const year = d.slice(0, 4);
  const month = d.slice(5, 7);
  return `${year}/${month}/${d}.md`;
});

const displayPath = computed(() => {
  if (!store.rootPath) return "";
  return `…/${dayFilePath.value}`;
});

async function openInEditor() {
  if (!store.rootPath) return;
  try {
    await invoke("open_in_editor", { rootPath: store.rootPath, date: store.currentDate });
  } catch (e) {
    logError("MonthView.openInEditor", e);
  }
}
</script>

<template>
  <div class="flex gap-4 p-4 max-w-7xl mx-auto items-start">
    <!-- Left 1/3: Month sidebar -->
    <div class="flex-1 min-w-[200px] flex flex-col gap-3 sticky top-4">
      <MonthNavigator
        :year="selectedYear"
        :month="selectedMonth"
        :availableMonths="store.availableMonths"
        @navigate="handleNavigate"
        @requestMonths="handleRequestMonths"
      />
      <CommitmentsPanel
        :progress="store.commitmentProgress"
        :selectedYear="selectedYear"
        :selectedMonth="selectedMonth"
      />
    </div>

    <!-- Right 2/3: Day detail -->
    <div class="flex-[2] min-w-0 flex flex-col gap-3">
      <DayStrip
        :dates="monthDates"
        :selectedDate="store.currentDate"
        :monthEntries="store.monthEntries"
        @selectDay="handleSelectDay"
      />

      <!-- DayNote -->
      <div
        ref="noteRef"
        class="text-xs text-gray-500 outline-none rounded px-3 py-1.5 bg-white border border-gray-200 hover:bg-gray-50 focus:bg-white focus:ring-2 focus:ring-blue-500 cursor-text min-h-[28px]"
        contenteditable="true"
        data-placeholder="Add a note…"
        @blur="saveNote"
        @paste="onNotePaste"
        @input="onNoteInput"
      ></div>

      <QuickEntry v-if="isSelectedToday" @appended="handleAppended" />

      <EntryList
        :entries="store.today?.entries || []"
        @update="(entryId, item, dur) => handleUpdateEntry(entryId, item, dur)"
        @delete="(entryId) => handleDeleteEntry(entryId)"
        @update-dimensions="(entryId, dims) => handleUpdateDimensions(entryId, dims)"
      />

      <!-- File path link -->
      <div v-if="store.rootPath" class="text-right">
        <button
          class="text-xs text-gray-400 hover:text-gray-600 transition-colors cursor-pointer"
          :title="store.rootPath + '/' + dayFilePath"
          @click="openInEditor"
        >
          {{ displayPath }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
[contenteditable]:empty::before {
  content: attr(data-placeholder);
  color: #cbd5e1;
}
</style>
```

- [ ] **Step 2: 验证 TypeScript 编译**

```bash
pnpm vue-tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/MonthView.vue
git commit -m "feat: add MonthView component as new main view"
```

---

### Task 8: 更新 App.vue —— 使用 MonthView

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: 更新 App.vue**

将 `src/App.vue` 的 `<script setup>` 中 import 部分替换：

```typescript
import { onMounted, onUnmounted, ref, provide } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useStore } from "./stores/useStore";
import SetupScreen from "./components/SetupScreen.vue";
import ConfigErrorBanner from "./components/ConfigErrorBanner.vue";
import MonthView from "./components/MonthView.vue";
import type { InitResult, ConfigErrorDetail } from "./types";
import { logError, logInfo } from "./utils/errorLog";
```

Template 中将 `<TodayView v-else-if="store.screen === 'ready'" />` 替换为：

```html
<MonthView v-else-if="store.screen === 'ready'" />
```

从 `initApp` 中删除 `loadCommitmentProgress` 调用和函数定义（MonthView 自己处理加载）。

更新后的 `initApp`:

```typescript
async function initApp() {
  logInfo("App.initApp", "start");
  try {
    const result = (await invoke("init")) as InitResult;
    switch (result.status) {
      case "NeedsSetup":
        store.screen = "setup";
        break;
      case "ConfigError":
        store.configErrors = result.data;
        store.screen = "error";
        break;
      case "Ready":
        store.rootPath = result.data.root_path;
        store.config = result.data.config;
        store.today = result.data.today;
        store.commitments = result.data.commitments;
        store.screen = "ready";
        break;
    }
    logInfo("App.initApp", result.status);
  } catch (e) {
    logError("App.initApp", e);
    store.configErrors = [{ kind: "InitError", message: `Failed: ${e}` }];
    store.screen = "error";
  }
}
```

删除 `loadCommitmentProgress` 函数。

- [ ] **Step 2: 验证 TypeScript 编译**

```bash
pnpm vue-tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/App.vue
git commit -m "refactor: switch App.vue to use MonthView, remove loadCommitmentProgress"
```

---

### Task 9: 删除旧组件

**Files:**
- Delete: `src/components/TodayView.vue`
- Delete: `src/components/DateNavigator.vue`
- Delete: `src/components/SummaryBar.vue`
- Delete: `src/components/EntryGroup.vue`（week/month 分组不再使用）

- [ ] **Step 1: 删除旧组件文件**

```bash
rm src/components/TodayView.vue src/components/DateNavigator.vue src/components/SummaryBar.vue src/components/EntryGroup.vue
```

- [ ] **Step 2: 验证 TypeScript 编译**

```bash
pnpm vue-tsc --noEmit
```

Expected: no errors. If any remaining imports reference these deleted files, fix them.

- [ ] **Step 3: Commit**

```bash
git rm src/components/TodayView.vue src/components/DateNavigator.vue src/components/SummaryBar.vue src/components/EntryGroup.vue
git commit -m "refactor: remove old TodayView, DateNavigator, SummaryBar, EntryGroup components"
```

---

### Task 10: 更新旧测试文件并删除废弃测试

**Files:**
- Modify: `src/__tests__/components/App.test.ts`
- Delete: `src/__tests__/components/TodayView.test.ts`
- Delete: `src/__tests__/components/DateNavigator.test.ts`
- Delete: `src/__tests__/components/SummaryBar.test.ts`
- Delete: `src/__tests__/components/EntryGroup.test.ts`（废弃组件）

- [ ] **Step 1: 更新 App.test.ts —— ToolbarView → MonthView**

`src/__tests__/components/App.test.ts` 中将所有 `TodayView` 替换为 `MonthView`：

- Line 110: `it("Ready: shows TodayView and populates store"` → `"Ready: shows MonthView and populates store"`
- Line 124: `expect(wrapper.findComponent({ name: "TodayView" }).exists()).toBe(true);` → `MonthView`
- Line 245-247: Comment referencing TodayView → update

- [ ] **Step 2: 删除废弃测试文件**

```bash
rm src/__tests__/components/TodayView.test.ts src/__tests__/components/DateNavigator.test.ts src/__tests__/components/SummaryBar.test.ts src/__tests__/components/EntryGroup.test.ts
```

- [ ] **Step 3: 运行所有测试**

```bash
pnpm test
```

Expected: all tests pass. Fix any failures.

- [ ] **Step 4: Commit**

```bash
git rm src/__tests__/components/TodayView.test.ts src/__tests__/components/DateNavigator.test.ts src/__tests__/components/SummaryBar.test.ts src/__tests__/components/EntryGroup.test.ts
git add src/__tests__/components/App.test.ts
git commit -m "test: update App test for MonthView, remove old component tests"
```

---

### Task 11: 创建 MonthView 集成测试

**Files:**
- Create: `src/__tests__/components/MonthView.test.ts`

- [ ] **Step 1: 编写 MonthView 测试**

```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import { makeConfig, makeDayFile, makeEntry, makeCommitmentProgress } from "../mocks/fixtures";
import MonthView from "../../components/MonthView.vue";
import DayStrip from "../../components/DayStrip.vue";
import MonthNavigator from "../../components/MonthNavigator.vue";
import CommitmentsPanel from "../../components/CommitmentsPanel.vue";
import QuickEntry from "../../components/QuickEntry.vue";
import EntryList from "../../components/EntryList.vue";

// Hoisted mocks for Tauri invoke
const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));

function mountMonthView(store = createTestStore()) {
  return mount(MonthView, {
    global: {
      provide: { [STORE_KEY as symbol]: store },
    },
  });
}

describe("MonthView", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 5, 14)); // June 14, 2026
    vi.clearAllMocks();

    // Default: get_entries returns empty day files
    mockInvoke.mockImplementation(async (cmd: string, _args: unknown) => {
      if (cmd === "get_entries") return { note: null, entries: [] };
      if (cmd === "get_commitment_progress") return [];
      return {};
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders left sidebar with MonthNavigator and CommitmentsPanel", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
      commitmentProgress: [makeCommitmentProgress()],
    });
    const wrapper = mountMonthView(store);
    expect(wrapper.findComponent(MonthNavigator).exists()).toBe(true);
    expect(wrapper.findComponent(CommitmentsPanel).exists()).toBe(true);
  });

  it("renders right panel with DayStrip and EntryList", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
      monthEntries: { "2026-06-14": [makeEntry()] },
    });
    store.today = { note: null, entries: [makeEntry()] };
    const wrapper = mountMonthView(store);
    expect(wrapper.findComponent(DayStrip).exists()).toBe(true);
    expect(wrapper.findComponent(EntryList).exists()).toBe(true);
  });

  it("QuickEntry visible when selected date is today", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14", // same as system time
    });
    const wrapper = mountMonthView(store);
    expect(wrapper.findComponent(QuickEntry).exists()).toBe(true);
  });

  it("QuickEntry hidden when selected date is not today", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-10", // not today
    });
    const wrapper = mountMonthView(store);
    expect(wrapper.findComponent(QuickEntry).exists()).toBe(false);
  });

  it("DayStrip receives monthDates from currentDate", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-02-14",
    });
    const wrapper = mountMonthView(store);
    const strip = wrapper.findComponent(DayStrip);
    expect(strip.props("dates")).toHaveLength(28); // February 2026
  });

  it("clicking a day in DayStrip updates currentDate and today", async () => {
    const entries = [makeEntry({ item: "Test", duration: 30 })];
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
      monthEntries: {
        "2026-06-14": [],
        "2026-06-15": entries,
      },
    });
    store.today = { note: null, entries: [] };
    const wrapper = mountMonthView(store);

    // Simulate DayStrip emitting selectDay
    const strip = wrapper.findComponent(DayStrip);
    await strip.vm.$emit("selectDay", "2026-06-15");
    await nextTick();

    expect(store.currentDate).toBe("2026-06-15");
    expect(store.today?.entries).toEqual(entries);
  });

  it("renders day note contenteditable", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });
    store.today = { note: "My note", entries: [] };
    const wrapper = mountMonthView(store);
    expect(wrapper.find('[contenteditable="true"]').exists()).toBe(true);
  });
});
```

- [ ] **Step 2: 运行测试**

```bash
pnpm test -- src/__tests__/components/MonthView.test.ts
```

Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/__tests__/components/MonthView.test.ts
git commit -m "test: add MonthView integration tests"
```

---

### Task 12: 最终验证

- [ ] **Step 1: 运行所有前端测试**

```bash
pnpm test
```

Expected: all tests pass.

- [ ] **Step 2: TypeScript 编译检查**

```bash
pnpm vue-tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Rust 编译检查**

```bash
cd src-tauri && cargo check && cargo test
```

Expected: cargo check clean, all tests pass.

- [ ] **Step 4: 构建验证**

```bash
pnpm tauri build --debug 2>&1 | tail -20
```

Expected: build succeeds. Or if full build takes too long, at minimum verify `cargo check` + `vue-tsc --noEmit` pass.

- [ ] **Step 5: Commit 任何遗漏的变更**

```bash
git status
# 如果有遗漏的变更，git add + commit
```
