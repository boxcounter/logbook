<!-- src/components/CommitmentsPanel.vue -->
<script setup lang="ts">
import { ref, computed } from "vue";
import type { Commitment, CommitmentProgress } from "../types";
import { formatDurationCompact } from "../utils/format";
import CommitmentsModal from "./composite/CommitmentsModal.vue";

const props = defineProps<{
  progress: CommitmentProgress[];
  commitments?: Commitment[];
  rootPath?: string;
  selectedYear: number;
  selectedMonth: number;
}>();

const emit = defineEmits<{ saved: [Commitment[]] }>();

// Roles start expanded; clicking the role header toggles its goal list.
const collapsed = ref<Record<string, boolean>>({});
function toggle(role: string) { collapsed.value[role] = !collapsed.value[role]; }

function pct(spent: number, alloc: number): string {
  if (alloc === 0) return "0%";
  return Math.min(100, Math.round((spent / alloc) * 100)) + "%";
}

const hasCommitments = computed(() => !!props.commitments && props.commitments.length > 0);

const modalOpen = ref(false);
function openEditor() { modalOpen.value = true; }
function closeEditor() { modalOpen.value = false; }
function onSaved(c: Commitment[]) { modalOpen.value = false; emit("saved", c); }
</script>

<template>
  <div data-test="commitments-panel">
    <div class="flex justify-between items-center mb-sm">
      <h3 class="text-micro font-bold text-[var(--color-text-secondary)] uppercase tracking-[0.5px]">Commitments</h3>
      <button
        v-if="hasCommitments"
        class="text-secondary text-[var(--color-brand-link)] font-medium cursor-pointer"
        data-test="edit-btn"
        @click="openEditor"
      >Edit</button>
    </div>

    <div v-for="s in progress" :key="s.role" class="mb-lg last:mb-0">
      <div
        data-test="role-toggle"
        class="flex justify-between items-center cursor-pointer rounded-[var(--radius-form-lg)] px-sm py-sm mb-2xs hover:bg-[var(--color-divider)]"
        @click="toggle(s.role)"
      >
        <span class="text-secondary font-semibold text-[var(--color-text-primary)]">
          {{ s.role }} {{ collapsed[s.role] ? "▸" : "▾" }}
        </span>
        <span class="text-secondary font-semibold text-[var(--color-text-primary)]">
          <span class="mono">{{ formatDurationCompact(s.goal_spent_minutes + s.general_spent_minutes) }}</span><span class="mono font-normal text-[var(--color-text-secondary)]"> / {{ (s.allocation_minutes / 60).toFixed(0) }}h</span>
        </span>
      </div>
      <div class="h-[4px] bg-[var(--color-divider)] rounded-[var(--radius-sm)] overflow-hidden mt-xs">
        <div
          data-test="progress-fill"
          class="h-full rounded-[var(--radius-sm)] transition-all"
          :style="{ width: pct(s.goal_spent_minutes + s.general_spent_minutes, s.allocation_minutes), background: 'linear-gradient(90deg, var(--color-brand-gradient-from), var(--color-brand-gradient-to))' }"
        />
      </div>
      <div v-if="!collapsed[s.role]" class="mt-sm flex flex-col gap-2xs">
        <div
          v-for="g in s.goals" :key="g.name"
          data-test="goal-row"
          class="flex justify-between text-secondary text-[var(--color-text-secondary)] py-xs pl-sm"
        >
          <span class="overflow-hidden text-ellipsis whitespace-nowrap flex-1 min-w-0" :title="g.name">{{ g.name }}</span>
          <span v-if="g.spent_minutes > 0" class="mono font-medium text-[var(--color-text-primary)] flex-shrink-0 ml-sm">{{ formatDurationCompact(g.spent_minutes) }}</span>
          <span v-else class="mono text-[var(--color-text-secondary)] flex-shrink-0 ml-sm">0</span>
        </div>
      </div>
    </div>

    <button
      v-if="!hasCommitments"
      data-test="setup-btn"
      class="mt-xs text-secondary text-[var(--color-brand-link)] font-medium cursor-pointer hover:underline"
      @click="openEditor"
    >+ Set up commitments</button>

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
  </div>
</template>
