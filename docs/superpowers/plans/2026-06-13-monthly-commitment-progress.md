# Monthly Commitment Progress — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace daily commitment progress with monthly cumulative progress — each role has a monthly hour budget, progress shows month-to-date actual hours spent.

**Architecture:** New Rust command `get_commitment_progress` scans all day files in a month and aggregates spent minutes by goal → role. Frontend CommitmentsPanel switches from daily computation to receiving pre-aggregated `CommitmentProgress[]` data. Color logic changes from fixed thresholds to elapsed-time-relative (green/orange/yellow/red).

**Tech Stack:** Tauri 2.x (Rust), Vue 3 + Composition API + TypeScript, Vitest + vue-test-utils (frontend), cargo test (Rust unit + integration)

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `src-tauri/src/models.rs` | Modify | Add `CommitmentProgress`, `GoalProgress` structs |
| `src-tauri/src/commands.rs` | Modify | Add `get_commitment_progress` command |
| `src-tauri/src/lib.rs` | Modify | Register new command |
| `src-tauri/tests/commitment_progress_integration.rs` | Create | Integration tests for new command |
| `src/types.ts` | Modify | Add TS interfaces for `CommitmentProgress`, `GoalProgress` |
| `src/stores/useStore.ts` | Modify | Add `commitmentProgress` field |
| `src/components/CommitmentsPanel.vue` | Modify | New props, remove daily logic, add `elapsedRatio`/`barColor` |
| `src/components/TodayView.vue` | Modify | Call `get_commitment_progress`, pass to panel |
| `src/App.vue` | Modify | Load commitment progress on init + watcher events |
| `src/__tests__/components/CommitmentsPanel.test.ts` | Modify | Rewrite for new props + color logic |
| `src/__tests__/components/TodayView.test.ts` | Modify | Update for new invoke calls |
| `src/__tests__/mocks/fixtures.ts` | Modify | Add `makeCommitmentProgress` factory |

---

### Task 1: Add Rust data models

**Files:**
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: Add `CommitmentProgress` and `GoalProgress` structs to models.rs**

Insert after the existing `Commitment` struct (after line 41):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentProgress {
    pub role: String,
    pub allocation_minutes: u32,
    pub spent_minutes: u32,
    pub goals: Vec<GoalProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalProgress {
    pub name: String,
    pub spent_minutes: u32,
}
```

- [ ] **Step 2: Verify compilation**

```bash
cd src-tauri && cargo check
```

Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/models.rs
git commit -m "feat: add CommitmentProgress and GoalProgress Rust structs"
```

---

### Task 2: Implement `get_commitment_progress` command

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block at the end of `commands.rs` (before the closing `}` of `mod tests`):

```rust
#[test]
fn test_get_commitment_progress_empty_month() {
    use std::fs;
    let tmp = std::env::temp_dir().join("logbook_test_cp_empty");
    let _ = fs::remove_dir_all(&tmp);

    // Create directory structure with _monthly.md but no day files
    let monthly_dir = tmp.join("2026").join("06");
    fs::create_dir_all(&monthly_dir).unwrap();
    fs::write(
        monthly_dir.join("_monthly.md"),
        "---\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it\n---\n",
    )
    .unwrap();

    let result = get_commitment_progress(
        tmp.to_string_lossy().into_owned(),
        2026,
        6,
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].role, "Dev");
    assert_eq!(result[0].allocation_minutes, 2400); // 40 * 60
    assert_eq!(result[0].spent_minutes, 0);
    assert_eq!(result[0].goals.len(), 1);
    assert_eq!(result[0].goals[0].name, "Ship it");
    assert_eq!(result[0].goals[0].spent_minutes, 0);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_get_commitment_progress_aggregates_spent() {
    use std::fs;
    let tmp = std::env::temp_dir().join("logbook_test_cp_agg");
    let _ = fs::remove_dir_all(&tmp);

    // Create _monthly.md
    let monthly_dir = tmp.join("2026").join("06");
    fs::create_dir_all(&monthly_dir).unwrap();
    fs::write(
        monthly_dir.join("_monthly.md"),
        "---\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it\n      - Review\n  - role: PM\n    allocation: 10\n    goals:\n      - Planning\n---\n",
    )
    .unwrap();

    // Create day file with entries matching goals
    fs::write(
        monthly_dir.join("2026-06-01.md"),
        "---\nentries:\n  - id: e1\n    item: Code\n    duration: 60\n    dimensions:\n      goal: Ship it\n  - id: e2\n    item: PR\n    duration: 30\n    dimensions:\n      goal: Review\n---\n",
    )
    .unwrap();

    fs::write(
        monthly_dir.join("2026-06-02.md"),
        "---\nentries:\n  - id: e3\n    item: Plan\n    duration: 45\n    dimensions:\n      goal: Planning\n---\n",
    )
    .unwrap();

    let result = get_commitment_progress(
        tmp.to_string_lossy().into_owned(),
        2026,
        6,
    )
    .unwrap();

    // Dev: Ship it(60) + Review(30) = 90 spent
    let dev = result.iter().find(|c| c.role == "Dev").unwrap();
    assert_eq!(dev.spent_minutes, 90);
    assert_eq!(dev.allocation_minutes, 2400);

    // PM: Planning(45) = 45 spent
    let pm = result.iter().find(|c| c.role == "PM").unwrap();
    assert_eq!(pm.spent_minutes, 45);
    assert_eq!(pm.allocation_minutes, 600); // 10 * 60

    // Goal-level check
    let ship_it = dev.goals.iter().find(|g| g.name == "Ship it").unwrap();
    assert_eq!(ship_it.spent_minutes, 60);
    let review = dev.goals.iter().find(|g| g.name == "Review").unwrap();
    assert_eq!(review.spent_minutes, 30);
    let planning = pm.goals.iter().find(|g| g.name == "Planning").unwrap();
    assert_eq!(planning.spent_minutes, 45);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_get_commitment_progress_ignores_unmatched_goals() {
    use std::fs;
    let tmp = std::env::temp_dir().join("logbook_test_cp_unmatch");
    let _ = fs::remove_dir_all(&tmp);

    let monthly_dir = tmp.join("2026").join("06");
    fs::create_dir_all(&monthly_dir).unwrap();
    fs::write(
        monthly_dir.join("_monthly.md"),
        "---\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it\n---\n",
    )
    .unwrap();

    // Entry with a goal NOT in any commitment
    fs::write(
        monthly_dir.join("2026-06-01.md"),
        "---\nentries:\n  - id: e1\n    item: Unknown task\n    duration: 60\n    dimensions:\n      goal: Not a goal\n---\n",
    )
    .unwrap();

    let result = get_commitment_progress(
        tmp.to_string_lossy().into_owned(),
        2026,
        6,
    )
    .unwrap();

    assert_eq!(result[0].spent_minutes, 0);
    assert_eq!(result[0].goals[0].spent_minutes, 0);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_get_commitment_progress_no_monthly_file() {
    let tmp = std::env::temp_dir().join("logbook_test_cp_nofile");
    let _ = std::fs::remove_dir_all(&tmp);

    let result = get_commitment_progress(
        tmp.to_string_lossy().into_owned(),
        2026,
        6,
    )
    .unwrap();

    assert!(result.is_empty());

    let _ = std::fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test test_get_commitment_progress
```

Expected: FAIL — `get_commitment_progress` not found.

- [ ] **Step 3: Implement `get_commitment_progress` command**

Add to `commands.rs` after the existing `get_commitments` function (after line 307):

```rust
#[tauri::command]
pub fn get_commitment_progress(
    root_path: String,
    year: i32,
    month: u32,
) -> Result<Vec<CommitmentProgress>, String> {
    use crate::models::{CommitmentProgress, GoalProgress};
    use std::collections::HashMap;

    let root = std::path::Path::new(&root_path);

    // 1. Read _monthly.md
    let monthly = crate::files::read_monthly_file(root, year, month).unwrap_or_else(|_| {
        MonthlyFile { commitments: vec![] }
    });

    let commitments = monthly.commitments;

    // 2. Build goal → (role, goal_name) map
    let mut goal_to_role: HashMap<String, (String, String)> = HashMap::new();
    for c in &commitments {
        for g in &c.goals {
            goal_to_role.insert(g.clone(), (c.role.clone(), g.clone()));
        }
    }

    // 3. Initialize result structures
    let mut role_spent: HashMap<String, u32> = HashMap::new();
    let mut goal_spent: HashMap<String, u32> = HashMap::new();
    for c in &commitments {
        role_spent.entry(c.role.clone()).or_insert(0);
        for g in &c.goals {
            goal_spent.entry(g.clone()).or_insert(0);
        }
    }

    // 4. Scan day files in the month directory
    let month_dir = root
        .join(year.to_string())
        .join(format!("{:02}", month));

    if month_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&month_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Skip _monthly.md and non-.md files
                if file_name == "_monthly.md" || !file_name.ends_with(".md") {
                    continue;
                }

                // Read the day file
                if let Ok(day_file) = crate::files::read_day_file(root, file_name.trim_end_matches(".md")) {
                    for e in &day_file.entries {
                        if let Some(goal) = e.dimensions.get("goal") {
                            if let Some((role, goal_name)) = goal_to_role.get(goal) {
                                *role_spent.entry(role.clone()).or_insert(0) += e.duration;
                                *goal_spent.entry(goal_name.clone()).or_insert(0) += e.duration;
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Build result vector
    let mut results: Vec<CommitmentProgress> = Vec::new();
    for c in &commitments {
        let goals: Vec<GoalProgress> = c
            .goals
            .iter()
            .map(|g| GoalProgress {
                name: g.clone(),
                spent_minutes: *goal_spent.get(g).unwrap_or(&0),
            })
            .collect();
        results.push(CommitmentProgress {
            role: c.role.clone(),
            allocation_minutes: c.allocation * 60,
            spent_minutes: *role_spent.get(&c.role).unwrap_or(&0),
            goals,
        });
    }

    Ok(results)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd src-tauri && cargo test test_get_commitment_progress
```

Expected: PASS — all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add get_commitment_progress Rust command"
```

---

### Task 3: Register command in Tauri handler

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `get_commitment_progress` to the invoke handler**

Add `commands::get_commitment_progress,` to the `invoke_handler` macro invocation on line 29. Insert after `commands::get_commitments,`:

```rust
commands::get_commitments,
commands::get_commitment_progress,
```

- [ ] **Step 2: Verify compilation**

```bash
cd src-tauri && cargo check
```

Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: register get_commitment_progress Tauri command"
```

---

### Task 4: Write Rust integration test

**Files:**
- Create: `src-tauri/tests/commitment_progress_integration.rs`

- [ ] **Step 1: Create integration test file**

```rust
/// Integration tests for get_commitment_progress command.
use std::collections::HashMap;
use std::fs;

use tauri_app_lib::models::NewEntry;

fn setup(suffix: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("logbook_int_cp_{}", suffix));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    // Write config.yaml
    fs::write(
        root.join("config.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
    )
    .unwrap();

    // Write _monthly.md for June 2026
    let monthly_dir = root.join("2026/06");
    fs::create_dir_all(&monthly_dir).unwrap();
    fs::write(
        monthly_dir.join("_monthly.md"),
        "---\ncommitments:\n  - role: Developer\n    allocation: 30\n    goals:\n      - Feature A\n      - Bug fixes\n  - role: VP\n    allocation: 15\n    goals:\n      - Strategy\n---\n",
    )
    .unwrap();

    root
}

fn teardown(root: &std::path::Path) {
    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_progress_on_empty_month() {
    let root = setup("empty");
    let progress = tauri_app_lib::commands::get_commitment_progress(
        root.to_string_lossy().into_owned(),
        2026,
        6,
    )
    .unwrap();

    assert_eq!(progress.len(), 2);
    assert_eq!(progress[0].role, "Developer");
    assert_eq!(progress[0].allocation_minutes, 1800); // 30 * 60
    assert_eq!(progress[0].spent_minutes, 0);
    assert_eq!(progress[1].role, "VP");
    assert_eq!(progress[1].allocation_minutes, 900); // 15 * 60
    assert_eq!(progress[1].spent_minutes, 0);

    teardown(&root);
}

#[test]
fn test_progress_aggregates_across_multiple_days() {
    let root = setup("multi_day");

    // Add entries across multiple days
    let mut dims_a = HashMap::new();
    dims_a.insert("goal".to_string(), "Feature A".to_string());

    let mut dims_b = HashMap::new();
    dims_b.insert("goal".to_string(), "Bug fixes".to_string());

    let mut dims_s = HashMap::new();
    dims_s.insert("goal".to_string(), "Strategy".to_string());

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-01",
        &NewEntry { item: "Day 1 feature".into(), duration: "60".into(), dimensions: dims_a.clone() },
    )
    .unwrap();

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-01",
        &NewEntry { item: "Day 1 strategy".into(), duration: "30".into(), dimensions: dims_s.clone() },
    )
    .unwrap();

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-05",
        &NewEntry { item: "Day 5 bugs".into(), duration: "45".into(), dimensions: dims_b.clone() },
    )
    .unwrap();

    let progress = tauri_app_lib::commands::get_commitment_progress(
        root.to_string_lossy().into_owned(),
        2026,
        6,
    )
    .unwrap();

    let dev = progress.iter().find(|c| c.role == "Developer").unwrap();
    // Feature A: 60, Bug fixes: 45 = 105 total
    assert_eq!(dev.spent_minutes, 105);
    let fa = dev.goals.iter().find(|g| g.name == "Feature A").unwrap();
    assert_eq!(fa.spent_minutes, 60);
    let bf = dev.goals.iter().find(|g| g.name == "Bug fixes").unwrap();
    assert_eq!(bf.spent_minutes, 45);

    let vp = progress.iter().find(|c| c.role == "VP").unwrap();
    assert_eq!(vp.spent_minutes, 30);

    teardown(&root);
}

#[test]
fn test_progress_no_monthly_file_returns_empty() {
    let tmp = std::env::temp_dir().join("logbook_int_cp_nofile");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let progress = tauri_app_lib::commands::get_commitment_progress(
        tmp.to_string_lossy().into_owned(),
        2026,
        6,
    )
    .unwrap();

    assert!(progress.is_empty());

    teardown(&tmp);
}
```

- [ ] **Step 2: Run integration tests**

```bash
cd src-tauri && cargo test --test commitment_progress_integration
```

Expected: PASS — all 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/commitment_progress_integration.rs
git commit -m "test: add integration tests for get_commitment_progress"
```

---

### Task 5: Add TypeScript types

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Add `CommitmentProgress` and `GoalProgress` interfaces**

Insert after the existing `Commitment` interface:

```typescript
export interface CommitmentProgress {
  role: string;
  allocation_minutes: number;
  spent_minutes: number;
  goals: GoalProgress[];
}

export interface GoalProgress {
  name: string;
  spent_minutes: number;
}
```

- [ ] **Step 2: Verify TypeScript compilation**

```bash
pnpm vue-tsc --noEmit
```

Expected: No new errors.

- [ ] **Step 3: Commit**

```bash
git add src/types.ts
git commit -m "feat: add CommitmentProgress and GoalProgress TypeScript types"
```

---

### Task 6: Add `commitmentProgress` to store

**Files:**
- Modify: `src/stores/useStore.ts`

- [ ] **Step 1: Add field to `AppStore` interface**

Add after `commitments: Commitment[]` (line 10):

```typescript
import type { CommitmentProgress } from "../types";
```

And add field:

```typescript
  commitments: Commitment[];
  commitmentProgress: CommitmentProgress[];
```

- [ ] **Step 2: Initialize field in `createStore()`**

Add after `commitments: []` (line 29):

```typescript
    commitments: [],
    commitmentProgress: [],
```

- [ ] **Step 3: Verify TypeScript compilation**

```bash
pnpm vue-tsc --noEmit
```

Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add src/stores/useStore.ts
git commit -m "feat: add commitmentProgress to store"
```

---

### Task 7: Add `makeCommitmentProgress` test fixture

**Files:**
- Modify: `src/__tests__/mocks/fixtures.ts`

- [ ] **Step 1: Add factory function**

Insert after `makeCommitment` (after line 65). Import type at top:

```typescript
import type {
  // ... existing imports
  CommitmentProgress,
  GoalProgress,
} from "../../types";
```

```typescript
export function makeCommitmentProgress(overrides?: Partial<CommitmentProgress>): CommitmentProgress {
  return {
    role: "Developer",
    allocation_minutes: 2400,
    spent_minutes: 0,
    goals: [
      { name: "Ship feature X", spent_minutes: 0 },
      { name: "Code review", spent_minutes: 0 },
    ],
    ...overrides,
  };
}
```

- [ ] **Step 2: Update the import at the top of fixtures.ts**

Change:
```typescript
import type {
  Entry,
  Config,
  Dimension,
  Commitment,
  DayFile,
  ConfigErrorDetail,
} from "../../types";
```

To:
```typescript
import type {
  Entry,
  Config,
  Dimension,
  Commitment,
  DayFile,
  ConfigErrorDetail,
  CommitmentProgress,
} from "../../types";
```

- [ ] **Step 3: Verify TypeScript compilation**

```bash
pnpm vue-tsc --noEmit
```

Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add src/__tests__/mocks/fixtures.ts
git commit -m "test: add makeCommitmentProgress fixture factory"
```

---

### Task 8: Rewrite CommitmentsPanel component

**Files:**
- Modify: `src/components/CommitmentsPanel.vue`

- [ ] **Step 1: Replace script block with new logic**

Replace the entire `<script setup>` block:

```typescript
import { computed } from "vue";
import type { CommitmentProgress } from "../types";
import { formatDuration } from "../utils/format";

const props = defineProps<{
  progress: CommitmentProgress[];
  selectedYear: number;
  selectedMonth: number;
}>();

function pct(spent: number, alloc: number): string {
  if (alloc === 0) return "0%";
  return Math.min(100, Math.round((spent / alloc) * 100)) + "%";
}

function barColor(spent: number, alloc: number): string {
  if (alloc === 0) return "bg-gray-300";

  const spentRatio = spent / alloc;

  // 超预算 → 红
  if (spentRatio > 1) return "bg-red-500";

  const elapsed = elapsedRatio();

  // 颜色参照时间进度，宽度参照固定预算
  if (spentRatio < elapsed * 0.6) return "bg-orange-500";
  if (spentRatio > elapsed * 1.4) return "bg-yellow-500";
  return "bg-green-500";
}

function elapsedRatio(): number {
  const now = new Date();
  const isCurrentMonth =
    props.selectedYear === now.getFullYear() &&
    props.selectedMonth === now.getMonth() + 1;

  if (isCurrentMonth) {
    // month is 1-based; new Date(year, month, 0) = last day of (month-1)
    const daysInMonth = new Date(props.selectedYear, props.selectedMonth, 0).getDate();
    return now.getDate() / daysInMonth;
  }
  return 1.0;
}
```

- [ ] **Step 2: Update template to use new props**

Replace the `<template>` block:

```html
<template>
  <div v-if="progress.length > 0" class="bg-white rounded-lg shadow-sm p-4">
    <h3 class="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-3">Commitments</h3>
    <div v-for="s in progress" :key="s.role" class="mb-4 last:mb-0">
      <div class="flex justify-between items-center text-sm mb-1">
        <span class="font-semibold text-gray-700">{{ s.role }}</span>
        <span class="text-gray-500 text-xs">
          {{ formatDuration(s.spent_minutes) }} / {{ (s.allocation_minutes / 60).toFixed(1) }}h
        </span>
      </div>
      <div class="h-1.5 bg-gray-100 rounded-full overflow-hidden mb-2">
        <div
          :class="barColor(s.spent_minutes, s.allocation_minutes)"
          class="h-full rounded-full transition-all"
          :style="{ width: pct(s.spent_minutes, s.allocation_minutes) }"
        />
      </div>
      <div class="ml-2 flex flex-col gap-0.5 text-xs">
        <div
          v-for="g in s.goals"
          :key="g.name"
          class="flex justify-between"
          :class="g.spent_minutes > 0 ? 'text-gray-600' : 'text-gray-300'"
        >
          <span>{{ g.name }}</span>
          <span v-if="g.spent_minutes > 0" class="font-medium text-gray-700">{{ formatDuration(g.spent_minutes) }}</span>
          <span v-else>0m</span>
        </div>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 3: Verify TypeScript compilation**

```bash
pnpm vue-tsc --noEmit
```

Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/CommitmentsPanel.vue
git commit -m "refactor: switch CommitmentsPanel to monthly cumulative progress"
```

---

### Task 9: Rewrite CommitmentsPanel tests

**Files:**
- Modify: `src/__tests__/components/CommitmentsPanel.test.ts`

- [ ] **Step 1: Replace test file completely**

Replace the entire file content:

```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount } from "@vue/test-utils";
import CommitmentsPanel from "../../components/CommitmentsPanel.vue";
import { makeCommitmentProgress } from "../mocks/fixtures";
import type { CommitmentProgress } from "../../types";

function mountPanel(progress: CommitmentProgress[], selectedYear = 2026, selectedMonth = 6) {
  return mount(CommitmentsPanel, {
    props: { progress, selectedYear, selectedMonth },
  });
}

// Helper to create a progress entry with specific spent values
function goalProgress(name: string, spentMinutes: number) {
  return { name, spent_minutes: spentMinutes };
}

// ============================================================

describe("CommitmentsPanel", () => {
  it("renders nothing when progress empty", () => {
    const wrapper = mountPanel([]);
    expect(wrapper.find(".bg-white").exists()).toBe(false);
  });

  it("renders each commitment role", () => {
    const progress = [
      makeCommitmentProgress({ role: "Developer" }),
      makeCommitmentProgress({ role: "Director" }),
    ];
    const wrapper = mountPanel(progress);
    const text = wrapper.text();
    expect(text).toContain("Developer");
    expect(text).toContain("Director");
  });

  it("shows monthly allocation in hours", () => {
    // 2400 minutes = 40.0h
    const progress = [makeCommitmentProgress({ allocation_minutes: 2400 })];
    const wrapper = mountPanel(progress);
    expect(wrapper.text()).toContain("40.0h");
  });

  it("shows spent / allocation ratio text", () => {
    const progress = [makeCommitmentProgress({ spent_minutes: 150, allocation_minutes: 2400 })];
    const wrapper = mountPanel(progress);
    // formatDuration(150) = "2h 30m"
    expect(wrapper.text()).toContain("2h 30m");
  });

  it("progress bar width reflects percentage", () => {
    // 1200 spent out of 2400 allocated = 50%
    const progress = [makeCommitmentProgress({ spent_minutes: 1200, allocation_minutes: 2400 })];
    const wrapper = mountPanel(progress);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.attributes("style")).toContain("width: 50%");
  });

  it("clamps progress bar width to 100%", () => {
    // 3000 spent > 2400 allocated → clamped to 100%
    const progress = [makeCommitmentProgress({ spent_minutes: 3000, allocation_minutes: 2400 })];
    const wrapper = mountPanel(progress);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.attributes("style")).toContain("width: 100%");
  });

  it("red bar when spent > allocation", () => {
    const progress = [makeCommitmentProgress({ spent_minutes: 3000, allocation_minutes: 2400 })];
    const wrapper = mountPanel(progress);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.classes()).toContain("bg-red-500");
  });

  it("renders goal breakdown with names and times", () => {
    const progress = [
      makeCommitmentProgress({
        goals: [
          goalProgress("Code review", 75),
          goalProgress("Ship feature X", 120),
        ],
      }),
    ];
    const wrapper = mountPanel(progress);
    const text = wrapper.text();
    expect(text).toContain("Code review");
    expect(text).toContain("1h 15m");
    expect(text).toContain("Ship feature X");
    expect(text).toContain("2h 0m");
  });

  it("shows zero goal as '0m' with gray text", () => {
    const progress = [makeCommitmentProgress()];
    const wrapper = mountPanel(progress);
    const text = wrapper.text();
    expect(text).toContain("0m");

    // Find the goal with 0 spent — should have text-gray-300 class
    const goalRow = wrapper.find(".text-gray-300");
    expect(goalRow.exists()).toBe(true);
    expect(goalRow.text()).toContain("0m");
  });

  it("zero allocation shows 0% width and gray bar", () => {
    const progress = [makeCommitmentProgress({ allocation_minutes: 0, spent_minutes: 60 })];
    const wrapper = mountPanel(progress);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.attributes("style")).toContain("width: 0%");
    expect(bar.classes()).toContain("bg-gray-300");
  });

  it("orange bar when spent significantly behind elapsed time (current month)", () => {
    // Mock date to June 15 (50% elapsed), spent is 0 → way behind → orange
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 5, 15)); // month is 0-indexed: 5 = June

    const progress = [makeCommitmentProgress({
      spent_minutes: 0,
      allocation_minutes: 2400, // 40h
    })];
    const wrapper = mountPanel(progress, 2026, 6);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.classes()).toContain("bg-orange-500");

    vi.useRealTimers();
  });

  it("green bar when spent is in sync with elapsed time (current month)", () => {
    vi.useFakeTimers();
    // June 15: ~50% elapsed. 1200/2400 = 50% → within [50%*0.6, 50%*1.4] = [30%, 70%] → green
    vi.setSystemTime(new Date(2026, 5, 15));

    const progress = [makeCommitmentProgress({
      spent_minutes: 1200,
      allocation_minutes: 2400,
    })];
    const wrapper = mountPanel(progress, 2026, 6);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.classes()).toContain("bg-green-500");

    vi.useRealTimers();
  });

  it("yellow bar when spent is ahead of elapsed time (current month)", () => {
    vi.useFakeTimers();
    // June 5: ~17% elapsed. 40% spent → > 17% * 1.4 = 23.3% → yellow
    vi.setSystemTime(new Date(2026, 5, 5));

    const progress = [makeCommitmentProgress({
      spent_minutes: 960, // 40% of 2400
      allocation_minutes: 2400,
    })];
    const wrapper = mountPanel(progress, 2026, 6);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.classes()).toContain("bg-yellow-500");

    vi.useRealTimers();
  });

  it("historical month uses 100% elapsed (color based on total completion)", () => {
    // May 2026 — historical month. elapsed = 100%.
    // 50% spent < 60% elapsed → orange
    const progress = [makeCommitmentProgress({
      spent_minutes: 1200, // 50% of 2400
      allocation_minutes: 2400,
    })];
    const wrapper = mountPanel(progress, 2026, 5);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.classes()).toContain("bg-orange-500");

    // 95% spent → within [60%, 140%] → green
    const progress2 = [makeCommitmentProgress({
      spent_minutes: 2280, // 95%
      allocation_minutes: 2400,
    })];
    const wrapper2 = mountPanel(progress2, 2026, 5);
    const bar2 = wrapper2.find(".h-1\\.5 > div");
    expect(bar2.classes()).toContain("bg-green-500");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
pnpm vitest run src/__tests__/components/CommitmentsPanel.test.ts
```

Expected: FAIL — old tests reference props that no longer exist.

Note: They should fail because we haven't updated them yet. Actually, since we already updated the component in Task 8, let's run the tests now to see them pass with the new component.

Wait — the tests were written for the OLD component. The component was already rewritten in Task 8. So these new tests should pass. Let me adjust: run the new tests.

```bash
pnpm vitest run src/__tests__/components/CommitmentsPanel.test.ts
```

Expected: PASS — all 12 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/__tests__/components/CommitmentsPanel.test.ts
git commit -m "test: rewrite CommitmentsPanel tests for monthly cumulative progress"
```

---

### Task 10: Update TodayView to load commitment progress

**Files:**
- Modify: `src/components/TodayView.vue`

- [ ] **Step 1: Add `loadCommitmentProgress` function and integrate into data flow**

In the `<script setup>` block, add after `loadPeriod` function (after line 31):

```typescript
async function loadCommitmentProgress() {
  const d = store.currentDate;
  const year = parseInt(d.slice(0, 4));
  const month = parseInt(d.slice(5, 7));
  try {
    store.commitmentProgress = (await invoke("get_commitment_progress", {
      rootPath: store.rootPath,
      year,
      month,
    })) as CommitmentProgress[];
  } catch (e) {
    logError("TodayView.loadCommitmentProgress", e);
    store.commitmentProgress = [];
  }
}
```

Add import for `CommitmentProgress` at the top:

```typescript
import type { Entry, DayFile, CommitmentProgress } from "../types";
```

Update the `loadPeriod` function to also load commitment progress. Add at the end of `loadPeriod` (after line 30, before the closing `}`):

```typescript
  await loadCommitmentProgress();
```

But wait — `loadCommitmentProgress` calls `invoke` and `loadPeriod` also calls `invoke` in a loop. These should run sequentially to avoid issues. Since `loadPeriod` already uses `await` in its loop, we can just await `loadCommitmentProgress` at the end.

Actually `loadPeriod` doesn't currently await `loadCommitmentProgress`. Let me see the flow:

```typescript
async function loadPeriod() {
  const dates = datesInPeriod(store.currentDate, store.granularity);
  const map: Record<string, Entry[]> = {};
  for (const date of dates) {
    // ...
  }
  store.periodEntries = map;
  // ...
}
```

Add `await loadCommitmentProgress();` at the end of `loadPeriod()`, after setting `store.today`:

```typescript
  if (store.currentDate in map) {
    store.today = { note: null, entries: map[store.currentDate] };
  }
  await loadCommitmentProgress();
```

- [ ] **Step 2: Create a helper to extract year/month from the current date**

Add a computed or parse inline in `loadCommitmentProgress`. We already parse in the function body above.

- [ ] **Step 3: Update CommitmentsPanel usage in template**

Change lines 180-183:

```html
      <CommitmentsPanel
        :commitments="store.commitments"
        :entries="store.today?.entries || []"
      />
```

To:

```html
      <CommitmentsPanel
        :progress="store.commitmentProgress"
        :selected-year="parseInt(store.currentDate.slice(0, 4))"
        :selected-month="parseInt(store.currentDate.slice(5, 7))"
      />
```

- [ ] **Step 4: Refresh commitment progress after entry mutations**

Add `await loadCommitmentProgress();` after each successful mutation:

In `handleUpdateDimensions` (after line 47 `store.today = df;`):
```typescript
    store.today = df;
    await loadCommitmentProgress();
```

In `handleUpdateEntry` (after line 72 `store.today = df;`):
```typescript
    store.today = df;
    await loadCommitmentProgress();
```

In `handleDeleteEntry` (after the timer callback, line 96, after `await invoke("delete_entry", ...)`):
```typescript
      await invoke("delete_entry", { rootPath: store.rootPath, date: store.currentDate, entryId });
      await loadCommitmentProgress();
```

- [ ] **Step 5: Verify TypeScript compilation**

```bash
pnpm vue-tsc --noEmit
```

Expected: No errors.

- [ ] **Step 6: Commit**

```bash
git add src/components/TodayView.vue
git commit -m "feat: integrate get_commitment_progress into TodayView"
```

---

### Task 11: Update App.vue for commitment progress on init and watcher

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: Add `loadCommitmentProgress` function**

Add after the `initApp` function (after line 93):

```typescript
async function loadCommitmentProgress() {
  if (store.screen !== "ready" || !store.rootPath) return;
  const d = store.currentDate;
  const year = parseInt(d.slice(0, 4));
  const month = parseInt(d.slice(5, 7));
  try {
    store.commitmentProgress = (await invoke("get_commitment_progress", {
      rootPath: store.rootPath,
      year,
      month,
    })) as import("./types").CommitmentProgress[];
  } catch (e) {
    logError("App.loadCommitmentProgress", e);
  }
}
```

- [ ] **Step 2: Call after initApp succeeds**

In `initApp`, after setting `store.screen = "ready"` (line 84), add:

```typescript
        store.screen = "ready";
        await loadCommitmentProgress();
```

- [ ] **Step 3: Call in watcher event handlers**

In the `commitments-changed` listener (line 38), add after `initApp()`:

Actually, the listener already calls `initApp()` which will now call `loadCommitmentProgress`. So no change needed there.

In the focus change handler (line 47-49), the logic already calls `initApp()` for date change. Good.

- [ ] **Step 4: Verify TypeScript compilation**

```bash
pnpm vue-tsc --noEmit
```

Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add src/App.vue
git commit -m "feat: load commitment progress on init and watcher events"
```

---

### Task 12: Update TodayView tests

**Files:**
- Modify: `src/__tests__/components/TodayView.test.ts`

- [ ] **Step 1: Update mock for `get_commitment_progress`**

In the `beforeEach` block (line 43), update `mockInvoke.mockResolvedValue` to handle `get_commitment_progress`:

```typescript
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_commitment_progress") return [];
      return makeDayFile();
    });
  });
```

- [ ] **Step 2: Add test that CommitmentProgress is called after loadPeriod**

Add to the describe block:

```typescript
  it("loadPeriod: calls get_commitment_progress after loading entries", async () => {
    const { wrapper, store } = mountToday({ currentDate: todayStr, granularity: "day" });
    const nav = wrapper.findComponent({ name: "DateNavigator" });
    await nav.vm.$emit("navigate");

    expect(mockInvoke).toHaveBeenCalledWith("get_commitment_progress", expect.objectContaining({
      rootPath: "/test",
      year: TODAY.getFullYear(),
      month: TODAY.getMonth() + 1,
    }));
  });

  it("loadPeriod: stores commitment progress in store", async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_commitment_progress") return [
        { role: "Dev", allocation_minutes: 1800, spent_minutes: 300, goals: [] },
      ];
      return makeDayFile();
    });

    const { wrapper, store } = mountToday({ currentDate: todayStr, granularity: "day" });
    const nav = wrapper.findComponent({ name: "DateNavigator" });
    await nav.vm.$emit("navigate");

    expect(store.commitmentProgress).toEqual([
      { role: "Dev", allocation_minutes: 1800, spent_minutes: 300, goals: [] },
    ]);
  });
```

- [ ] **Step 3: Update CommitmentsPanel props assertion**

Add a test:

```typescript
  it("passes commitmentProgress to CommitmentsPanel", () => {
    const progress = [{ role: "Dev", allocation_minutes: 1800, spent_minutes: 300, goals: [] }];
    const { wrapper } = mountToday({ commitmentProgress: progress });

    const panel = wrapper.findComponent({ name: "CommitmentsPanel" });
    expect(panel.props("progress")).toEqual(progress);
  });
```

- [ ] **Step 4: Run tests**

```bash
pnpm vitest run src/__tests__/components/TodayView.test.ts
```

Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/__tests__/components/TodayView.test.ts
git commit -m "test: update TodayView tests for get_commitment_progress"
```

---

### Task 13: Final verification

**Files:** None (verification only)

- [ ] **Step 1: Run all Rust tests**

```bash
cd src-tauri && cargo test
```

Expected: All tests pass (existing + new).

- [ ] **Step 2: Run all frontend tests**

```bash
pnpm vitest run
```

Expected: All tests pass.

- [ ] **Step 3: Run TypeScript check**

```bash
pnpm vue-tsc --noEmit
```

Expected: No errors.

- [ ] **Step 4: Run full Rust check**

```bash
cd src-tauri && cargo check
```

Expected: No errors.

- [ ] **Step 5: Commit if any config changes**

No commit needed unless previous commits missed something.
