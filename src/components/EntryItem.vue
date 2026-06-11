<script setup lang="ts">
import { ref } from "vue";
import type { Entry } from "../types";
import { formatDuration, resolveDelta } from "../utils/format";

const props = defineProps<{
  entry: Entry;
  index: number;
}>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
}>();

// Item text editing
const editingItem = ref(false);
const itemInput = ref("");

function startEditItem() {
  itemInput.value = props.entry.item;
  editingItem.value = true;
}

function commitItem() {
  editingItem.value = false;
  const newItem = itemInput.value.trim() || "(untitled)";
  if (newItem !== props.entry.item) {
    emit("update", props.entry.id, newItem, props.entry.duration);
  }
}

function cancelItem() {
  editingItem.value = false;
}

function handleItemKey(e: KeyboardEvent) {
  if (e.key === "Enter") { e.preventDefault(); commitItem(); }
  if (e.key === "Escape") { e.preventDefault(); cancelItem(); }
}

// Duration editing
const editingDuration = ref(false);
const durInput = ref("");

function startEditDuration() {
  durInput.value = String(props.entry.duration);
  editingDuration.value = true;
}

function commitDuration() {
  editingDuration.value = false;
  const newDur = resolveDelta(durInput.value, props.entry.duration);
  if (newDur !== props.entry.duration) {
    emit("update", props.entry.id, props.entry.item, newDur);
  }
}

function cancelDuration() {
  editingDuration.value = false;
}

function handleDurKey(e: KeyboardEvent) {
  if (e.key === "Enter") { e.preventDefault(); commitDuration(); }
  if (e.key === "Escape") { e.preventDefault(); cancelDuration(); }
}

function dimLabel(dims: Record<string, string>): string {
  return Object.entries(dims)
    .filter(([, v]) => v)
    .map(([, v]) => v)
    .join(" · ");
}
</script>

<template>
  <div class="flex items-start gap-3 py-3 border-b border-gray-100 last:border-b-0 group">
    <span class="text-xs text-gray-400 w-5 text-right pt-0.5 tabular-nums shrink-0">{{ index + 1 }}</span>
    <div class="flex-1 min-w-0">
      <!-- Item text: double-click to edit -->
      <div v-if="editingItem" class="text-sm">
        <input
          v-model="itemInput"
          class="w-full px-1 py-0.5 border-2 border-blue-500 rounded text-sm outline-none"
          @keydown="handleItemKey"
          @blur="commitItem"
        />
      </div>
      <div
        v-else
        class="text-sm text-gray-800 cursor-default rounded px-0.5 -mx-0.5 hover:bg-gray-50"
        @dblclick="startEditItem"
      >
        {{ entry.item }}
      </div>
      <div v-if="dimLabel(entry.dimensions)" class="text-xs text-gray-400 mt-0.5">
        {{ dimLabel(entry.dimensions) }}
      </div>
    </div>
    <!-- Duration: double-click to edit -->
    <span v-if="editingDuration" class="text-sm shrink-0">
      <input
        v-model="durInput"
        class="w-14 text-right px-1 py-0.5 border-2 border-blue-500 rounded text-sm outline-none tabular-nums"
        @keydown="handleDurKey"
        @blur="commitDuration"
      />
    </span>
    <span
      v-else
      class="text-sm text-gray-600 tabular-nums shrink-0 cursor-default rounded px-1 hover:bg-gray-50"
      @dblclick="startEditDuration"
    >
      {{ formatDuration(entry.duration) }}
    </span>
    <!-- Delete: hover only -->
    <button
      class="text-gray-400 hover:text-red-500 text-base leading-none shrink-0 opacity-0 group-hover:opacity-100 transition-opacity p-0.5"
      title="Delete"
      @click="emit('delete', props.entry.id)"
    >
      ×
    </button>
  </div>
</template>
