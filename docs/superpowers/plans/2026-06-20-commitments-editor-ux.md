# Commitments Editor UX 重设计 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 commitments 编辑从侧栏原始内联表单升级为居中 modal 编辑器——精致控件、招牌进度条、拖拽排序、5h 步进、删除约束、超额软警告，且严格贴合 `tokens.css`。

**Architecture:** 新增 `CommitmentsModal.vue`（Teleport 到 body 的轻量 modal，不引入 dialog 库），由 `CommitmentsPanel.vue` 在「Edit / Set up commitments」时打开。Modal 用内部 `RoleRow/GoalRow` 工作副本编辑（含稳定 `key` 供拖拽），保存时映射回 `Commitment[]` 调既有 `set_commitments`。删除约束由前端依据 `CommitmentProgress`（logged 时长）主动禁用，后端保留「删有 entry 的 goal 即拒绝」作防线。后端 `validate_commitments` 扩展 role 名唯一 + goal 名全局唯一。

**Tech Stack:** Vue 3 (`<script setup>`) + TypeScript + Tailwind v4（token 变量）+ Vitest/@vue/test-utils；Rust（命令校验）；新增前端依赖 `vuedraggable@4`。

**关键参考：**
- Spec：`docs/superpowers/specs/2026-06-20-commitments-editor-ux-design.md`
- 定稿 mockup：`docs/superpowers/specs/2026-06-20-commitments-editor-ux-mockup.html`
- Tokens：`src/assets/tokens.css`
- 既有：`src-tauri/src/commands.rs`（`set_commitments` L595、`validate_commitments` L731、测试 L1225+）

**全局约定（每个前端组件都适用）：**
- 字号用 `text-[length:var(--app-text-*)]`（必须带 `length:`，否则 `tailwind-token-usage.test.ts` 失败）。颜色用 `text-[var(--color-*)]` / `bg-[var(--color-*)]`。
- 数字用 `class="mono"`。
- 每个 Task 末尾 commit。

**文件结构：**
- 改：`src-tauri/src/commands.rs` — `validate_commitments` + 同文件测试
- 改：`package.json` — 加 `vuedraggable`
- 建：`src/components/composite/CommitmentsModal.vue`
- 建：`src/__tests__/components/composite/CommitmentsModal.test.ts`
- 改：`src/components/CommitmentsPanel.vue`
- 改：`src/__tests__/components/CommitmentsPanel.test.ts`
- 删：`src/components/composite/CommitmentsEditor.vue` 与其测试
- 改：`SPEC.md`、`src-tauri/CLAUDE.md`

---

## Task 1: 后端校验扩展（role 名唯一 + goal 名全局唯一）

**Files:**
- Modify: `src-tauri/src/commands.rs:731-756`（`validate_commitments`）
- Modify: `src-tauri/src/commands.rs:1281-1289`（现有 `test_validate_commitments_duplicate_goal_same_role`）
- Test: 同文件 `#[cfg(test)] mod tests`

- [ ] **Step 1: 更新同 role 重复测试为全局语义，新增 across-roles + role 重复 + reorder 守卫测试**

替换 `test_validate_commitments_duplicate_goal_same_role`（L1281-1289）为以下内容：

```rust
    #[test]
    fn test_validate_commitments_duplicate_goal_same_role() {
        let c = make_commitments(vec![("Dev", 40, vec!["Ship it", "Ship it"])]);
        let result = validate_commitments(&c);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("already exists"));
        assert!(err.contains("Ship it"));
    }

    #[test]
    fn test_validate_commitments_duplicate_goal_across_roles() {
        let c = make_commitments(vec![
            ("Dev", 40, vec!["Shared goal"]),
            ("TL", 20, vec!["Shared goal"]),
        ]);
        let result = validate_commitments(&c);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("already exists"));
        assert!(err.contains("Shared goal"));
    }

    #[test]
    fn test_validate_commitments_duplicate_role() {
        let c = make_commitments(vec![
            ("Dev", 40, vec!["A"]),
            ("Dev", 20, vec!["B"]),
        ]);
        let result = validate_commitments(&c);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Role"));
        assert!(err.contains("already exists"));
        assert!(err.contains("Dev"));
    }

    // Guard: reordering goals within a role (same set, different order) must
    // NOT be misread as a rename by detect_goal_changes.
    #[test]
    fn test_detect_goal_changes_reorder_is_not_rename() {
        let old = make_commitments(vec![("Dev", 40, vec!["A", "B", "C"])]);
        let new = make_commitments(vec![("Dev", 40, vec!["C", "A", "B"])]);
        let changes = detect_goal_changes(&old, &new);
        assert!(changes.renames.is_empty(), "reorder must not produce renames");
        assert!(changes.deleted.is_empty(), "reorder must not produce deletions");
    }
```

- [ ] **Step 2: 运行确认 across-roles / duplicate-role 失败**

Run: `cd src-tauri && cargo test validate_commitments`
Expected: `..._duplicate_goal_across_roles` 与 `..._duplicate_role` FAIL（当前仅查同 role 内 + 不查 role 重复）。reorder 守卫预计 PASS（既有 deleted 用全局 set-diff）；若 FAIL 说明 rename 启发式对重排敏感，需在 Step 3 顺带修正 `detect_goal_changes` 使其忽略纯重排。

- [ ] **Step 3: 扩展 `validate_commitments`**

替换 `src-tauri/src/commands.rs:731-756` 整个函数为：

```rust
fn validate_commitments(commitments: &[Commitment]) -> Result<(), String> {
    if commitments.is_empty() {
        return Err("At least one role is required".to_string());
    }
    let mut role_set = std::collections::HashSet::new();
    let mut goal_set = std::collections::HashSet::new();
    for c in commitments {
        let role = c.role.trim();
        if role.is_empty() {
            return Err("Role name cannot be empty".to_string());
        }
        if !role_set.insert(role.to_string()) {
            return Err(format!("Role '{}' already exists", role));
        }
        if c.allocation == 0 {
            return Err(format!(
                "Allocation for '{}' must be greater than 0",
                c.role
            ));
        }
        for g in &c.goals {
            let goal = g.trim();
            if goal.is_empty() {
                return Err("Goal name cannot be empty".to_string());
            }
            if !goal_set.insert(goal.to_string()) {
                return Err(format!("Goal '{}' already exists", goal));
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 4: 运行确认全部通过**

Run: `cd src-tauri && cargo test validate_commitments && cargo test detect_goal_changes`
Expected: 全部 PASS。再跑 `cd src-tauri && cargo test` 确认无回归。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(backend): role + global goal uniqueness in validate_commitments"
```

---

## Task 2: 引入 vuedraggable 依赖

**Files:**
- Modify: `package.json:15-23`

- [ ] **Step 1: 安装**

Run: `pnpm add vuedraggable@^4.1.0`

- [ ] **Step 2: 验证**

Run: `pnpm vue-tsc --noEmit`
Expected: 无错误。确认 `package.json` `dependencies` 含 `"vuedraggable": "^4.1.0"`。

- [ ] **Step 3: Commit**

```bash
git add package.json pnpm-lock.yaml
git commit -m "build: add vuedraggable for commitments drag-reorder"
```

---

## Task 3: CommitmentsModal.vue 基础（渲染 + 可拖拽 role/goal + 编辑副本 + Save/Cancel）

骨架：Teleport 遮罩、header、可拖拽的 role 卡片列表（每张含 role 名输入、allocation 数字输入、可拖拽 goal 行）、Add Goal / Add Role、Cancel / Save。`RoleRow/GoalRow` 工作副本含稳定 `key`（拖拽 item-key）。

**Files:**
- Create: `src/components/composite/CommitmentsModal.vue`
- Test: `src/__tests__/components/composite/CommitmentsModal.test.ts`

- [ ] **Step 1: 写失败测试**

创建 `src/__tests__/components/composite/CommitmentsModal.test.ts`：

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { invoke } from "@tauri-apps/api/core";
import CommitmentsModal from "../../../components/composite/CommitmentsModal.vue";
import { makeCommitment, makeCommitmentProgress } from "../../mocks/fixtures";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// vuedraggable stub: render the #item slot for each model element (no real DnD in jsdom)
vi.mock("vuedraggable", () => ({
  default: {
    name: "draggable",
    props: ["modelValue", "itemKey", "handle", "group", "tag", "animation"],
    emits: ["update:modelValue"],
    render() {
      const items = (this as any).modelValue || [];
      const slots = (this as any).$slots;
      return items.map((element: any, index: number) =>
        slots.item ? slots.item({ element, index }) : null
      );
    },
  },
}));

const baseProps = () => ({
  open: true,
  commitments: [
    makeCommitment({ role: "Developer", allocation: 40, goals: ["Ship onboarding v2", "Review auth PR"] }),
  ],
  progress: [
    makeCommitmentProgress({
      role: "Developer", allocation_minutes: 2400, spent_minutes: 870,
      goals: [
        { name: "Ship onboarding v2", spent_minutes: 865 },
        { name: "Review auth PR", spent_minutes: 5 },
      ],
    }),
  ],
  rootPath: "/tmp", selectedYear: 2026, selectedMonth: 6,
});

function mountModal(overrides = {}) {
  return mount(CommitmentsModal, {
    props: { ...baseProps(), ...overrides },
    global: { stubs: { teleport: true } },
  });
}

beforeEach(() => { (invoke as any).mockReset?.(); (invoke as any).mockResolvedValue?.([]); });

describe("CommitmentsModal — base", () => {
  it("renders role and goal values from props", () => {
    const w = mountModal();
    expect((w.find("[data-test='role-name']").element as HTMLInputElement).value).toBe("Developer");
    const goals = w.findAll("[data-test='goal-name']").map(g => (g.element as HTMLInputElement).value);
    expect(goals).toContain("Ship onboarding v2");
    expect(goals).toContain("Review auth PR");
  });

  it("renders a drag handle per role and per goal", () => {
    const w = mountModal();
    expect(w.findAll("[data-test='drag-grip-role']").length).toBe(1);
    expect(w.findAll("[data-test='drag-grip-goal']").length).toBe(2);
  });

  it("adds a goal row on + Add Goal", async () => {
    const w = mountModal();
    const before = w.findAll("[data-test='goal-name']").length;
    await w.find("[data-test='add-goal']").trigger("click");
    expect(w.findAll("[data-test='goal-name']").length).toBe(before + 1);
  });

  it("adds a role on + Add Role", async () => {
    const w = mountModal();
    await w.find("[data-test='add-role']").trigger("click");
    expect(w.findAll("[data-test='role-name']").length).toBe(2);
  });

  it("Save calls set_commitments with trimmed commitments and emits saved+close", async () => {
    const w = mountModal();
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).toHaveBeenCalledWith("set_commitments", expect.objectContaining({
      rootPath: "/tmp", year: 2026, month: 6,
      commitments: [{ role: "Developer", allocation: 40, goals: ["Ship onboarding v2", "Review auth PR"] }],
    }));
    expect(w.emitted("saved")).toBeTruthy();
    expect(w.emitted("close")).toBeTruthy();
  });

  it("Cancel emits close without invoking backend", async () => {
    const w = mountModal();
    await w.find("[data-test='cancel']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.emitted("close")).toBeTruthy();
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts`
Expected: FAIL（组件不存在）。

- [ ] **Step 3: 创建组件**

创建 `src/components/composite/CommitmentsModal.vue`：

```vue
<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import draggable from "vuedraggable";
import type { Commitment, CommitmentProgress } from "../../types";

interface GoalRow { name: string; origName: string | null; key: number }
interface RoleRow { role: string; allocation: number; goals: GoalRow[]; origRole: string | null; key: number }

const props = defineProps<{
  open: boolean;
  commitments: Commitment[];
  progress: CommitmentProgress[];
  rootPath: string;
  selectedYear: number;
  selectedMonth: number;
}>();

const emit = defineEmits<{ saved: []; close: [] }>();

const NEW_ROLE_ALLOC = 5;
let _key = 0;
const nextKey = () => ++_key;

const draft = ref<RoleRow[]>([]);
const error = ref("");
const saving = ref(false);

function buildDraft() {
  draft.value = props.commitments.map(c => ({
    role: c.role, allocation: c.allocation, origRole: c.role, key: nextKey(),
    goals: c.goals.map(g => ({ name: g, origName: g, key: nextKey() })),
  }));
  error.value = "";
}
watch(() => props.open, (o) => { if (o) buildDraft(); }, { immediate: true });

function toCommitments(rows: RoleRow[]): Commitment[] {
  return rows.map(r => ({
    role: r.role.trim(),
    allocation: r.allocation,
    goals: r.goals.map(g => g.name.trim()).filter(n => n !== ""),
  }));
}

function addRole() {
  draft.value.push({ role: "", allocation: NEW_ROLE_ALLOC, origRole: null, key: nextKey(), goals: [{ name: "", origName: null, key: nextKey() }] });
}
function addGoal(ri: number) {
  draft.value[ri].goals.push({ name: "", origName: null, key: nextKey() });
}

const monthLabel = computed(() =>
  new Date(props.selectedYear, props.selectedMonth - 1, 1).toLocaleDateString("en-US", { month: "long", year: "numeric" })
);

async function save() {
  saving.value = true;
  error.value = "";
  try {
    await invoke("set_commitments", {
      rootPath: props.rootPath,
      year: props.selectedYear,
      month: props.selectedMonth,
      commitments: toCommitments(draft.value),
    });
    emit("saved");
    emit("close");
  } catch (e) {
    error.value = typeof e === "string" ? e : String(e);
  } finally {
    saving.value = false;
  }
}
function cancel() { emit("close"); }
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      data-test="overlay" tabindex="-1"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
    >
      <div
        role="dialog" aria-modal="true"
        class="relative w-[660px] max-w-[92vw] max-h-[88vh] flex flex-col bg-[var(--color-surface)]
               border border-[var(--color-border-form)] rounded-[var(--radius-lg)]
               shadow-[var(--shadow-popover)] overflow-hidden"
      >
        <!-- Header -->
        <div class="flex justify-between items-start px-[28px] pt-[24px] pb-[16px] border-b border-[var(--color-divider)]">
          <div>
            <div class="text-[length:var(--app-text-xl)] font-bold text-[var(--color-text-primary)] tracking-[-0.3px]">Edit Commitments</div>
            <div class="text-[length:var(--app-text-xs)] text-[var(--color-text-muted)] mt-[2px]">{{ monthLabel }}</div>
          </div>
        </div>

        <!-- Body -->
        <div class="px-[28px] pt-[16px] pb-[4px] overflow-y-auto">
          <draggable v-model="draft" item-key="key" handle=".drag-grip-role" tag="div" :animation="150">
            <template #item="{ element: r, index: ri }">
              <div class="bg-[var(--color-page-bg)] border border-[var(--color-divider)] rounded-[var(--radius-form-lg)] p-[16px] mb-[12px]" data-test="role-card">
                <div class="flex items-center gap-[8px]">
                  <span data-test="drag-grip-role" class="drag-grip-role cursor-grab text-[var(--color-text-disabled)] select-none px-[2px]">⠿</span>
                  <input
                    v-model="r.role" data-test="role-name" placeholder="Role"
                    class="flex-1 px-[10px] py-[6px] border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                           text-[length:var(--app-text-base)] font-semibold text-[var(--color-text-primary)]
                           bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                  />
                  <input
                    v-model.number="r.allocation" type="number" data-test="alloc"
                    class="w-[42px] text-center px-[4px] py-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                           text-[length:var(--app-text-base)] font-semibold text-[var(--color-text-primary)] mono
                           bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                  />
                  <span class="text-[length:var(--app-text-xs-alt)] text-[var(--color-text-muted)]">h</span>
                </div>

                <div class="mt-[12px]">
                  <draggable v-model="r.goals" item-key="key" handle=".drag-grip-goal" tag="div" class="flex flex-col gap-[8px]" :animation="150">
                    <template #item="{ element: g, index: gi }">
                      <div class="flex items-center gap-[8px]">
                        <span data-test="drag-grip-goal" class="drag-grip-goal cursor-grab text-[var(--color-text-disabled)] select-none px-[2px]">⠿</span>
                        <input
                          v-model="g.name" data-test="goal-name" placeholder="Goal name"
                          class="flex-1 px-[10px] py-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                                 text-[length:var(--app-text-sm)] text-[var(--color-text-secondary)]
                                 bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                        />
                        <button
                          data-test="goal-remove"
                          class="text-[length:var(--app-text-base)] text-[var(--color-text-disabled)] hover:text-[var(--color-danger)] cursor-pointer px-[4px]"
                          @click="r.goals.splice(gi, 1)"
                        >&times;</button>
                      </div>
                    </template>
                  </draggable>
                  <button
                    data-test="add-goal"
                    class="self-start mt-[8px] text-[length:var(--app-text-xs)] font-medium text-[var(--color-brand-link)] cursor-pointer hover:underline"
                    @click="addGoal(ri)"
                  >+ Add Goal</button>
                </div>
              </div>
            </template>
          </draggable>

          <button
            data-test="add-role"
            class="my-[4px] py-[6px] text-[length:var(--app-text-sm)] font-semibold text-[var(--color-brand-link)] cursor-pointer hover:underline"
            @click="addRole"
          >+ Add Role</button>
        </div>

        <!-- Footer -->
        <div v-if="error" class="px-[28px] pb-[8px] text-[length:var(--app-text-xs-alt)] text-[var(--color-danger)]">{{ error }}</div>
        <div class="flex justify-end gap-[8px] px-[28px] py-[16px] border-t border-[var(--color-divider)]">
          <button
            data-test="cancel"
            class="text-[length:var(--app-text-sm)] font-semibold text-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)] rounded-[var(--radius-form)] px-[14px] py-[6px] cursor-pointer"
            @click="cancel"
          >Cancel</button>
          <button
            data-test="save" :disabled="saving"
            class="text-[length:var(--app-text-sm)] font-semibold text-white bg-[var(--color-brand-solid)] hover:bg-[var(--color-brand-link)] rounded-[var(--radius-form)] px-[14px] py-[6px] cursor-pointer disabled:opacity-50"
            @click="save"
          >{{ saving ? "Saving…" : "Save" }}</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
```

- [ ] **Step 4: 运行确认通过**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts`
Expected: 6 个测试全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/CommitmentsModal.vue src/__tests__/components/composite/CommitmentsModal.test.ts
git commit -m "feat(ui): CommitmentsModal base — draggable roles/goals, edit copy, save/cancel"
```

---

## Task 4: Allocation 步进控件（±5h、− 最小 5、直接输入、↑↓）

**Files:**
- Modify: `src/components/composite/CommitmentsModal.vue`
- Test: `src/__tests__/components/composite/CommitmentsModal.test.ts`

- [ ] **Step 1: 追加失败测试**

```ts
describe("CommitmentsModal — allocation stepper", () => {
  it("increments by 5 on +", async () => {
    const w = mountModal();
    await w.find("[data-test='alloc-inc']").trigger("click");
    expect((w.find("[data-test='alloc']").element as HTMLInputElement).value).toBe("45");
  });
  it("decrements by 5 on -", async () => {
    const w = mountModal();
    await w.find("[data-test='alloc-dec']").trigger("click");
    expect((w.find("[data-test='alloc']").element as HTMLInputElement).value).toBe("35");
  });
  it("disables - at the 5h floor and never goes below 5", async () => {
    const w = mountModal({
      commitments: [makeCommitment({ role: "Developer", allocation: 5, goals: [] })],
      progress: [makeCommitmentProgress({ role: "Developer", allocation_minutes: 300, spent_minutes: 0, goals: [] })],
    });
    const dec = w.find("[data-test='alloc-dec']");
    expect((dec.element as HTMLButtonElement).disabled).toBe(true);
    await dec.trigger("click");
    expect((w.find("[data-test='alloc']").element as HTMLInputElement).value).toBe("5");
  });
  it("Arrow Up/Down adjusts by 5", async () => {
    const w = mountModal();
    const inp = w.find("[data-test='alloc']");
    await inp.trigger("keydown", { key: "ArrowUp" });
    expect((inp.element as HTMLInputElement).value).toBe("45");
    await inp.trigger("keydown", { key: "ArrowDown" });
    expect((inp.element as HTMLInputElement).value).toBe("40");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts -t "allocation stepper"`
Expected: FAIL。

- [ ] **Step 3: 实现 stepper**

`<script setup>` 中 `addGoal` 之后追加：

```ts
const STEP = 5;
const MIN_ALLOC = 5;
function stepAlloc(ri: number, delta: number) {
  draft.value[ri].allocation = Math.max(MIN_ALLOC, (draft.value[ri].allocation || 0) + delta);
}
function onAllocInput(ri: number, e: Event) {
  const v = Math.floor(Number((e.target as HTMLInputElement).value));
  draft.value[ri].allocation = Number.isFinite(v) && v >= 1 ? v : 1;
}
```

替换模板中 allocation 的单个 `<input data-test="alloc" …>`（连同其后的 `<span>h</span>`）为：

```vue
                  <span class="inline-flex items-center gap-[5px]">
                    <button
                      data-test="alloc-dec" :disabled="r.allocation <= MIN_ALLOC"
                      class="w-[24px] h-[26px] flex items-center justify-center border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-[length:var(--app-text-base)] text-[var(--color-text-secondary)] bg-[var(--color-surface)]
                             hover:border-[var(--color-brand-solid)] hover:text-[var(--color-brand-link)]
                             disabled:text-[var(--color-text-disabled)] disabled:cursor-not-allowed disabled:hover:border-[var(--color-border-form)]
                             cursor-pointer transition-[border-color,color] duration-150"
                      @click="stepAlloc(ri, -STEP)"
                    >&minus;</button>
                    <input
                      :value="r.allocation" type="number" data-test="alloc"
                      class="w-[42px] text-center px-[4px] py-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-[length:var(--app-text-base)] font-semibold text-[var(--color-text-primary)] mono
                             bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                      @input="onAllocInput(ri, $event)"
                      @keydown.up.prevent="stepAlloc(ri, STEP)"
                      @keydown.down.prevent="stepAlloc(ri, -STEP)"
                    />
                    <button
                      data-test="alloc-inc"
                      class="w-[24px] h-[26px] flex items-center justify-center border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-[length:var(--app-text-base)] text-[var(--color-text-secondary)] bg-[var(--color-surface)]
                             hover:border-[var(--color-brand-solid)] hover:text-[var(--color-brand-link)]
                             cursor-pointer transition-[border-color,color] duration-150"
                      @click="stepAlloc(ri, STEP)"
                    >+</button>
                    <span class="text-[length:var(--app-text-xs-alt)] text-[var(--color-text-muted)]">h</span>
                  </span>
```

（`STEP`/`MIN_ALLOC` 为 `<script setup>` 顶层常量，模板可直接引用。）

- [ ] **Step 4: 运行确认通过**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts`
Expected: 全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/CommitmentsModal.vue src/__tests__/components/composite/CommitmentsModal.test.ts
git commit -m "feat(ui): allocation stepper (±5h, floor 5, direct input, arrows)"
```

---

## Task 5: header 汇总 + 招牌进度条 + per-goal logged + 超额 amber

**Files:**
- Modify: `src/components/composite/CommitmentsModal.vue`
- Test: `src/__tests__/components/composite/CommitmentsModal.test.ts`

- [ ] **Step 1: 追加失败测试**

```ts
describe("CommitmentsModal — summary, progress & over-commit", () => {
  it("header shows live committed total and logged total", async () => {
    const w = mountModal(); // committed 40h, logged 870m = 14h 30m
    expect(w.find("[data-test='committed']").text()).toContain("40h");
    expect(w.find("[data-test='logged']").text()).toContain("14h 30m");
    await w.find("[data-test='alloc-inc']").trigger("click"); // 40→45
    expect(w.find("[data-test='committed']").text()).toContain("45h");
  });
  it("shows role logged and per-goal logged", () => {
    const w = mountModal();
    expect(w.find("[data-test='role-spent']").text()).toContain("14h 30m");
    const logged = w.findAll("[data-test='goal-logged']").map(n => n.text());
    expect(logged.some(t => t.includes("14h 25m"))).toBe(true);
    expect(logged.some(t => t.includes("5m"))).toBe(true);
  });
  it("bar fills proportionally to spent/allocation", () => {
    const w = mountModal(); // 870/2400 ≈ 36%
    expect((w.find("[data-test='bar-fill']").element as HTMLElement).style.width).toBe("36%");
  });
  it("turns amber + 'over by' when allocation drops below logged", async () => {
    const w = mountModal();
    const dec = w.find("[data-test='alloc-dec']");
    for (let i = 0; i < 6; i++) await dec.trigger("click"); // 40→10
    expect((w.find("[data-test='bar-fill']").element as HTMLElement).style.width).toBe("100%");
    expect(w.find("[data-test='role-spent']").text()).toContain("over by");
    expect(w.find("[data-test='bar-fill']").classes().join(" ")).toContain("color-warning");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts -t "summary, progress"`
Expected: FAIL。

- [ ] **Step 3: 实现**

`<script setup>` 顶部 import 加：

```ts
import { formatDuration } from "../../utils/format";
```

`monthLabel` 之后追加：

```ts
const committedHours = computed(() => draft.value.reduce((s, r) => s + (r.allocation || 0), 0));
const loggedTotal = computed(() => props.progress.reduce((s, p) => s + p.spent_minutes, 0));

function roleSpent(origRole: string | null): number {
  if (!origRole) return 0;
  return props.progress.find(p => p.role === origRole)?.spent_minutes ?? 0;
}
function goalLogged(origName: string | null): number {
  if (!origName) return 0;
  for (const p of props.progress) {
    const g = p.goals.find(x => x.name === origName);
    if (g) return g.spent_minutes;
  }
  return 0;
}
function barPct(r: RoleRow): number {
  const alloc = r.allocation * 60;
  const spent = roleSpent(r.origRole);
  if (alloc <= 0) return spent > 0 ? 100 : 0;
  return Math.min(100, Math.round((spent / alloc) * 100));
}
function isOver(r: RoleRow): boolean { return roleSpent(r.origRole) > r.allocation * 60; }
function overBy(r: RoleRow): string { return formatDuration(roleSpent(r.origRole) - r.allocation * 60); }
```

header 区块的 `<div class="flex justify-between …">` 内，标题块之后追加右侧汇总：

```vue
          <div class="text-right text-[length:var(--app-text-xs-alt)] text-[var(--color-text-muted)] leading-[1.8]">
            <div>Committed <span data-test="committed" class="mono font-bold text-[var(--color-brand-link)]">{{ committedHours }}h</span></div>
            <div>Logged <span data-test="logged" class="mono font-semibold text-[var(--color-text-primary)]">{{ formatDuration(loggedTotal) }}</span></div>
          </div>
```

role-top 那个 `<div class="flex items-center gap-[8px]">…</div>`（role 名+stepper 行）之后，goals 区块（`<div class="mt-[12px]">`）之前插入进度条：

```vue
                <div class="flex items-center gap-[8px] mt-[8px]">
                  <div class="flex-1 h-[4px] bg-[var(--color-divider)] rounded-[2px] overflow-hidden">
                    <div
                      data-test="bar-fill"
                      class="h-full rounded-[2px] transition-[width] duration-150"
                      :class="isOver(r) ? 'bg-[var(--color-warning)]' : 'bg-gradient-to-r from-[var(--color-brand-gradient-from)] to-[var(--color-brand-gradient-to)]'"
                      :style="{ width: barPct(r) + '%' }"
                    ></div>
                  </div>
                  <span
                    data-test="role-spent" class="text-[length:var(--app-text-xs-alt)] whitespace-nowrap"
                    :class="isOver(r) ? 'text-[var(--color-warning)] font-semibold' : 'text-[var(--color-text-muted)]'"
                  >
                    <span class="mono" :class="isOver(r) ? '' : 'text-[var(--color-text-primary)] font-semibold'">{{ formatDuration(roleSpent(r.origRole)) }}</span>
                    <template v-if="isOver(r)"> · over by {{ overBy(r) }}</template>
                    <template v-else> logged</template>
                  </span>
                </div>
```

每个 goal 行的 `<input data-test="goal-name">` 与 remove 按钮之间插入：

```vue
                        <span
                          data-test="goal-logged"
                          class="text-[length:var(--app-text-xs-alt)] mono whitespace-nowrap min-w-[46px] text-right"
                          :class="goalLogged(g.origName) > 0 ? 'text-[var(--color-text-muted)]' : 'text-[var(--color-text-disabled)]'"
                        >{{ goalLogged(g.origName) > 0 ? formatDuration(goalLogged(g.origName)) : "0" }}</span>
```

- [ ] **Step 4: 运行确认通过**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts`
Expected: 全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/CommitmentsModal.vue src/__tests__/components/composite/CommitmentsModal.test.ts
git commit -m "feat(ui): header summary, live progress bar, per-goal logged, amber over-commit"
```

---

## Task 6: 删除约束 + 行内确认（依据 logged 主动禁用）

**Files:**
- Modify: `src/components/composite/CommitmentsModal.vue`
- Test: `src/__tests__/components/composite/CommitmentsModal.test.ts`

- [ ] **Step 1: 追加失败测试**

```ts
describe("CommitmentsModal — delete constraints", () => {
  it("disables goal remove when the goal has logged time", () => {
    const w = mountModal();
    expect(w.findAll("[data-test='goal-remove']").every(b => (b.element as HTMLButtonElement).disabled)).toBe(true);
  });
  it("enables remove for a freshly added (0-logged) goal", async () => {
    const w = mountModal();
    await w.find("[data-test='add-goal']").trigger("click");
    const removes = w.findAll("[data-test='goal-remove']");
    expect((removes[removes.length - 1].element as HTMLButtonElement).disabled).toBe(false);
  });
  it("removes a 0-logged goal on click", async () => {
    const w = mountModal();
    await w.find("[data-test='add-goal']").trigger("click");
    const before = w.findAll("[data-test='goal-name']").length;
    const removes = w.findAll("[data-test='goal-remove']");
    await removes[removes.length - 1].trigger("click");
    expect(w.findAll("[data-test='goal-name']").length).toBe(before - 1);
  });
  it("disables role Delete when any goal has logged time", () => {
    const w = mountModal();
    expect((w.find("[data-test='role-delete']").element as HTMLButtonElement).disabled).toBe(true);
  });
  it("role Delete on a 0-logged role shows inline confirm then removes", async () => {
    const w = mountModal({
      commitments: [
        makeCommitment({ role: "Developer", allocation: 40, goals: ["Ship onboarding v2"] }),
        makeCommitment({ role: "Advisor", allocation: 5, goals: ["Office hours"] }),
      ],
      progress: [
        makeCommitmentProgress({ role: "Developer", spent_minutes: 870, allocation_minutes: 2400, goals: [{ name: "Ship onboarding v2", spent_minutes: 870 }] }),
        makeCommitmentProgress({ role: "Advisor", spent_minutes: 0, allocation_minutes: 300, goals: [{ name: "Office hours", spent_minutes: 0 }] }),
      ],
    });
    const advisorDel = w.findAll("[data-test='role-delete']")[1];
    expect((advisorDel.element as HTMLButtonElement).disabled).toBe(false);
    await advisorDel.trigger("click");
    await w.find("[data-test='role-delete-confirm']").trigger("click");
    expect(w.findAll("[data-test='role-name']").length).toBe(1);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts -t "delete constraints"`
Expected: FAIL。

- [ ] **Step 3: 实现**

`<script setup>` 追加：

```ts
const confirmRoleIdx = ref<number | null>(null);
function goalRemovable(g: GoalRow): boolean { return goalLogged(g.origName) === 0; }
function roleDeletable(r: RoleRow): boolean { return r.goals.every(g => goalLogged(g.origName) === 0); }
function removeGoal(ri: number, gi: number) {
  if (!goalRemovable(draft.value[ri].goals[gi])) return;
  draft.value[ri].goals.splice(gi, 1);
}
function requestDeleteRole(ri: number) { if (roleDeletable(draft.value[ri])) confirmRoleIdx.value = ri; }
function confirmDeleteRole() { if (confirmRoleIdx.value !== null) { draft.value.splice(confirmRoleIdx.value, 1); confirmRoleIdx.value = null; } }
function cancelDeleteRole() { confirmRoleIdx.value = null; }
```

替换 goal remove 按钮为：

```vue
                        <button
                          data-test="goal-remove" :disabled="!goalRemovable(g)"
                          :title="goalRemovable(g) ? 'Remove goal' : `${formatDuration(goalLogged(g.origName))} logged — rename instead`"
                          class="text-[length:var(--app-text-base)] cursor-pointer px-[4px] transition-[color] duration-150
                                 text-[var(--color-text-disabled)] hover:text-[var(--color-danger)]
                                 disabled:text-[var(--color-divider)] disabled:cursor-not-allowed disabled:hover:text-[var(--color-divider)]"
                          @click="removeGoal(ri, gi)"
                        >&times;</button>
```

在 role-top 行（`h` 那个 span 之后、容器闭合前）追加 Delete / 行内确认：

```vue
                  <span v-if="confirmRoleIdx === ri" class="inline-flex items-center gap-[10px] text-[length:var(--app-text-xs)]">
                    <span class="text-[var(--color-danger)] whitespace-nowrap">Delete role?</span>
                    <a data-test="role-delete-confirm" class="font-semibold text-[var(--color-danger)] cursor-pointer" @click="confirmDeleteRole">Delete</a>
                    <a class="font-semibold text-[var(--color-text-muted)] cursor-pointer" @click="cancelDeleteRole">Cancel</a>
                  </span>
                  <button
                    v-else
                    data-test="role-delete" :disabled="!roleDeletable(r)"
                    :title="roleDeletable(r) ? 'Delete role' : `Has logged time — can't delete this month`"
                    class="text-[length:var(--app-text-xs-alt)] cursor-pointer px-[5px] py-[4px] transition-[color] duration-150
                           text-[var(--color-text-muted)] hover:text-[var(--color-danger)]
                           disabled:text-[var(--color-text-disabled)] disabled:cursor-not-allowed disabled:hover:text-[var(--color-text-disabled)]"
                    @click="requestDeleteRole(ri)"
                  >Delete</button>
```

- [ ] **Step 4: 运行确认通过**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts`
Expected: 全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/CommitmentsModal.vue src/__tests__/components/composite/CommitmentsModal.test.ts
git commit -m "feat(ui): delete constraints + inline role-delete confirm (logged-gated)"
```

---

## Task 7: 前端校验（role 必填/唯一、goal 全局唯一、清空有 logged 的 goal 拦截、空 goal 丢弃）

**Files:**
- Modify: `src/components/composite/CommitmentsModal.vue`
- Test: `src/__tests__/components/composite/CommitmentsModal.test.ts`

- [ ] **Step 1: 追加失败测试**

```ts
describe("CommitmentsModal — validation", () => {
  it("blocks save + message on empty role name", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("");
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.text()).toContain("Role name is required");
  });
  it("blocks save on duplicate role names", async () => {
    const w = mountModal({
      commitments: [makeCommitment({ role: "Developer", allocation: 40, goals: [] }), makeCommitment({ role: "Developer", allocation: 20, goals: [] })],
      progress: [],
    });
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.text()).toContain("Duplicate role name");
  });
  it("blocks save on duplicate goal names across roles", async () => {
    const w = mountModal({
      commitments: [makeCommitment({ role: "Developer", allocation: 40, goals: ["Shared"] }), makeCommitment({ role: "Advisor", allocation: 5, goals: ["Shared"] })],
      progress: [],
    });
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.text()).toContain("Duplicate goal name");
  });
  it("blocks emptying a goal that has logged time", async () => {
    const w = mountModal();
    await w.findAll("[data-test='goal-name']")[0].setValue("");
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.text()).toContain("can't be empty");
  });
  it("silently drops a blank 0-logged goal row on save", async () => {
    (invoke as any).mockResolvedValue([]);
    const w = mountModal({
      commitments: [makeCommitment({ role: "Developer", allocation: 40, goals: ["Ship onboarding v2"] })],
      progress: [makeCommitmentProgress({ role: "Developer", spent_minutes: 0, allocation_minutes: 2400, goals: [{ name: "Ship onboarding v2", spent_minutes: 0 }] })],
    });
    await w.find("[data-test='add-goal']").trigger("click");
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).toHaveBeenCalledWith("set_commitments", expect.objectContaining({
      commitments: [{ role: "Developer", allocation: 40, goals: ["Ship onboarding v2"] }],
    }));
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts -t "validation"`
Expected: FAIL。

- [ ] **Step 3: 实现**

`<script setup>` 追加（放在 `save` 之前）：

```ts
const showErrors = ref(false);
function dupSet(names: string[]): Set<string> {
  const seen = new Set<string>(), dup = new Set<string>();
  for (const n of names) { const t = n.trim(); if (!t) continue; if (seen.has(t)) dup.add(t); else seen.add(t); }
  return dup;
}
const dupRoles = computed(() => dupSet(draft.value.map(r => r.role)));
const dupGoals = computed(() => dupSet(draft.value.flatMap(r => r.goals.map(g => g.name))));
function roleNameInvalid(r: RoleRow): boolean { return r.role.trim() === "" || dupRoles.value.has(r.role.trim()); }
function goalNameInvalid(g: GoalRow): boolean {
  const t = g.name.trim();
  if (t === "") return goalLogged(g.origName) > 0;
  return dupGoals.value.has(t);
}
function validate(): string | null {
  if (draft.value.length === 0) return "At least one role is required";
  for (const r of draft.value) {
    if (r.role.trim() === "") return "Role name is required";
    if (dupRoles.value.has(r.role.trim())) return "Duplicate role name — each role must be unique";
    for (const g of r.goals) {
      if (g.name.trim() === "" && goalLogged(g.origName) > 0) return "Goal with logged time can't be empty";
    }
  }
  if (dupGoals.value.size > 0) return "Duplicate goal name — each goal must be unique";
  return null;
}
```

`save()` 开头加拦截：

```ts
async function save() {
  const msg = validate();
  if (msg) { showErrors.value = true; error.value = msg; return; }
  saving.value = true;
  error.value = "";
  // …其余不变
```

role-name input 追加 `:class`：

```vue
                    :class="showErrors && roleNameInvalid(r) ? 'border-[var(--color-danger)]' : ''"
```

goal-name input 追加 `:class`：

```vue
                          :class="showErrors && goalNameInvalid(g) ? 'border-[var(--color-danger)]' : ''"
```

- [ ] **Step 4: 运行确认通过**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts`
Expected: 全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/CommitmentsModal.vue src/__tests__/components/composite/CommitmentsModal.test.ts
git commit -m "feat(ui): client validation (role required/unique, goal global-unique, logged-goal empty block)"
```

---

## Task 8: 键盘交互（goal Enter 续行、role Enter→alloc、⌘/Ctrl+Enter 保存）

**Files:**
- Modify: `src/components/composite/CommitmentsModal.vue`
- Test: `src/__tests__/components/composite/CommitmentsModal.test.ts`

- [ ] **Step 1: 追加失败测试**

```ts
describe("CommitmentsModal — keyboard", () => {
  it("Enter in a goal input adds a new goal row below", async () => {
    const w = mountModal();
    const before = w.findAll("[data-test='goal-name']").length;
    await w.findAll("[data-test='goal-name']")[0].trigger("keydown", { key: "Enter" });
    expect(w.findAll("[data-test='goal-name']").length).toBe(before + 1);
  });
  it("Enter on a trailing blank goal does NOT add another", async () => {
    const w = mountModal();
    await w.find("[data-test='add-goal']").trigger("click");
    const count = w.findAll("[data-test='goal-name']").length;
    await w.findAll("[data-test='goal-name']")[count - 1].trigger("keydown", { key: "Enter" });
    expect(w.findAll("[data-test='goal-name']").length).toBe(count);
  });
  it("Cmd/Ctrl+Enter saves", async () => {
    (invoke as any).mockResolvedValue([]);
    const w = mountModal();
    await w.find("[data-test='overlay']").trigger("keydown", { key: "Enter", metaKey: true });
    expect(invoke).toHaveBeenCalledWith("set_commitments", expect.anything());
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts -t "keyboard"`
Expected: FAIL。

- [ ] **Step 3: 实现**

`<script setup>` 追加：

```ts
function onGoalEnter(ri: number, gi: number) {
  const goals = draft.value[ri].goals;
  if (gi === goals.length - 1 && goals[gi].name.trim() === "") return;
  goals.splice(gi + 1, 0, { name: "", origName: null, key: nextKey() });
}
function onModalKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") { e.preventDefault(); save(); }
}
```

overlay 根 `<div data-test="overlay" …>` 追加 `@keydown="onModalKeydown"`（已有 `tabindex="-1"`）。

goal-name input 追加：`@keydown.enter.prevent="onGoalEnter(ri, gi)"`。

role-name input 追加（Enter 聚焦本卡 allocation）：

```vue
                    @keydown.enter.prevent="($event.target as HTMLElement).closest('[data-test=role-card]')?.querySelector('[data-test=alloc]')?.focus()"
```

- [ ] **Step 4: 运行确认通过**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts`
Expected: 全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/CommitmentsModal.vue src/__tests__/components/composite/CommitmentsModal.test.ts
git commit -m "feat(ui): keyboard map (goal Enter, role Enter→alloc, Cmd+Enter save)"
```

---

## Task 9: 关闭与放弃确认（Esc / 点遮罩 / Cancel，有改动才确认）

**Files:**
- Modify: `src/components/composite/CommitmentsModal.vue`
- Test: `src/__tests__/components/composite/CommitmentsModal.test.ts`

- [ ] **Step 1: 追加失败测试**

```ts
describe("CommitmentsModal — close & discard", () => {
  it("Esc closes immediately when there are no changes", async () => {
    const w = mountModal();
    await w.find("[data-test='overlay']").trigger("keydown", { key: "Escape" });
    expect(w.emitted("close")).toBeTruthy();
  });
  it("Esc with changes shows discard confirm instead of closing", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("Changed");
    await w.find("[data-test='overlay']").trigger("keydown", { key: "Escape" });
    expect(w.emitted("close")).toBeFalsy();
    expect(w.find("[data-test='discard-confirm']").exists()).toBe(true);
  });
  it("Discard in the confirm emits close", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("Changed");
    await w.find("[data-test='cancel']").trigger("click");
    await w.find("[data-test='discard-yes']").trigger("click");
    expect(w.emitted("close")).toBeTruthy();
  });
  it("clicking the backdrop behaves like cancel (no changes → close)", async () => {
    const w = mountModal();
    await w.find("[data-test='overlay']").trigger("click");
    expect(w.emitted("close")).toBeTruthy();
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts -t "close & discard"`
Expected: FAIL。

- [ ] **Step 3: 实现**

`<script setup>` 追加：

```ts
const showDiscard = ref(false);
const isDirty = computed(() =>
  JSON.stringify(toCommitments(draft.value)) !==
  JSON.stringify(props.commitments.map(c => ({ role: c.role.trim(), allocation: c.allocation, goals: [...c.goals] })))
);
function requestClose() { if (isDirty.value) { showDiscard.value = true; return; } emit("close"); }
function confirmDiscard() { showDiscard.value = false; emit("close"); }
function keepEditing() { showDiscard.value = false; }
```

把 `cancel()` 删除，`data-test="cancel"` 按钮的 `@click` 改为 `requestClose`。

`onModalKeydown` 改为：

```ts
function onModalKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") { e.preventDefault(); save(); return; }
  if (e.key === "Escape") { e.preventDefault(); requestClose(); }
}
```

overlay 根 div 追加 `@click.self="requestClose"`。

footer 之后（dialog 容器内，容器已有 `relative`）追加放弃浮层：

```vue
        <div v-if="showDiscard" data-test="discard-confirm" class="absolute inset-0 flex items-center justify-center bg-black/10">
          <div class="bg-[var(--color-surface)] border border-[var(--color-border-form)] rounded-[var(--radius-card)] shadow-[var(--shadow-toast)] p-[16px] max-w-[300px]">
            <div class="text-[length:var(--app-text-base)] font-semibold text-[var(--color-text-primary)] mb-[4px]">Discard changes?</div>
            <div class="text-[length:var(--app-text-xs)] text-[var(--color-text-secondary)] mb-[14px]">Your edits to this month's commitments won't be saved.</div>
            <div class="flex justify-end gap-[8px]">
              <button class="text-[length:var(--app-text-xs)] font-semibold text-[var(--color-text-muted)] px-[12px] py-[6px] cursor-pointer" @click="keepEditing">Keep editing</button>
              <button data-test="discard-yes" class="text-[length:var(--app-text-xs)] font-semibold text-[var(--color-danger)] bg-red-50 rounded-[var(--radius-form)] px-[12px] py-[6px] cursor-pointer" @click="confirmDiscard">Discard</button>
            </div>
          </div>
        </div>
```

- [ ] **Step 4: 运行确认通过**

Run: `pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts`
Expected: 全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/CommitmentsModal.vue src/__tests__/components/composite/CommitmentsModal.test.ts
git commit -m "feat(ui): close handling + discard confirm on dirty Esc/backdrop/cancel"
```

---

## Task 10: 接入 CommitmentsPanel（打开 modal + 空状态入口）

**Files:**
- Modify: `src/components/CommitmentsPanel.vue`
- Test: `src/__tests__/components/CommitmentsPanel.test.ts`

- [ ] **Step 1: 改写 Panel 测试**

替换 `src/__tests__/components/CommitmentsPanel.test.ts` 全文为：

```ts
// src/__tests__/components/CommitmentsPanel.test.ts
import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import CommitmentsPanel from "../../components/CommitmentsPanel.vue";
import { makeCommitmentProgress, makeCommitment } from "../mocks/fixtures";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("vuedraggable", () => ({
  default: { name: "draggable", props: ["modelValue","itemKey","handle","group","tag","animation"], emits: ["update:modelValue"],
    render() { const items=(this as any).modelValue||[]; const s=(this as any).$slots; return items.map((element:any,index:number)=> s.item? s.item({element,index}):null); } },
}));

function mountPanel(overrides = {}) {
  return mount(CommitmentsPanel, {
    props: {
      progress: [makeCommitmentProgress({ role: "Developer", spent_minutes: 1230, allocation_minutes: 2400 })],
      commitments: [makeCommitment()],
      rootPath: "/x", selectedYear: 2026, selectedMonth: 6,
      ...overrides,
    },
    global: { stubs: { teleport: true } },
  });
}

describe("CommitmentsPanel", () => {
  it("renders role name and mono spent/allocation", () => {
    const w = mountPanel();
    expect(w.text()).toContain("Developer");
    expect(w.text()).toContain("20h 30m");
    expect(w.text()).toContain("40");
  });
  it("opens the modal on Edit click", async () => {
    const w = mountPanel();
    expect(w.find("[role='dialog']").exists()).toBe(false);
    await w.find("[data-test='edit-btn']").trigger("click");
    expect(w.find("[role='dialog']").exists()).toBe(true);
  });
  it("shows 'Set up commitments' and opens modal when there are no commitments", async () => {
    const w = mountPanel({ progress: [], commitments: [] });
    const setup = w.find("[data-test='setup-btn']");
    expect(setup.exists()).toBe(true);
    await setup.trigger("click");
    expect(w.find("[role='dialog']").exists()).toBe(true);
  });
  it("toggles the goal list for a role", async () => {
    const w = mountPanel();
    const goalRows = () => w.findAll("[data-test='goal-row']");
    const initial = goalRows().length;
    await w.find("[data-test='role-toggle']").trigger("click");
    expect(goalRows().length).not.toBe(initial);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `pnpm vitest run src/__tests__/components/CommitmentsPanel.test.ts`
Expected: FAIL（旧 Panel 无 modal、无 setup-btn）。

- [ ] **Step 3: 改写 CommitmentsPanel.vue**

替换 `src/components/CommitmentsPanel.vue` 的 `<script setup>` 为：

```ts
import { ref } from "vue";
import type { Commitment, CommitmentProgress } from "../types";
import { formatDuration } from "../utils/format";
import CommitmentsModal from "./composite/CommitmentsModal.vue";

const props = defineProps<{
  progress: CommitmentProgress[];
  commitments?: Commitment[];
  rootPath?: string;
  selectedYear: number;
  selectedMonth: number;
}>();

const emit = defineEmits<{ saved: [] }>();

const collapsed = ref<Record<string, boolean>>({});
function toggle(role: string) { collapsed.value[role] = !collapsed.value[role]; }
function pct(spent: number, alloc: number): string {
  if (alloc === 0) return "0%";
  return Math.min(100, Math.round((spent / alloc) * 100)) + "%";
}
const hasCommitments = () => !!props.commitments && props.commitments.length > 0;

const modalOpen = ref(false);
function openEditor() { modalOpen.value = true; }
function closeEditor() { modalOpen.value = false; }
function onSaved() { modalOpen.value = false; emit("saved"); }
```

模板改动：
1. 外层根改为始终渲染：`<div data-test="commitments-panel">`（移除依赖 `isEditing` 的 `v-if`）。
2. header 的 Edit 按钮：

```vue
      <button
        v-if="commitments && commitments.length > 0"
        class="text-[length:var(--app-text-xs)] text-[var(--color-brand-link)] font-medium cursor-pointer"
        data-test="edit-btn"
        @click="openEditor"
      >Edit</button>
```

3. 把原来的 `<template v-if="!isEditing"> … progress 列表 … </template>` 改为直接渲染（去掉该 `<template v-if>` 包裹，保留内部 progress 列表 DOM 不变）。
4. progress 列表之后追加空状态入口：

```vue
      <button
        v-if="!hasCommitments()"
        data-test="setup-btn"
        class="mt-[4px] text-[length:var(--app-text-xs)] text-[var(--color-brand-link)] font-medium cursor-pointer hover:underline"
        @click="openEditor"
      >+ Set up commitments</button>
```

5. 删除原 `<CommitmentsEditor v-else … />`，改为末尾挂 modal：

```vue
    <CommitmentsModal
      :open="modalOpen"
      :commitments="commitments || []"
      :progress="progress"
      :root-path="rootPath || ''"
      :selected-year="selectedYear"
      :selected-month="selectedMonth"
      @saved="onSaved"
      @close="closeEditor"
    />
```

（删除 `isEditing`、`enterEdit`、`cancelEdit` 等旧编辑态变量与 `CommitmentsEditor` 的 import。）

- [ ] **Step 4: 运行确认通过**

Run: `pnpm vitest run src/__tests__/components/CommitmentsPanel.test.ts`
Expected: 全部 PASS。再跑 `pnpm vitest run` 确认无其它回归。

- [ ] **Step 5: Commit**

```bash
git add src/components/CommitmentsPanel.vue src/__tests__/components/CommitmentsPanel.test.ts
git commit -m "feat(ui): open CommitmentsModal from panel + empty-state setup entry"
```

---

## Task 11: 删除被取代的旧内联编辑器

**Files:**
- Delete: `src/components/composite/CommitmentsEditor.vue`
- Delete: `src/__tests__/components/composite/CommitmentsEditor.test.ts`

- [ ] **Step 1: 确认无残留引用**

Run: `grep -rn "CommitmentsEditor" src/`
Expected: 仅匹配将被删的两个文件自身。否则先清除引用。

- [ ] **Step 2: 删除**

```bash
git rm src/components/composite/CommitmentsEditor.vue src/__tests__/components/composite/CommitmentsEditor.test.ts
```

- [ ] **Step 3: 全量校验**

Run: `pnpm vue-tsc --noEmit && pnpm vitest run`
Expected: 类型无错、测试全绿。

- [ ] **Step 4: Commit**

```bash
git commit -m "chore: remove inline CommitmentsEditor superseded by CommitmentsModal"
```

---

## Task 12: 文档同步

**Files:**
- Modify: `SPEC.md`
- Modify: `src-tauri/CLAUDE.md`

- [ ] **Step 1: 修正 SPEC.md**

确保命令清单列出 `set_commitments`，并把 L40 附近「Commitments 通过直接编辑 `_monthly.md` 文件写入…不提供 `set_commitments` 命令」改为：

```
Commitments 通过 `set_commitments(root_path, year, month, commitments)` 写入（校验 + goal 改名批量更新 entry + 原子写 `_monthly.md`；文件监听随后重新读取）。校验：role 名非空且唯一、allocation > 0、goal 名非空且全局唯一、删除有 entry 引用的 goal 拒绝。
```

- [ ] **Step 2: 修正 src-tauri/CLAUDE.md**

把「关键约定」里 `Commitments 不在 Rust 端写入——用户直接编辑 _monthly.md，由 notify watcher 重新读取` 改为：

```
- Commitments 经 `set_commitments` 命令写入（校验 + goal 改名批量更新 entry + 原子写 _monthly.md）；外部直接编辑 _monthly.md 仍由 notify watcher 重新读取
```

- [ ] **Step 3: 一致性检查 + Commit**

Run: `grep -n "set_commitments" SPEC.md src-tauri/CLAUDE.md`
Expected: 两文件均出现且描述与实现一致。

```bash
git add SPEC.md src-tauri/CLAUDE.md
git commit -m "docs: sync set_commitments command + validation rules"
```

---

## 完成校验（全部 Task 后）

- [ ] Run: `pnpm vue-tsc --noEmit`（前端类型）
- [ ] Run: `pnpm vitest run`（前端全绿）
- [ ] Run: `cd src-tauri && cargo check && cargo test`（后端全绿）
- [ ] 手动（`pnpm tauri dev`）：打开 modal → 调 allocation 看进度条实时变化与 amber 超额 → 拖拽重排 role/goal → 删除约束（有 logged 的 × / Delete 禁用）→ 改名保存后 entry 归属更新 → 空状态「Set up commitments」→ 改动后 Esc 弹放弃确认。
