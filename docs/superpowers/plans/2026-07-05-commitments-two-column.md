# CommitmentsModal 两栏布局重构 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 CommitmentsModal 从单列 RoleCard 列表重构为两栏 master-detail 布局（与 DimensionEditorModal 结构一致），解决长列表滚动和心理安全问题。

**Architecture:** 左侧 ~210px 面板显示角色列表（拖拽排序 + 点击切换），右侧 flex-1 面板编辑选中角色。RoleCard.vue 和 GoalRow.vue 的模板逻辑内联到 CommitmentsModal。数据流不变——`draft` 集中管理，`selectedIndex` 驱动右侧面板。

**Tech Stack:** Vue 3 SFC + `<script setup>`，vue-draggable-plus，不变更后端/IPC。

## Global Constraints

- 交互原则遵循 `docs/interaction-principles.md`（脏检确认、Esc/Cmd+Enter 键盘、IME 安全）
- 设计 token 约束：间距 `p-md`/`px-2xl` 等，字号 `text-body`/`text-secondary`/`text-micro`
- 命名约定：`data-test` 属性命名遵循已有模式
- Props/emits 接口不变（CommitmentsPanel 调用方无需改动）
- 整体保存（不引入 per-role 自动保存）

## File Structure

| 文件 | 操作 | 职责 |
|------|------|------|
| `src/components/composite/CommitmentsModal.vue` | 重写 | 两栏 master-detail，内联 RoleCard + GoalRow 逻辑 |
| `src/components/composite/RoleCard.vue` | 删除 | 不再需要 |
| `src/components/composite/GoalRow.vue` | 删除 | 不再需要 |
| `src/__tests__/components/composite/CommitmentsModal.test.ts` | 更新 | 适配新 DOM 结构 + 新增左右面板交互测试 |
| `src/__tests__/components/composite/CommitmentsModal.dnd.test.ts` | 更新 | 适配新组件结构（无 RoleCard 子组件） |

---

### Task 1: 重写 CommitmentsModal.vue

**Files:**
- Create: `src/components/composite/CommitmentsModal.vue`（覆盖）

**Interfaces:**
- Consumes: `RoleRowModel`, `GoalRowModel`, `Commitment`, `CommitmentProgress` from `types.ts`；`formatDurationCompact` from `format.ts`；`goalLoggedMinutes` from `commitments.ts`；`VueDraggable` from `vue-draggable-plus`
- Produces: Props/emits 不变——`open`, `commitments`, `progress`, `rootPath`, `selectedYear`, `selectedMonth` → `saved`, `close`

- [ ] **Step 1: 写入新 CommitmentsModal.vue**

```vue
<script setup lang="ts">
import { ref, computed, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { VueDraggable } from "vue-draggable-plus";
import type { Commitment, CommitmentProgress, RoleRowModel, GoalRowModel } from "../../types";
import { formatDurationCompact } from "../../utils/format";
import { logError } from "../../utils/errorLog";
import { goalLoggedMinutes } from "../../utils/commitments";

const props = defineProps<{
  open: boolean;
  commitments: Commitment[];
  progress: CommitmentProgress[];
  rootPath: string;
  selectedYear: number;
  selectedMonth: number;
}>();

const emit = defineEmits<{ saved: [Commitment[]]; close: [] }>();

const NEW_ROLE_ALLOC = 5;
let _key = 0;
const nextKey = () => ++_key;

const draft = ref<RoleRowModel[]>([]);
const selectedIndex = ref(0);
const error = ref("");
const saving = ref(false);
const showErrors = ref(false);
const showDiscard = ref(false);
const confirmingDelete = ref(false);
const overlayRef = ref<HTMLElement>();
const roleNameInputRef = ref<HTMLInputElement>();

function buildDraft() {
  draft.value = props.commitments.map((c): RoleRowModel => ({
    role: c.role, allocation: c.allocation, origRole: c.role, key: nextKey(),
    goals: c.goals.map((g): GoalRowModel => ({ name: g, origName: g, key: nextKey() })),
  }));
  selectedIndex.value = 0;
  error.value = "";
  showErrors.value = false;
  showDiscard.value = false;
}
watch(() => props.open, (o) => {
  if (!o) return;
  buildDraft();
  nextTick(() => overlayRef.value?.focus());
}, { immediate: true });

function toCommitments(rows: RoleRowModel[]): Commitment[] {
  return rows.map(r => ({
    role: r.role.trim(),
    allocation: r.allocation,
    goals: r.goals.map(g => g.name.trim()).filter(n => n !== ""),
  }));
}

const monthLabel = computed(() =>
  new Date(props.selectedYear, props.selectedMonth - 1, 1)
    .toLocaleDateString("en-US", { month: "long", year: "numeric" })
);

const committedHours = computed(() => draft.value.reduce((s, r) => s + (r.allocation || 0), 0));
const loggedTotal = computed(() =>
  props.progress.reduce((s, p) => s + p.goal_spent_minutes + p.general_spent_minutes, 0)
);

const selectedRole = computed(() => draft.value[selectedIndex.value] ?? null);

function dupSet(names: string[]): Set<string> {
  const seen = new Set<string>(), dup = new Set<string>();
  for (const n of names) { const t = n.trim(); if (!t) continue; if (seen.has(t)) dup.add(t); else seen.add(t); }
  return dup;
}
const dupRoles = computed(() => dupSet(draft.value.map(r => r.role)));
const dupGoals = computed(() => dupSet(draft.value.flatMap(r => r.goals.map(g => g.name))));

function validate(): string | null {
  if (draft.value.length === 0) return "At least one role is required";
  for (const r of draft.value) {
    if (r.role.trim() === "") return "Role name is required";
    if (dupRoles.value.has(r.role.trim())) return "Duplicate role name — each role must be unique";
    for (const g of r.goals) {
      if (g.name.trim() === "" && goalLoggedMinutes(props.progress, g.origName) > 0) return "Goal with logged time can't be empty";
    }
  }
  if (dupGoals.value.size > 0) return "Duplicate goal name — each goal must be unique";
  return null;
}

const isDirty = computed(() =>
  JSON.stringify(toCommitments(draft.value)) !==
  JSON.stringify(props.commitments.map(c => ({
    role: c.role.trim(),
    allocation: c.allocation,
    goals: c.goals.map(g => g.trim()).filter(n => n !== ""),
  })))
);

function requestClose() { if (isDirty.value) { showDiscard.value = true; return; } emit("close"); }
function confirmDiscard() { showDiscard.value = false; emit("close"); }
function keepEditing() { showDiscard.value = false; }

async function save() {
  const msg = validate();
  if (msg) { showErrors.value = true; error.value = msg; return; }
  saving.value = true;
  error.value = "";
  const saved = toCommitments(draft.value);
  try {
    await invoke("set_commitments", {
      rootPath: props.rootPath,
      year: props.selectedYear,
      month: props.selectedMonth,
      commitments: saved,
    });
    emit("saved", saved);
    emit("close");
  } catch (e) {
    logError("CommitmentsModal.save", e);
    error.value = typeof e === "string" ? e : String(e);
  } finally {
    saving.value = false;
  }
}

function onModalKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") { e.preventDefault(); save(); return; }
  if (e.key === "Escape") { e.preventDefault(); requestClose(); }
}

// ── Left panel ──────────────────────────────────────────────────

function addRole() {
  draft.value.push({
    role: "", allocation: NEW_ROLE_ALLOC, origRole: null, key: nextKey(),
    goals: [{ name: "", origName: null, key: nextKey() }],
  });
  selectedIndex.value = draft.value.length - 1;
  nextTick(() => roleNameInputRef.value?.focus());
}

function selectRole(index: number) { selectedIndex.value = index; }

function navigateRole(delta: 1 | -1) {
  if (draft.value.length <= 1) return;
  selectedIndex.value = (selectedIndex.value + delta + draft.value.length) % draft.value.length;
}

function removeRole(r: RoleRowModel) {
  const i = draft.value.findIndex(x => x.key === r.key);
  if (i < 0) return;
  draft.value.splice(i, 1);
  if (selectedIndex.value >= draft.value.length) {
    selectedIndex.value = Math.max(0, draft.value.length - 1);
  }
  confirmingDelete.value = false;
}

function roleSpentMinutes(r: RoleRowModel): number {
  if (!r.origRole) return 0;
  const p = props.progress.find(x => x.role === r.origRole);
  return (p?.goal_spent_minutes ?? 0) + (p?.general_spent_minutes ?? 0);
}

// ── Right panel (selected role) ──────────────────────────────────

const STEP = 5;
const MIN_ALLOC = 5;

function stepAlloc(delta: number) {
  if (!selectedRole.value) return;
  selectedRole.value.allocation = Math.max(MIN_ALLOC, (selectedRole.value.allocation || 0) + delta);
}

function onAllocInput(e: Event) {
  const el = e.target as HTMLInputElement;
  const v = Math.floor(Number(el.value));
  const next = Number.isFinite(v) && v >= 1 ? v : 1;
  if (selectedRole.value) selectedRole.value.allocation = next;
  if (el.value !== String(next)) el.value = String(next);
}

function addGoal() {
  if (!selectedRole.value) return;
  selectedRole.value.goals.push({ name: "", origName: null, key: nextKey() });
}

function onGoalEnter(g: GoalRowModel) {
  if (!selectedRole.value) return;
  const goals = selectedRole.value.goals;
  const gi = goals.findIndex(x => x.key === g.key);
  if (gi === -1) return;
  if (gi === goals.length - 1 && goals[gi].name.trim() === "") return;
  goals.splice(gi + 1, 0, { name: "", origName: null, key: nextKey() });
}

function goalLogged(origName: string | null): number {
  return goalLoggedMinutes(props.progress, origName);
}

function removeGoal(g: GoalRowModel) {
  if (goalLogged(g.origName) > 0) return;
  if (!selectedRole.value) return;
  const i = selectedRole.value.goals.findIndex(x => x.key === g.key);
  if (i >= 0) selectedRole.value.goals.splice(i, 1);
}

const roleNameInvalid = computed(() => {
  if (!selectedRole.value) return false;
  const t = selectedRole.value.role.trim();
  return t === "" || dupRoles.value.has(t);
});

function goalNameInvalid(g: GoalRowModel): boolean {
  const t = g.name.trim();
  if (t === "") return goalLogged(g.origName) > 0;
  return dupGoals.value.has(t);
}

const roleSpent = computed(() => {
  if (!selectedRole.value?.origRole) return 0;
  const p = props.progress.find(x => x.role === selectedRole.value!.origRole);
  return (p?.goal_spent_minutes ?? 0) + (p?.general_spent_minutes ?? 0);
});

const allocMinutes = computed(() => (selectedRole.value?.allocation ?? 0) * 60);
const isOver = computed(() => roleSpent.value > allocMinutes.value);
const barPct = computed(() => {
  const a = allocMinutes.value;
  if (a <= 0) return roleSpent.value > 0 ? 100 : 0;
  return Math.min(100, Math.round((roleSpent.value / a) * 100));
});
const overBy = computed(() => formatDurationCompact(roleSpent.value - allocMinutes.value));

const roleDeletable = computed(() => {
  if (!selectedRole.value) return false;
  return selectedRole.value.goals.every(g => goalLogged(g.origName) === 0);
});

function requestDelete() { if (roleDeletable.value) confirmingDelete.value = true; }
function cancelDelete() { confirmingDelete.value = false; }

const leftRoleBar = (r: RoleRowModel): number => {
  const a = r.allocation * 60;
  const s = roleSpentMinutes(r);
  if (a <= 0) return s > 0 ? 100 : 0;
  return Math.min(100, Math.round((s / a) * 100));
};
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      ref="overlayRef"
      data-test="overlay" tabindex="-1"
      @keydown="onModalKeydown"
      @click.self="requestClose"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
    >
      <div
        role="dialog" aria-modal="true"
        class="relative w-[660px] max-w-[92vw] max-h-[88vh] flex flex-col bg-[var(--color-surface)]
               border border-[var(--color-border-form)] rounded-[var(--radius-lg)]
               shadow-[var(--shadow-popover)] overflow-hidden"
      >
        <!-- Header -->
        <div class="flex justify-between items-start px-2xl pt-xl pb-lg border-b border-[var(--color-divider)]">
          <div>
            <div class="text-title font-bold text-[var(--color-text-primary)] tracking-[-0.3px]">Edit Commitments</div>
            <div class="text-secondary text-[var(--color-text-muted)] mt-2xs">{{ monthLabel }}</div>
          </div>
          <div class="text-right text-secondary text-[var(--color-text-muted)]">
            <div>Committed <span data-test="committed" class="mono font-bold text-[var(--color-brand-link)]">{{ committedHours }}h</span></div>
            <div>Logged <span data-test="logged" class="mono font-semibold text-[var(--color-text-primary)]">{{ formatDurationCompact(loggedTotal) }}</span></div>
          </div>
        </div>

        <!-- Body: two-column -->
        <div class="flex-1 flex min-h-0">
          <!-- Left panel: role list -->
          <div class="w-[210px] flex-shrink-0 border-r border-[var(--color-divider)] bg-[var(--color-surface-muted)] p-md flex flex-col">
            <VueDraggable
              v-model="draft"
              handle=".drag-grip-role"
              :animation="150"
              class="flex-1 space-y-2xs"
            >
              <div
                v-for="(r, index) in draft" :key="r.key"
                class="flex items-center gap-sm px-sm py-xs rounded-[var(--radius-form-lg)] cursor-pointer border"
                :class="index === selectedIndex
                  ? 'bg-[var(--color-brand-soft-bg)] border-[var(--color-brand-link)]'
                  : 'border-transparent'"
                :data-test="index === selectedIndex ? 'role-row-selected' : 'role-row'"
                tabindex="0"
                @click="selectRole(index)"
                @keydown.up.prevent="navigateRole(-1)"
                @keydown.down.prevent="navigateRole(1)"
              >
                <span data-test="drag-grip-role" class="drag-grip-role cursor-grab text-[var(--color-text-disabled)] select-none px-2xs">⠿</span>
                <div
                  class="w-[3px] h-[16px] rounded-[1px] flex-shrink-0"
                  :style="{
                    background: leftRoleBar(r) >= 100
                      ? 'var(--color-warning)'
                      : `linear-gradient(to right, var(--color-brand-gradient-from), var(--color-brand-gradient-to))`,
                  }"
                ></div>
                <span class="text-body text-[var(--color-text-primary)] flex-1 truncate">{{ r.role || 'New role' }}</span>
                <span class="text-micro text-[var(--color-text-muted)]">
                  {{ formatDurationCompact(roleSpentMinutes(r)) }} / {{ r.allocation }}h
                </span>
              </div>
            </VueDraggable>

            <button
              data-test="add-role"
              class="text-secondary font-semibold text-[var(--color-brand-link)] mt-sm text-left cursor-pointer"
              @click="addRole"
            >+ Add Role</button>
          </div>

          <!-- Right panel -->
          <template v-if="selectedRole">
            <div class="flex-1 flex flex-col min-h-0">
              <div class="flex-1 overflow-y-auto px-2xl py-xl">
                <!-- Role name + delete -->
                <div class="flex items-center gap-sm">
                  <input
                    ref="roleNameInputRef"
                    v-model="selectedRole.role"
                    data-test="role-name" placeholder="Role"
                    class="flex-1 text-title font-semibold text-[var(--color-text-primary)] bg-transparent
                           border-0 border-b-2 border-[var(--color-border-form)] rounded-none
                           px-0 pb-xs outline-none focus:border-[var(--color-brand-solid)]"
                    :class="showErrors && roleNameInvalid ? 'border-[var(--color-danger)]' : ''"
                  />
                  <span v-if="confirmingDelete" class="inline-flex items-center gap-sm text-secondary">
                    <span class="text-[var(--color-danger)] whitespace-nowrap">Delete role?</span>
                    <button type="button" data-test="role-delete-confirm" class="font-semibold text-[var(--color-danger)] cursor-pointer" @click="removeRole(selectedRole)">Delete</button>
                    <button type="button" data-test="role-delete-cancel" class="font-semibold text-[var(--color-text-muted)] cursor-pointer" @click="cancelDelete">Cancel</button>
                  </span>
                  <button
                    v-else
                    data-test="role-delete" :disabled="!roleDeletable"
                    :title="roleDeletable ? 'Delete role' : `Has logged time — can't delete this month`"
                    class="text-secondary cursor-pointer px-xs py-xs transition-[color] duration-[var(--motion-fast)]
                           text-[var(--color-text-muted)] hover:text-[var(--color-danger)]
                           disabled:text-[var(--color-text-disabled)] disabled:cursor-not-allowed disabled:hover:text-[var(--color-text-disabled)]"
                    @click="requestDelete"
                  >Delete</button>
                </div>

                <!-- Allocation stepper -->
                <div class="flex items-center gap-sm mt-md">
                  <span class="inline-flex items-center gap-xs">
                    <button
                      data-test="alloc-dec" :disabled="selectedRole.allocation <= MIN_ALLOC"
                      class="w-[24px] h-[26px] flex items-center justify-center border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-body text-[var(--color-text-secondary)] bg-[var(--color-surface)]
                             hover:border-[var(--color-brand-solid)] hover:text-[var(--color-brand-link)]
                             disabled:text-[var(--color-text-disabled)] disabled:cursor-not-allowed disabled:hover:border-[var(--color-border-form)]
                             cursor-pointer transition-[border-color,color] duration-[var(--motion-fast)]"
                      @click="stepAlloc(-STEP)"
                    >&minus;</button>
                    <input
                      :value="selectedRole.allocation" type="number" data-test="alloc"
                      class="w-[52px] text-center px-xs py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-body font-semibold text-[var(--color-text-primary)] mono
                             bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                      @input="onAllocInput($event)"
                      @keydown.up.prevent="stepAlloc(STEP)"
                      @keydown.down.prevent="stepAlloc(-STEP)"
                    />
                    <button
                      data-test="alloc-inc"
                      class="w-[24px] h-[26px] flex items-center justify-center border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-body text-[var(--color-text-secondary)] bg-[var(--color-surface)]
                             hover:border-[var(--color-brand-solid)] hover:text-[var(--color-brand-link)]
                             cursor-pointer transition-[border-color,color] duration-[var(--motion-fast)]"
                      @click="stepAlloc(STEP)"
                    >+</button>
                    <span class="text-secondary text-[var(--color-text-muted)]">h</span>
                  </span>
                </div>

                <!-- Progress bar -->
                <div class="flex items-center gap-sm mt-sm">
                  <div class="flex-1 h-[4px] bg-[var(--color-divider)] rounded-full overflow-hidden">
                    <div
                      data-test="bar-fill"
                      class="h-full rounded-full transition-[width] duration-[var(--motion-fast)]"
                      :class="isOver ? 'bg-[var(--color-warning)]' : 'bg-gradient-to-r from-[var(--color-brand-gradient-from)] to-[var(--color-brand-gradient-to)]'"
                      :style="{ width: barPct + '%' }"
                    ></div>
                  </div>
                  <span
                    data-test="role-spent" class="text-secondary whitespace-nowrap"
                    :class="isOver ? 'text-[var(--color-warning)] font-semibold' : 'text-[var(--color-text-muted)]'"
                  >
                    <span class="mono" :class="isOver ? '' : 'text-[var(--color-text-primary)] font-semibold'">{{ formatDurationCompact(roleSpent) }}</span>
                    <template v-if="isOver"> · over by {{ overBy }}</template>
                    <template v-else> logged</template>
                  </span>
                </div>

                <div class="border-t border-[var(--color-divider)] my-lg"></div>

                <!-- Goal list -->
                <VueDraggable v-model="selectedRole.goals" handle=".drag-grip-goal" :animation="150" class="flex flex-col gap-sm">
                  <div v-for="g in selectedRole.goals" :key="g.key" class="flex items-center gap-sm">
                    <span data-test="drag-grip-goal" class="drag-grip-goal cursor-grab text-[var(--color-text-disabled)] select-none px-2xs">⠿</span>
                    <input
                      v-model="g.name" data-test="goal-name" placeholder="Goal name"
                      @keydown.enter.exact.prevent="onGoalEnter(g)"
                      class="flex-1 px-sm py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-secondary text-[var(--color-text-secondary)]
                             bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                      :class="showErrors && goalNameInvalid(g) ? 'border-[var(--color-danger)]' : ''"
                    />
                    <span
                      data-test="goal-logged"
                      class="text-secondary mono whitespace-nowrap min-w-[46px] text-right"
                      :class="goalLogged(g.origName) > 0 ? 'text-[var(--color-text-muted)]' : 'text-[var(--color-text-disabled)]'"
                    >{{ goalLogged(g.origName) > 0 ? formatDurationCompact(goalLogged(g.origName)) : "0" }}</span>
                    <button
                      data-test="goal-remove" :disabled="goalLogged(g.origName) > 0"
                      :title="goalLogged(g.origName) > 0 ? `${formatDurationCompact(goalLogged(g.origName))} logged — rename instead` : 'Remove goal'"
                      class="text-body cursor-pointer px-xs transition-[color] duration-[var(--motion-fast)]
                             text-[var(--color-text-disabled)] hover:text-[var(--color-danger)]
                             disabled:text-[var(--color-divider)] disabled:cursor-not-allowed disabled:hover:text-[var(--color-divider)]"
                      @click="removeGoal(g)"
                    >&times;</button>
                  </div>
                </VueDraggable>
                <button
                  data-test="add-goal"
                  class="self-start mt-sm text-secondary font-medium text-[var(--color-brand-link)] cursor-pointer hover:underline"
                  @click="addGoal"
                >+ Add Goal</button>
              </div>
            </div>
          </template>
          <template v-else>
            <div class="flex-1 flex items-center justify-center px-2xl">
              <p class="text-secondary text-[var(--color-text-muted)]">No roles yet. Add a role to get started.</p>
            </div>
          </template>
        </div>

        <!-- Footer -->
        <div v-if="error" class="px-2xl pb-sm text-secondary text-[var(--color-danger)]" data-test="save-error">{{ error }}</div>
        <div class="flex justify-end gap-sm px-2xl py-lg border-t border-[var(--color-divider)]">
          <button
            data-test="cancel"
            class="text-secondary font-semibold text-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer"
            @click="requestClose"
          >Cancel</button>
          <button
            data-test="save" :disabled="saving"
            class="text-secondary font-semibold text-white bg-[var(--color-brand-solid)] hover:bg-[var(--color-brand-link)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer disabled:opacity-50"
            @click="save"
          >{{ saving ? "Saving…" : "Save" }}</button>
        </div>

        <!-- Discard confirmation overlay -->
        <div v-if="showDiscard" data-test="discard-confirm" class="absolute inset-0 flex items-center justify-center bg-black/10">
          <div class="bg-[var(--color-surface)] border border-[var(--color-border-form)] rounded-[var(--radius-card)] shadow-[var(--shadow-toast)] p-lg max-w-[300px]">
            <div class="text-body font-semibold text-[var(--color-text-primary)] mb-xs">Discard changes?</div>
            <div class="text-secondary text-[var(--color-text-secondary)] mb-md">Your edits to this month's commitments won't be saved.</div>
            <div class="flex justify-end gap-sm">
              <button class="text-secondary font-semibold text-[var(--color-text-muted)] px-md py-sm cursor-pointer" @click="keepEditing">Keep editing</button>
              <button data-test="discard-yes" class="text-secondary font-semibold text-[var(--color-danger)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer" @click="confirmDiscard">Discard</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
```

- [ ] **Step 2: 提交**

```bash
git add src/components/composite/CommitmentsModal.vue
git commit -m "refactor: CommitmentsModal two-column master-detail layout"
```

---

### Task 2: 更新 CommitmentsModal.test.ts

**Files:**
- Modify: `src/__tests__/components/composite/CommitmentsModal.test.ts`

**Interfaces:**
- Consumes: 新 CommitmentsModal 的 `data-test` 属性（`role-row`, `role-row-selected`, `role-name`, `goal-name` 等）
- Produces: 与旧测试相同的覆盖范围 + 新增左右面板交互测试

- [ ] **Step 1: 重写测试文件**

关键变更：
- 单列 → 两栏后 `role-name` 只渲染选中角色（首个自动选中）
- `goal-name` 只渲染选中角色的目标
- 多角色测试需遍历左侧面板 `role-row` 切换选中
- `add-role` 后 `role-row` 计数验证替代 `role-name` 计数
- 新增：左侧面板 ↑/↓ 导航、空状态、`save-error` data-test

```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { nextTick } from "vue";
import { mount } from "@vue/test-utils";
import { invoke } from "@tauri-apps/api/core";
import CommitmentsModal from "../../../components/composite/CommitmentsModal.vue";
import { makeCommitment, makeCommitmentProgress } from "../../mocks/fixtures";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

vi.mock("vue-draggable-plus", () => ({
  VueDraggable: {
    name: "VueDraggable",
    props: ["modelValue", "handle", "animation", "group", "tag"],
    emits: ["update:modelValue"],
    render() { return (this as any).$slots.default?.(); },
  },
}));

const baseProps = () => ({
  open: true,
  commitments: [
    makeCommitment({ role: "Developer", allocation: 40, goals: ["Ship onboarding v2", "Review auth PR"] }),
  ],
  progress: [
    makeCommitmentProgress({
      role: "Developer", allocation_minutes: 2400, goal_spent_minutes: 870, general_spent_minutes: 0,
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

// ── Base rendering ──────────────────────────────────────────────

describe("CommitmentsModal — base", () => {
  it("renders role and goal values from props", () => {
    const w = mountModal();
    expect((w.find("[data-test='role-name']").element as HTMLInputElement).value).toBe("Developer");
    const goals = w.findAll("[data-test='goal-name']").map(g => (g.element as HTMLInputElement).value);
    expect(goals).toContain("Ship onboarding v2");
    expect(goals).toContain("Review auth PR");
  });

  it("renders a drag handle per role (left panel) and per goal (right panel)", () => {
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
    expect(w.findAll("[data-test='role-row']").length).toBe(2);
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

  it("editing the draft does not mutate the commitments prop (working-copy isolation)", async () => {
    const goals = ["Ship onboarding v2", "Review auth PR"];
    const commitments = [makeCommitment({ role: "Developer", allocation: 40, goals })];
    const w = mountModal({ commitments });

    await w.find("[data-test='role-name']").setValue("Architect");
    await w.findAll("[data-test='goal-name']")[0].setValue("Ship onboarding v3");
    await w.find("[data-test='add-goal']").trigger("click");

    expect(commitments[0].role).toBe("Developer");
    expect(commitments[0].goals).toBe(goals);
    expect(goals).toEqual(["Ship onboarding v2", "Review auth PR"]);
  });
});

// ── Right panel: empty state ───────────────────────────────────

describe("CommitmentsModal — empty state", () => {
  it("shows empty message in right panel when no roles", async () => {
    const w = mountModal({
      commitments: [makeCommitment({ role: "Solo", allocation: 40, goals: [] })],
      progress: [makeCommitmentProgress({ role: "Solo", allocation_minutes: 2400, goal_spent_minutes: 0, general_spent_minutes: 0, goals: [] })],
    });
    await w.find("[data-test='role-delete']").trigger("click");
    await w.find("[data-test='role-delete-confirm']").trigger("click");
    expect(w.text()).toContain("No roles yet");
  });
});

// ── Allocation stepper ──────────────────────────────────────────

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
      progress: [makeCommitmentProgress({ role: "Developer", allocation_minutes: 300, goal_spent_minutes: 0, general_spent_minutes: 0, goals: [] })],
    });
    const dec = w.find("[data-test='alloc-dec']");
    expect((dec.element as HTMLButtonElement).disabled).toBe(true);
    await dec.trigger("click");
    expect((w.find("[data-test='alloc']").element as HTMLInputElement).value).toBe("5");
  });

  it("clamps a cleared field to 1 and re-syncs the input (no desync)", async () => {
    const w = mountModal({
      commitments: [makeCommitment({ role: "Developer", allocation: 1, goals: [] })],
      progress: [makeCommitmentProgress({ role: "Developer", allocation_minutes: 60, goal_spent_minutes: 0, general_spent_minutes: 0, goals: [] })],
    });
    const inp = w.find("[data-test='alloc']");
    await inp.setValue("");
    expect((inp.element as HTMLInputElement).value).toBe("1");
  });
});

// ── Summary, progress & over-commit ─────────────────────────────

describe("CommitmentsModal — summary, progress & over-commit", () => {
  it("header shows live committed total and logged total", async () => {
    const w = mountModal();
    expect(w.find("[data-test='committed']").text()).toContain("40h");
    expect(w.find("[data-test='logged']").text()).toContain("14.5h");
    await w.find("[data-test='alloc-inc']").trigger("click");
    expect(w.find("[data-test='committed']").text()).toContain("45h");
  });
  it("shows selected role logged and per-goal logged", () => {
    const w = mountModal();
    expect(w.find("[data-test='role-spent']").text()).toContain("14.5h");
    const logged = w.findAll("[data-test='goal-logged']").map(n => n.text());
    expect(logged.some(t => t.includes("14.4h"))).toBe(true);
    expect(logged.some(t => t.includes("0.1h"))).toBe(true);
  });
  it("bar fills proportionally to spent/allocation", () => {
    const w = mountModal();
    expect((w.find("[data-test='bar-fill']").element as HTMLElement).style.width).toBe("36%");
  });
  it("keeps per-goal logged matched by original name after a rename", async () => {
    const w = mountModal();
    await w.findAll("[data-test='goal-name']")[0].setValue("Renamed goal");
    const logged = w.findAll("[data-test='goal-logged']").map(n => n.text());
    expect(logged.some(t => t.includes("14.4h"))).toBe(true);
  });
  it("turns amber + 'over by' when allocation drops below logged", async () => {
    const w = mountModal();
    const dec = w.find("[data-test='alloc-dec']");
    for (let i = 0; i < 6; i++) await dec.trigger("click");
    expect((w.find("[data-test='bar-fill']").element as HTMLElement).style.width).toBe("100%");
    expect(w.find("[data-test='role-spent']").text()).toContain("over by");
    expect(w.find("[data-test='bar-fill']").classes().join(" ")).toContain("color-warning");
  });
});

// ── Delete constraints ──────────────────────────────────────────

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
        makeCommitmentProgress({ role: "Developer", goal_spent_minutes: 870, general_spent_minutes: 0, allocation_minutes: 2400, goals: [{ name: "Ship onboarding v2", spent_minutes: 870 }] }),
        makeCommitmentProgress({ role: "Advisor", goal_spent_minutes: 0, general_spent_minutes: 0, allocation_minutes: 300, goals: [{ name: "Office hours", spent_minutes: 0 }] }),
      ],
    });
    // Select Advisor (index 1) in left panel
    await w.findAll("[data-test='role-row']")[1].trigger("click");
    await w.find("[data-test='role-delete']").trigger("click");
    await w.find("[data-test='role-delete-confirm']").trigger("click");
    expect(w.findAll("[data-test='role-row']").length).toBe(1);
  });

  it("Cancel in the role-delete confirm dismisses without removing", async () => {
    const w = mountModal({
      commitments: [
        makeCommitment({ role: "Developer", allocation: 40, goals: ["Ship onboarding v2"] }),
        makeCommitment({ role: "Advisor", allocation: 5, goals: ["Office hours"] }),
      ],
      progress: [
        makeCommitmentProgress({ role: "Developer", goal_spent_minutes: 870, general_spent_minutes: 0, allocation_minutes: 2400, goals: [{ name: "Ship onboarding v2", spent_minutes: 870 }] }),
        makeCommitmentProgress({ role: "Advisor", goal_spent_minutes: 0, general_spent_minutes: 0, allocation_minutes: 300, goals: [{ name: "Office hours", spent_minutes: 0 }] }),
      ],
    });
    await w.findAll("[data-test='role-row']")[1].trigger("click");
    await w.find("[data-test='role-delete']").trigger("click");
    await w.find("[data-test='role-delete-cancel']").trigger("click");
    expect(w.findAll("[data-test='role-row']").length).toBe(2);
    expect(w.find("[data-test='role-delete-confirm']").exists()).toBe(false);
  });

  it("clicking a logged goal's remove does not delete it", async () => {
    const w = mountModal();
    const before = w.findAll("[data-test='goal-name']").length;
    await w.findAll("[data-test='goal-remove']")[0].trigger("click");
    expect(w.findAll("[data-test='goal-name']").length).toBe(before);
  });
});

// ── Validation ──────────────────────────────────────────────────

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
      progress: [makeCommitmentProgress({ role: "Developer", goal_spent_minutes: 0, general_spent_minutes: 0, allocation_minutes: 2400, goals: [{ name: "Ship onboarding v2", spent_minutes: 0 }] })],
    });
    await w.find("[data-test='add-goal']").trigger("click");
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).toHaveBeenCalledWith("set_commitments", expect.objectContaining({
      commitments: [{ role: "Developer", allocation: 40, goals: ["Ship onboarding v2"] }],
    }));
  });

  it("red-borders the offending role field after a blocked save", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("");
    await w.find("[data-test='save']").trigger("click");
    expect(w.find("[data-test='role-name']").classes()).toContain("border-[var(--color-danger)]");
  });
  it("red-borders a duplicate goal field after a blocked save", async () => {
    const w = mountModal({
      commitments: [
        makeCommitment({ role: "Developer", allocation: 40, goals: ["Shared"] }),
        makeCommitment({ role: "Advisor", allocation: 5, goals: ["Shared"] }),
      ],
      progress: [],
    });
    await w.find("[data-test='save']").trigger("click");
    // Only selected role's goals are rendered; both "Shared" are on different roles.
    // We need to check the first role's goal and also select the second role to verify its goal.
    const firstGoalSaved = w.findAll("[data-test='goal-name']")
      .find(i => (i.element as HTMLInputElement).value === "Shared");
    expect(firstGoalSaved!.classes()).toContain("border-[var(--color-danger)]");
    // Switch to second role
    await w.findAll("[data-test='role-row']")[1].trigger("click");
    const secondGoalSaved = w.findAll("[data-test='goal-name']")
      .find(i => (i.element as HTMLInputElement).value === "Shared");
    expect(secondGoalSaved!.classes()).toContain("border-[var(--color-danger)]");
  });

  it("clears the error and saves after the user fixes an invalid field", async () => {
    (invoke as any).mockResolvedValue([]);
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("");
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.text()).toContain("Role name is required");
    await w.find("[data-test='role-name']").setValue("Engineer");
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).toHaveBeenCalledWith("set_commitments", expect.objectContaining({
      commitments: expect.arrayContaining([expect.objectContaining({ role: "Engineer" })]),
    }));
    expect(w.text()).not.toContain("Role name is required");
  });

  it("blocks save with 'At least one role is required' when the draft is empty", async () => {
    const w = mountModal({
      commitments: [makeCommitment({ role: "Solo", allocation: 40, goals: [] })],
      progress: [makeCommitmentProgress({ role: "Solo", allocation_minutes: 2400, goal_spent_minutes: 0, general_spent_minutes: 0, goals: [] })],
    });
    await w.find("[data-test='role-delete']").trigger("click");
    await w.find("[data-test='role-delete-confirm']").trigger("click");
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.text()).toContain("At least one role is required");
  });

  it("error message uses data-test='save-error'", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("");
    await w.find("[data-test='save']").trigger("click");
    expect(w.find("[data-test='save-error']").exists()).toBe(true);
  });
});

// ── Keyboard ────────────────────────────────────────────────────

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
  it("Cmd+Enter in a goal input does NOT insert a goal row (only saves)", async () => {
    (invoke as any).mockResolvedValue([]);
    const w = mountModal();
    const before = w.findAll("[data-test='goal-name']").length;
    await w.findAll("[data-test='goal-name']")[0].trigger("keydown", { key: "Enter", metaKey: true });
    expect(w.findAll("[data-test='goal-name']").length).toBe(before);
    expect(invoke).toHaveBeenCalledWith("set_commitments", expect.anything());
  });

  // New: left panel role navigation
  it("ArrowDown on left panel row moves to next role", async () => {
    const w = mountModal({
      commitments: [
        makeCommitment({ role: "Developer", allocation: 40, goals: [] }),
        makeCommitment({ role: "Advisor", allocation: 5, goals: [] }),
      ],
      progress: [],
    });
    expect(w.find("[data-test='role-row-selected']").text()).toContain("Developer");
    await w.find("[data-test='role-row-selected']").trigger("keydown", { key: "ArrowDown" });
    expect(w.find("[data-test='role-row-selected']").text()).toContain("Advisor");
  });

  it("ArrowUp on left panel row wraps to last role", async () => {
    const w = mountModal({
      commitments: [
        makeCommitment({ role: "Developer", allocation: 40, goals: [] }),
        makeCommitment({ role: "Advisor", allocation: 5, goals: [] }),
      ],
      progress: [],
    });
    expect(w.find("[data-test='role-row-selected']").text()).toContain("Developer");
    await w.find("[data-test='role-row-selected']").trigger("keydown", { key: "ArrowUp" });
    expect(w.find("[data-test='role-row-selected']").text()).toContain("Advisor");
  });

  it("Arrow keys do nothing when only one role", async () => {
    const w = mountModal({
      commitments: [makeCommitment({ role: "Developer", allocation: 40, goals: [] })],
      progress: [],
    });
    await w.find("[data-test='role-row-selected']").trigger("keydown", { key: "ArrowDown" });
    expect(w.find("[data-test='role-row-selected']").text()).toContain("Developer");
  });
});

// ── Close & discard ─────────────────────────────────────────────

describe("CommitmentsModal — close & discard", () => {
  it("moves focus into the dialog on open so esc reaches it", async () => {
    const w = mount(CommitmentsModal, {
      props: { ...baseProps(), open: true },
      attachTo: document.body,
      global: { stubs: { teleport: true } },
    });
    await nextTick();
    await nextTick();
    const overlay = w.find("[data-test='overlay']").element;
    expect(document.activeElement).toBe(overlay);
    w.unmount();
  });
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
  it("Keep editing dismisses the discard confirm without closing", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("Changed");
    await w.find("[data-test='cancel']").trigger("click");
    expect(w.find("[data-test='discard-confirm']").exists()).toBe(true);
    await w.find("[data-test='discard-confirm'] button").trigger("click");
    expect(w.find("[data-test='discard-confirm']").exists()).toBe(false);
    expect(w.emitted("close")).toBeFalsy();
  });
  it("backdrop click with changes shows the discard confirm (does not close)", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("Changed");
    await w.find("[data-test='overlay']").trigger("click");
    expect(w.emitted("close")).toBeFalsy();
    expect(w.find("[data-test='discard-confirm']").exists()).toBe(true);
  });
});

// ── Left panel: role switching preserves draft ──────────────────

describe("CommitmentsModal — role switching", () => {
  it("preserves unsaved goal text when switching between roles", async () => {
    const w = mountModal({
      commitments: [
        makeCommitment({ role: "Developer", allocation: 40, goals: ["Code review"] }),
        makeCommitment({ role: "Advisor", allocation: 5, goals: ["Office hours"] }),
      ],
      progress: [],
    });
    await w.findAll("[data-test='goal-name']")[0].setValue("Updated goal");
    // Switch to Advisor
    await w.findAll("[data-test='role-row']")[1].trigger("click");
    // Switch back to Developer
    await w.findAll("[data-test='role-row']")[0].trigger("click");
    expect((w.findAll("[data-test='goal-name']")[0].element as HTMLInputElement).value).toBe("Updated goal");
  });

  it("switching roles does not trigger save", async () => {
    const w = mountModal({
      commitments: [
        makeCommitment({ role: "Developer", allocation: 40, goals: ["Code review"] }),
        makeCommitment({ role: "Advisor", allocation: 5, goals: ["Office hours"] }),
      ],
      progress: [],
    });
    await w.findAll("[data-test='role-row']")[1].trigger("click");
    expect(invoke).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: 运行测试验证失败**

```bash
pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts
```
预期: 部分旧测试因 dom 结构变更失败（正常——Task 1 的组件改动未提交前在此 step 验证）

- [ ] **Step 3: 提交**

```bash
git add src/__tests__/components/composite/CommitmentsModal.test.ts
git commit -m "test: adapt CommitmentsModal tests for two-column layout"
```

---

### Task 3: 更新 CommitmentsModal.dnd.test.ts

**Files:**
- Modify: `src/__tests__/components/composite/CommitmentsModal.dnd.test.ts`

- [ ] **Step 1: 更新 DnD 焦点稳定性测试**

该测试引用 `RoleCard` 子组件——需改为查询 CommitmentsModal 自身的元素。

```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { invoke } from "@tauri-apps/api/core";
import CommitmentsModal from "../../../components/composite/CommitmentsModal.vue";
import { makeCommitment, makeCommitmentProgress } from "../../mocks/fixtures";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => { (invoke as any).mockReset?.(); (invoke as any).mockResolvedValue?.([]); });

function mountModal() {
  return mount(CommitmentsModal, {
    props: {
      open: true,
      commitments: [makeCommitment({ role: "Developer", allocation: 40, goals: ["A", "B"] })],
      progress: [makeCommitmentProgress({ role: "Developer", allocation_minutes: 2400, goal_spent_minutes: 870, general_spent_minutes: 0, goals: [{ name: "A", spent_minutes: 865 }, { name: "B", spent_minutes: 5 }] })],
      rootPath: "/tmp", selectedYear: 2026, selectedMonth: 6,
    },
    attachTo: document.body,
  });
}

describe("CommitmentsModal — DnD focus stability (real vue-draggable-plus)", () => {
  it("keeps the same allocation input DOM node across a stepper change", async () => {
    const w = mountModal();
    const before = document.querySelector("[data-test='alloc']") as HTMLInputElement;
    before.setAttribute("data-marker", "X");
    await w.find("[data-test='alloc-inc']").trigger("click");
    const after = document.querySelector("[data-test='alloc']") as HTMLInputElement;
    expect(after.getAttribute("data-marker")).toBe("X");
    expect(after.value).toBe("45");
  });
});
```

变更：`w.findComponent({ name: "RoleCard" }).find(...)` → `w.find("[data-test='alloc-inc']")`（不再有 RoleCard 子组件）。

- [ ] **Step 2: 运行测试验证**

```bash
pnpm vitest run src/__tests__/components/composite/CommitmentsModal.dnd.test.ts
```

- [ ] **Step 3: 提交**

```bash
git add src/__tests__/components/composite/CommitmentsModal.dnd.test.ts
git commit -m "test: update DnD focus-stability test for inlined role editor"
```

---

### Task 4: 删除 RoleCard.vue 和 GoalRow.vue

- [ ] **Step 1: 确认无其他引用**

```bash
rg "RoleCard" src/ --type vue --type ts | grep -v "node_modules\|__tests__"
rg "GoalRow" src/ --type vue --type ts | grep -v "node_modules\|__tests__"
```

预期: 只有 CommitmentsModal 自身引用（已被移除）。

- [ ] **Step 2: 删除并提交**

```bash
git rm src/components/composite/RoleCard.vue src/components/composite/GoalRow.vue
git commit -m "refactor: remove RoleCard and GoalRow (inlined into CommitmentsModal)"
```

---

### Task 5: 全量验证

- [ ] **Step 1: 运行全量测试**

```bash
pnpm vitest run
```

预期: 所有测试通过，无 regression。

- [ ] **Step 2: 运行 lint/verify**

```bash
pnpm run verify
```

- [ ] **Step 3: 热重载验证（可选）**

```bash
pnpm tauri dev
```

手动验证: 打开 Commitments Modal → 添加角色、添加目标、切换角色（左侧点击）、拖拽排序、分配步进、保存、Esc 关闭、脏检确认。

---

## 自检清单

- [x] Spec coverage: 两栏布局、左侧角色导航、右侧编辑器、删除确认、整体保存、键盘导航、空状态——均有对应任务覆盖
- [x] Placeholder scan: 无 TBD/TODO/假代码——所有步骤含完整可执行代码
- [x] Type consistency: `RoleRowModel`, `GoalRowModel`, `Commitment`, `CommitmentProgress` 类型在各任务间一致；`data-test` 属性名统一
