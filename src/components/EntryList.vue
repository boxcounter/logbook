<script setup lang="ts">
import type { Entry } from "../types";
import EntryItem from "./EntryItem.vue";

defineProps<{ entries: Entry[] }>();
const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
}>();
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
      />
    </div>
  </div>
</template>
