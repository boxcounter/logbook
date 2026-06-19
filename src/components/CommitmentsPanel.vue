<script setup lang="ts">
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Commitment, CommitmentProgress } from "../types";
import { formatDuration } from "../utils/format";

const props = defineProps<{
  progress: CommitmentProgress[];
  commitments?: Commitment[];
  rootPath?: string;
  selectedYear: number;
  selectedMonth: number;
}>();

const emit = defineEmits<{
  saved: [];
}>();

// ---- Display mode helpers ----

function pct(spent: number, alloc: number): string {
  if (alloc === 0) return "0%";
  return Math.min(100, Math.round((spent / alloc) * 100)) + "%";
}

function barColor(spent: number, alloc: number): string {
  if (alloc === 0) return "bg-gray-300";
  const spentRatio = spent / alloc;
  if (spentRatio > 1) return "bg-red-500";
  const elapsed = elapsedRatio();
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
    const daysInMonth = new Date(props.selectedYear, props.selectedMonth, 0).getDate();
    return now.getDate() / daysInMonth;
  }
  return 1.0;
}

// ---- Edit mode ----

const isEditing = ref(false);
const editingCommitments = ref<Commitment[]>([]);
const editError = ref("");
const isSaving = ref(false);
const lastSavedCommitments = ref<Commitment[]>([]);

// Watch for external changes while editing (file watcher pushes new data)
watch(
  () => props.commitments,
  (newVal, oldVal) => {
    if (!isEditing.value) return;
    if (!newVal || !oldVal) return;
    if (JSON.stringify(newVal) === JSON.stringify(oldVal)) return;
    if (JSON.stringify(newVal) === JSON.stringify(lastSavedCommitments.value)) return;

    // External modification detected — exit edit mode, display refreshes
    isEditing.value = false;
    editingCommitments.value = [];
    editError.value = "";
  }
);

function enterEdit() {
  if (!props.commitments || props.commitments.length === 0) return;
  const snapshot = JSON.parse(JSON.stringify(props.commitments)) as Commitment[];
  editingCommitments.value = snapshot;
  lastSavedCommitments.value = JSON.parse(JSON.stringify(props.commitments)) as Commitment[];
  editError.value = "";
  isEditing.value = true;
}

function cancelEdit() {
  isEditing.value = false;
  editingCommitments.value = [];
  editError.value = "";
}

function addGoal(roleIndex: number) {
  editingCommitments.value[roleIndex].goals.push("");
}

function removeGoal(roleIndex: number, goalIndex: number) {
  editingCommitments.value[roleIndex].goals.splice(goalIndex, 1);
}

function addRole() {
  editingCommitments.value.push({ role: "", allocation: 0, goals: [] });
}

function removeRole(roleIndex: number) {
  if (editingCommitments.value.length <= 1) return;
  editingCommitments.value.splice(roleIndex, 1);
}

// ---- Frontend pre-validation ----

function preValidate(): string | null {
  if (editingCommitments.value.length === 0) {
    return "At least one role is required";
  }
  for (const c of editingCommitments.value) {
    if (!c.role.trim()) {
      return "Role name cannot be empty";
    }
    if (c.allocation === 0 || !c.allocation) {
      return `Allocation for '${c.role || "unnamed"}' must be greater than 0`;
    }
    for (const g of c.goals) {
      if (!g.trim()) {
        return "Goal name cannot be empty";
      }
    }
  }
  return null;
}

// ---- Save ----

async function save() {
  const err = preValidate();
  if (err) {
    editError.value = err;
    return;
  }

  if (!props.rootPath) return;

  isSaving.value = true;
  editError.value = "";

  try {
    await invoke("set_commitments", {
      rootPath: props.rootPath,
      year: props.selectedYear,
      month: props.selectedMonth,
      commitments: editingCommitments.value.map((c) => ({
        role: c.role.trim(),
        allocation: c.allocation,
        goals: c.goals.map((g) => g.trim()).filter((g) => g !== ""),
      })),
    });

    isEditing.value = false;
    lastSavedCommitments.value = JSON.parse(JSON.stringify(editingCommitments.value)) as Commitment[];
    editingCommitments.value = [];
    emit("saved");
  } catch (e) {
    editError.value = typeof e === "string" ? e : String(e);
  } finally {
    isSaving.value = false;
  }
}

defineExpose({ editingCommitments });
</script>

<template>
  <div v-if="progress.length > 0 || (commitments && commitments.length > 0) || isEditing" data-test="commitments-panel" class="bg-[var(--color-surface)] rounded-[var(--radius-card)] shadow-[var(--shadow-card)] p-4">
    <div class="flex justify-between items-center mb-3">
      <h3 class="text-[var(--text-xs)] font-bold text-[var(--color-text-secondary)] uppercase tracking-wide">Commitments</h3>
      <button
        v-if="!isEditing && commitments && commitments.length > 0"
        class="text-[var(--text-sm)] text-[var(--color-brand-link)] font-medium cursor-pointer"
        data-test="edit-btn"
        @click="enterEdit"
      >
        Edit
      </button>
    </div>

    <!-- Display mode -->
    <template v-if="!isEditing">
      <div v-for="s in progress" :key="s.role" class="mb-4 last:mb-0">
        <div class="flex justify-between items-center text-[var(--text-sm)] mb-1">
          <span class="font-semibold text-[var(--color-text-primary)]">{{ s.role }}</span>
          <span class="text-[var(--text-sm)] text-[var(--color-text-secondary)]">
            {{ formatDuration(s.spent_minutes) }} / {{ (s.allocation_minutes / 60).toFixed(1) }}h
          </span>
        </div>
        <div data-test="progress-bar" class="h-[8px] bg-[var(--color-divider)] rounded-[4px] overflow-hidden mb-2">
          <div
            data-test="progress-fill"
            :class="barColor(s.spent_minutes, s.allocation_minutes)"
            class="h-full rounded-[4px] transition-all"
            :style="{ width: pct(s.spent_minutes, s.allocation_minutes) }"
          />
        </div>
        <div class="ml-2 flex flex-col gap-0.5 text-[var(--text-sm)]">
          <div
            v-for="g in s.goals"
            :key="g.name"
            data-test="goal-row"
            class="flex justify-between"
            :class="g.spent_minutes > 0 ? 'text-[var(--color-text-secondary)]' : 'text-[var(--color-text-secondary)] opacity-40'"
          >
            <span>{{ g.name }}</span>
            <span v-if="g.spent_minutes > 0" class="font-medium text-[var(--color-text-primary)]">{{ formatDuration(g.spent_minutes) }}</span>
            <span v-else>0m</span>
          </div>
        </div>
      </div>
    </template>

    <!-- Edit mode -->
    <template v-else>
      <div v-if="editError" class="mb-3 p-2 bg-red-50 border border-red-200 rounded-[var(--radius-form)] text-[var(--text-sm)] text-[var(--color-danger)]">
        {{ editError }}
      </div>

      <div v-for="(c, ri) in editingCommitments" :key="ri" class="mb-4 last:mb-0">
        <div class="flex items-center gap-2 mb-2">
          <input
            v-model="c.role"
            type="text"
            placeholder="Role"
            class="flex-1 border-2 border-[var(--color-border-form)] rounded-[var(--radius-form)] text-[var(--text-base)] bg-[var(--color-surface)] text-[var(--color-text-primary)] px-[12px] py-[8px] outline-none focus:border-[var(--color-brand-solid)] focus:bg-[#fafaff] focus:shadow-[var(--shadow-focus-ring)] transition-all duration-200"
          />
          <label class="text-[var(--text-sm)] text-[var(--color-text-secondary)] whitespace-nowrap">Alloc:</label>
          <input
            v-model.number="c.allocation"
            type="number"
            min="1"
            placeholder="hours"
            class="w-16 border-2 border-[var(--color-border-form)] rounded-[var(--radius-form)] text-[var(--text-base)] bg-[var(--color-surface)] text-[var(--color-text-primary)] px-[12px] py-[8px] outline-none focus:border-[var(--color-brand-solid)] focus:bg-[#fafaff] focus:shadow-[var(--shadow-focus-ring)] transition-all duration-200"
          />
          <span class="text-[var(--text-sm)] text-[var(--color-text-secondary)]">h</span>
          <button
            v-if="editingCommitments.length > 1"
            class="text-[var(--text-sm)] text-[var(--color-text-secondary)] hover:text-[var(--color-danger)] cursor-pointer transition-colors ml-1"
            data-test="delete-role-btn"
            @click="removeRole(ri)"
          >
            Delete Role
          </button>
        </div>

        <div class="ml-4 flex flex-col gap-1.5">
          <div v-for="(_g, gi) in c.goals" :key="gi" class="flex items-center gap-1">
            <input
              v-model="c.goals[gi]"
              type="text"
              placeholder="Goal name"
              class="flex-1 border-2 border-[var(--color-border-form)] rounded-[var(--radius-form)] text-[var(--text-base)] bg-[var(--color-surface)] text-[var(--color-text-primary)] px-[12px] py-[8px] outline-none focus:border-[var(--color-brand-solid)] focus:bg-[#fafaff] focus:shadow-[var(--shadow-focus-ring)] transition-all duration-200"
            />
            <button
              class="text-[var(--text-sm)] text-[var(--color-text-secondary)] hover:text-[var(--color-danger)] cursor-pointer transition-colors px-1"
              data-test="delete-goal-btn"
              @click="removeGoal(ri, gi)"
            >
              ✕
            </button>
          </div>
          <button
            class="text-[var(--text-sm)] text-[var(--color-brand-link)] font-medium cursor-pointer self-start"
            data-test="add-goal-btn"
            @click="addGoal(ri)"
          >
            + Add Goal
          </button>
        </div>

        <hr v-if="ri < editingCommitments.length - 1" class="my-3 border-[var(--color-divider)]" />
      </div>

      <button
        class="text-[var(--text-sm)] text-[var(--color-brand-link)] font-medium cursor-pointer mt-2"
        data-test="add-role-btn"
        @click="addRole"
      >
        + Add Role
      </button>

      <div class="flex justify-end gap-2 mt-4 pt-3 border-t border-[var(--color-divider)]">
        <button
          class="bg-[var(--color-divider)] text-[var(--color-text-secondary)] rounded-full px-[16px] py-[7px] text-[var(--text-sm)] font-semibold cursor-pointer hover:opacity-80 transition-all"
          data-test="cancel-btn"
          @click="cancelEdit"
        >
          Cancel
        </button>
        <button
          class="bg-gradient-to-br from-[var(--color-brand-gradient-from)] to-[var(--color-brand-gradient-to)] text-white rounded-full px-[16px] py-[7px] text-[var(--text-sm)] font-semibold cursor-pointer hover:-translate-y-px hover:shadow-[var(--shadow-card)] transition-all disabled:opacity-50"
          :disabled="isSaving"
          data-test="save-btn"
          @click="save"
        >
          {{ isSaving ? "Saving..." : "Save" }}
        </button>
      </div>
    </template>
  </div>
</template>
