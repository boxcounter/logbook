<script setup lang="ts">
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
</script>

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
