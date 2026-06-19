<!-- src/components/EntryList.vue -->
<script setup lang="ts">
import type { Entry } from "../types";
import EntryRow from "./composite/EntryRow.vue";

defineProps<{ entries: Entry[]; justAddedId?: string | null }>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
  updateDimensions: [entryId: string, dimensions: Record<string, string>];
}>();
</script>

<template>
  <div class="flex-1 flex flex-col gap-[2px] overflow-y-auto pr-[4px]">
    <div v-if="entries.length === 0" class="p-8 text-center text-[var(--color-text-secondary)] text-[length:var(--app-text-sm)]">
      No entries yet. Log your first work item below.
    </div>
    <EntryRow
      v-for="(entry, index) in entries"
      :key="entry.id"
      :entry="entry"
      :index="index"
      :just-added="entry.id === justAddedId"
      @update="(id, item, dur) => emit('update', id, item, dur)"
      @delete="(id) => emit('delete', id)"
      @update-dimensions="(id, dims) => emit('updateDimensions', id, dims)"
    />
  </div>
</template>
