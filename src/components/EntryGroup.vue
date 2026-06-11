<script setup lang="ts">
import { ref } from "vue";
import type { Entry } from "../types";
import { formatDuration } from "../utils/format";
import EntryItem from "./EntryItem.vue";

const props = defineProps<{
  label: string;
  entries: Entry[];
  defaultOpen?: boolean;
}>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
}>();

const open = ref(props.defaultOpen ?? true);
</script>

<template>
  <div class="border-b border-gray-100 last:border-b-0">
    <button
      class="w-full flex items-center justify-between px-4 py-2 hover:bg-gray-50 text-left"
      @click="open = !open"
    >
      <span class="text-sm font-medium text-gray-600">{{ label }}</span>
      <span class="text-xs text-gray-400">
        {{ entries.length }} {{ entries.length === 1 ? "entry" : "entries" }}
        · {{ formatDuration(entries.reduce((s, e) => s + e.duration, 0)) }}
      </span>
    </button>
    <div v-if="open" class="px-4 pb-2">
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
