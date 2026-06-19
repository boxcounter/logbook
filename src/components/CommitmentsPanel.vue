<script setup lang="ts">
import { ref } from "vue";
import type { Commitment, CommitmentProgress } from "../types";
import { formatDuration } from "../utils/format";
import CommitmentsEditor from "./composite/CommitmentsEditor.vue";

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

function enterEdit() {
  if (!props.commitments || props.commitments.length === 0) return;
  isEditing.value = true;
}

function cancelEdit() {
  isEditing.value = false;
}
</script>

<template>
  <div v-if="progress.length > 0 || (commitments && commitments.length > 0) || isEditing" data-test="commitments-panel" class="bg-[var(--color-surface)] rounded-[var(--radius-card)] shadow-[var(--shadow-card)] p-4">
    <div class="flex justify-between items-center mb-3">
      <h3 class="text-[var(--app-text-xs)] font-bold text-[var(--color-text-secondary)] uppercase tracking-wide">Commitments</h3>
      <button
        v-if="!isEditing && commitments && commitments.length > 0"
        class="text-[var(--app-text-sm)] text-[var(--color-brand-link)] font-medium cursor-pointer"
        data-test="edit-btn"
        @click="enterEdit"
      >
        Edit
      </button>
    </div>

    <!-- Display mode -->
    <template v-if="!isEditing">
      <div v-for="s in progress" :key="s.role" class="mb-4 last:mb-0">
        <div class="flex justify-between items-center text-[var(--app-text-sm)] mb-1">
          <span class="font-semibold text-[var(--color-text-primary)]">{{ s.role }}</span>
          <span class="text-[var(--app-text-sm)] text-[var(--color-text-secondary)]">
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
        <div class="ml-2 flex flex-col gap-0.5 text-[var(--app-text-sm)]">
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
    <CommitmentsEditor
      v-else
      :commitments="commitments || []"
      :root-path="rootPath || ''"
      :selected-year="selectedYear"
      :selected-month="selectedMonth"
      @saved="emit('saved')"
      @cancel="cancelEdit"
    />
  </div>
</template>
