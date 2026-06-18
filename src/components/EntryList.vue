<script setup lang="ts">
import type { Entry } from "../types";
import { computed } from "vue";
import EntryRow from "./composite/EntryRow.vue";
import { formatDuration } from "../utils/format";

const props = defineProps<{
  entries: Entry[];
}>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
  updateDimensions: [entryId: string, dimensions: Record<string, string>];
}>();

const totalMinutes = computed(() =>
  props.entries.reduce((s, e) => s + e.duration, 0)
);

const entryCount = computed(() => props.entries.length);
</script>

<template>
  <div class="bg-[var(--color-surface)] rounded-[var(--radius-card)] shadow-[var(--shadow-card)]">
    <div v-if="entries.length === 0" class="p-8 text-center text-[var(--color-text-secondary)] text-sm">
      No entries yet. Log your first work item above.
    </div>
    <div v-else class="px-4">
      <EntryRow
        v-for="(entry, index) in entries"
        :key="entry.id"
        :entry="entry"
        :index="index"
        @update="(entryId, item, dur) => emit('update', entryId, item, dur)"
        @delete="(entryId) => emit('delete', entryId)"
        @update-dimensions="(entryId, dims) => emit('updateDimensions', entryId, dims)"
      />
      <div class="flex justify-between text-[13px] text-[var(--color-text-secondary)] py-3 border-t-2 border-[var(--color-divider)] mt-1">
        <span>{{ entryCount }} {{ entryCount === 1 ? "entry" : "entries" }}</span>
        <span class="font-bold text-[15px] text-[var(--color-brand-link)]">{{ formatDuration(totalMinutes) }}</span>
      </div>
    </div>
  </div>
</template>
