<script setup lang="ts">
import { computed } from "vue";
import type { Granularity, Entry } from "../types";
import { formatDuration } from "../utils/format";

const props = defineProps<{
  entries: Entry[];
  granularity: Granularity;
  periodEntries?: Record<string, Entry[]>;
}>();

const totalMinutes = computed(() => {
  if (props.granularity === "day") {
    return props.entries.reduce((s, e) => s + e.duration, 0);
  }
  if (!props.periodEntries) return 0;
  return Object.values(props.periodEntries)
    .flat()
    .reduce((s, e) => s + e.duration, 0);
});

const entryCount = computed(() => {
  if (props.granularity === "day") {
    return props.entries.length;
  }
  if (!props.periodEntries) return 0;
  return Object.values(props.periodEntries).reduce((s, arr) => s + arr.length, 0);
});

function dateLabel(dateStr: string): string {
  const d = new Date(dateStr + "T00:00:00");
  return d.toLocaleDateString("en-US", { weekday: "short", day: "numeric" });
}

const daySummaries = computed(() => {
  if (props.granularity !== "week" || !props.periodEntries) return null;
  const sorted = Object.keys(props.periodEntries).sort();
  return sorted.map(date => ({
    label: dateLabel(date),
    minutes: props.periodEntries![date].reduce((s, e) => s + e.duration, 0),
  }));
});

const weekSummaries = computed(() => {
  if (props.granularity !== "month" || !props.periodEntries) return null;
  const weeks: Record<string, number> = {};
  for (const [date, entries] of Object.entries(props.periodEntries)) {
    const d = new Date(date + "T00:00:00");
    const day = d.getDay();
    const weekStart = new Date(d);
    weekStart.setDate(d.getDate() - (day === 0 ? 6 : day - 1));
    const weekEnd = new Date(weekStart);
    weekEnd.setDate(weekStart.getDate() + 6);
    const fmt = (dt: Date) =>
      dt.toLocaleDateString("en-US", { month: "short", day: "numeric" });
    const key = `${fmt(weekStart)} – ${fmt(weekEnd)}`;
    weeks[key] =
      (weeks[key] || 0) + entries.reduce((s, e) => s + e.duration, 0);
  }
  return Object.entries(weeks).map(([label, minutes]) => ({ label, minutes }));
});
</script>

<template>
  <div v-if="entryCount > 0" class="text-xs text-gray-500 px-1 space-y-1">
    <!-- Day mode: single total -->
    <template v-if="granularity === 'day'">
      <div class="flex justify-between">
        <span>{{ entryCount }} {{ entryCount === 1 ? "entry" : "entries" }}</span>
        <span class="font-medium text-gray-700">{{ formatDuration(totalMinutes) }}</span>
      </div>
    </template>
    <!-- Week mode: day subtotals + week total -->
    <template v-else-if="granularity === 'week' && daySummaries">
      <div
        v-for="day in daySummaries"
        :key="day.label"
        class="flex justify-between ml-2"
      >
        <span>{{ day.label }}</span>
        <span>{{ formatDuration(day.minutes) }}</span>
      </div>
      <div
        class="flex justify-between font-medium text-gray-700 pt-1 border-t border-gray-200"
      >
        <span>Week total</span>
        <span>{{ formatDuration(totalMinutes) }}</span>
      </div>
    </template>
    <!-- Month mode: week subtotals + month total -->
    <template v-else-if="granularity === 'month' && weekSummaries">
      <div
        v-for="week in weekSummaries"
        :key="week.label"
        class="flex justify-between ml-2"
      >
        <span>{{ week.label }}</span>
        <span>{{ formatDuration(week.minutes) }}</span>
      </div>
      <div
        class="flex justify-between font-medium text-gray-700 pt-1 border-t border-gray-200"
      >
        <span>Month total</span>
        <span>{{ formatDuration(totalMinutes) }}</span>
      </div>
    </template>
  </div>
</template>
