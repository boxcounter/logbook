<script setup lang="ts">
import { ref } from "vue";
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

function enterEdit() {
  if (!props.commitments || props.commitments.length === 0) return;
  const snapshot = JSON.parse(JSON.stringify(props.commitments)) as Commitment[];
  editingCommitments.value = snapshot;
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
  <div v-if="progress.length > 0 || (commitments && commitments.length > 0) || isEditing" class="bg-white rounded-lg shadow-sm p-4">
    <div class="flex justify-between items-center mb-3">
      <h3 class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Commitments</h3>
      <button
        v-if="!isEditing && commitments && commitments.length > 0"
        class="text-xs text-gray-400 hover:text-gray-600 transition-colors cursor-pointer"
        data-test="edit-btn"
        @click="enterEdit"
      >
        ✏️ 编辑
      </button>
    </div>

    <!-- Display mode -->
    <template v-if="!isEditing">
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
    </template>

    <!-- Edit mode -->
    <template v-else>
      <div v-if="editError" class="mb-3 p-2 bg-red-50 border border-red-200 rounded text-xs text-red-700">
        {{ editError }}
      </div>

      <div v-for="(c, ri) in editingCommitments" :key="ri" class="mb-4 last:mb-0">
        <div class="flex items-center gap-2 mb-2">
          <input
            v-model="c.role"
            type="text"
            placeholder="Role"
            class="flex-1 px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <label class="text-xs text-gray-500 whitespace-nowrap">Alloc:</label>
          <input
            v-model.number="c.allocation"
            type="number"
            min="1"
            placeholder="hours"
            class="w-16 px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <span class="text-xs text-gray-400">h</span>
          <button
            v-if="editingCommitments.length > 1"
            class="text-xs text-red-400 hover:text-red-600 cursor-pointer ml-1"
            data-test="delete-role-btn"
            @click="removeRole(ri)"
          >
            删除 Role
          </button>
        </div>

        <div class="ml-4 flex flex-col gap-1.5">
          <div v-for="(_g, gi) in c.goals" :key="gi" class="flex items-center gap-1">
            <input
              v-model="c.goals[gi]"
              type="text"
              placeholder="Goal name"
              class="flex-1 px-2 py-0.5 text-xs border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <button
              class="text-gray-400 hover:text-red-500 text-xs cursor-pointer px-1"
              data-test="delete-goal-btn"
              @click="removeGoal(ri, gi)"
            >
              ✕
            </button>
          </div>
          <button
            class="text-xs text-blue-500 hover:text-blue-700 cursor-pointer self-start"
            data-test="add-goal-btn"
            @click="addGoal(ri)"
          >
            + 添加 Goal
          </button>
        </div>

        <hr v-if="ri < editingCommitments.length - 1" class="my-3 border-gray-100" />
      </div>

      <button
        class="text-xs text-blue-500 hover:text-blue-700 cursor-pointer mt-2"
        data-test="add-role-btn"
        @click="addRole"
      >
        + 添加 Role
      </button>

      <div class="flex justify-end gap-2 mt-4 pt-3 border-t border-gray-100">
        <button
          class="px-3 py-1 text-xs text-gray-600 bg-gray-100 rounded hover:bg-gray-200 cursor-pointer"
          data-test="cancel-btn"
          @click="cancelEdit"
        >
          取消
        </button>
        <button
          class="px-3 py-1 text-xs text-white bg-blue-500 rounded hover:bg-blue-600 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="isSaving"
          data-test="save-btn"
          @click="save"
        >
          {{ isSaving ? "保存中…" : "保存" }}
        </button>
      </div>
    </template>
  </div>
</template>
