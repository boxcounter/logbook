<script setup lang="ts">
import type { Entry, Granularity } from "../types";
import { computed } from "vue";
import EntryItem from "./EntryItem.vue";
import EntryGroup from "./EntryGroup.vue";

const props = defineProps<{
  entries: Entry[];
  granularity: Granularity;
  periodEntries?: Record<string, Entry[]>;
}>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
  updateDimensions: [entryId: string, dimensions: Record<string, string>];
}>();

interface Group {
  label: string;
  entries: Entry[];
}

const groups = computed<Group[]>(() => {
  if (props.granularity === "day") {
    return [];
  }
  if (!props.periodEntries) return [];

  if (props.granularity === "week") {
    const sorted = Object.keys(props.periodEntries).sort();
    const result: Group[] = [];
    for (const date of sorted) {
      const entries = props.periodEntries[date];
      if (entries.length === 0) continue;
      const d = new Date(date + "T00:00:00");
      const label = d.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" });
      result.push({ label, entries });
    }
    return result;
  }

  // Month: group by week
  const weeks: Record<string, Entry[]> = {};
  for (const [date, entries] of Object.entries(props.periodEntries)) {
    if (entries.length === 0) continue;
    const d = new Date(date + "T00:00:00");
    const day = d.getDay();
    const weekStart = new Date(d);
    weekStart.setDate(d.getDate() - (day === 0 ? 6 : day - 1));
    const weekEnd = new Date(weekStart);
    weekEnd.setDate(weekStart.getDate() + 6);
    const fmt = (dt: Date) => dt.toLocaleDateString("en-US", { month: "short", day: "numeric" });
    const key = `${fmt(weekStart)} – ${fmt(weekEnd)}`;
    if (!weeks[key]) weeks[key] = [];
    weeks[key].push(...entries);
  }
  const result: Group[] = [];
  for (const [label, entries] of Object.entries(weeks)) {
    result.push({ label, entries });
  }
  return result;
});
</script>

<template>
  <div class="bg-white rounded-lg shadow-sm">
    <div v-if="granularity === 'day' && entries.length === 0" class="p-8 text-center text-gray-400 text-sm">
      No entries yet. Log your first work item above.
    </div>
    <div v-else-if="granularity !== 'day' && groups.length === 0" class="p-8 text-center text-gray-400 text-sm">
      No entries for this period.
    </div>
    <!-- Day mode: flat list -->
    <div v-else-if="granularity === 'day'" class="px-4">
      <EntryItem
        v-for="(entry, index) in entries"
        :key="entry.id"
        :entry="entry"
        :index="index"
        @update="(entryId, item, dur) => emit('update', entryId, item, dur)"
        @delete="(entryId) => emit('delete', entryId)"
        @update-dimensions="(entryId, dims) => emit('updateDimensions', entryId, dims)"
      />
    </div>
    <!-- Week/Month: grouped -->
    <EntryGroup
      v-else
      v-for="group in groups"
      :key="group.label"
      :label="group.label"
      :entries="group.entries"
      :defaultOpen="true"
      @update="(entryId, item, dur) => emit('update', entryId, item, dur)"
      @delete="(entryId) => emit('delete', entryId)"
      @update-dimensions="(entryId, dims) => emit('updateDimensions', entryId, dims)"
    />
  </div>
</template>
