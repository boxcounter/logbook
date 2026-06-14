<script setup lang="ts">
import type { Entry } from "../types";
import { computed } from "vue";
import EntryItem from "./EntryItem.vue";
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
  <div class="bg-white rounded-lg shadow-sm">
    <div v-if="entries.length === 0" class="p-8 text-center text-gray-400 text-sm">
      No entries yet. Log your first work item above.
    </div>
    <div v-else class="px-4">
      <EntryItem
        v-for="(entry, index) in entries"
        :key="entry.id"
        :entry="entry"
        :index="index"
        @update="(entryId, item, dur) => emit('update', entryId, item, dur)"
        @delete="(entryId) => emit('delete', entryId)"
        @update-dimensions="(entryId, dims) => emit('updateDimensions', entryId, dims)"
      />
      <!-- Inline summary row -->
      <div class="flex justify-between text-xs text-gray-500 py-2 border-t border-gray-200 mt-2">
        <span>{{ entryCount }} {{ entryCount === 1 ? "entry" : "entries" }}</span>
        <span class="font-medium text-gray-700">{{ formatDuration(totalMinutes) }}</span>
      </div>
    </div>
  </div>
</template>
