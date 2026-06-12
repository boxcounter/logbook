<script setup lang="ts">
import { computed } from "vue";
import type { Commitment, Entry } from "../types";
import { formatDuration } from "../utils/format";

const props = defineProps<{
  commitments: Commitment[];
  entries: Entry[];
}>();

interface GoalStat {
  name: string;
  spent: number;
}

interface CommitmentStat {
  role: string;
  allocationMinutes: number;
  spentMinutes: number;
  goals: GoalStat[];
}

const stats = computed<CommitmentStat[]>(() => {
  return props.commitments.map((c) => {
    const WORKING_DAYS_PER_MONTH = 20;
    const dailyAllocation = Math.round((c.allocation * 60) / WORKING_DAYS_PER_MONTH);
    const goals: GoalStat[] = c.goals.map((name) => ({
      name,
      spent: props.entries
        .filter((e) => e.dimensions["goal"] === name)
        .reduce((sum, e) => sum + e.duration, 0),
    }));
    const spentMinutes = goals.reduce((sum, g) => sum + g.spent, 0);
    return { role: c.role, allocationMinutes: dailyAllocation, spentMinutes, goals };
  });
});

function pct(spent: number, alloc: number): string {
  if (alloc === 0) return "0%";
  return Math.min(100, Math.round((spent / alloc) * 100)) + "%";
}

function barColor(spent: number, alloc: number): string {
  if (alloc === 0) return "bg-gray-300";
  const ratio = spent / alloc;
  if (ratio > 1) return "bg-red-500";
  if (ratio > 0.8) return "bg-yellow-500";
  return "bg-green-500";
}
</script>

<template>
  <div v-if="stats.length > 0" class="bg-white rounded-lg shadow-sm p-4">
    <h3 class="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-3">Commitments</h3>
    <div v-for="s in stats" :key="s.role" class="mb-4 last:mb-0">
      <div class="flex justify-between items-center text-sm mb-1">
        <span class="font-semibold text-gray-700">{{ s.role }}</span>
        <span class="text-gray-500 text-xs">
          {{ formatDuration(s.spentMinutes) }} / {{ (s.allocationMinutes / 60).toFixed(1) }}h
        </span>
      </div>
      <div class="h-1.5 bg-gray-100 rounded-full overflow-hidden mb-2">
        <div
          :class="barColor(s.spentMinutes, s.allocationMinutes)"
          class="h-full rounded-full transition-all"
          :style="{ width: pct(s.spentMinutes, s.allocationMinutes) }"
        />
      </div>
      <div class="ml-2 flex flex-col gap-0.5 text-xs">
        <div
          v-for="g in s.goals"
          :key="g.name"
          class="flex justify-between"
          :class="g.spent > 0 ? 'text-gray-600' : 'text-gray-300'"
        >
          <span>{{ g.name }}</span>
          <span v-if="g.spent > 0" class="font-medium text-gray-700">{{ formatDuration(g.spent) }}</span>
          <span v-else>0m</span>
        </div>
      </div>
    </div>
  </div>
</template>
