<script setup lang="ts">
import { ref, computed } from "vue";
import { parseDurationFromText, stripDurations, formatDuration } from "../utils/format";

const input = ref("");
const error = ref("");

const emit = defineEmits<{
  submit: [item: string, durationMinutes: number];
}>();

const parsedPreview = computed(() => {
  if (!input.value.trim()) return null;
  const d = parseDurationFromText(input.value.trim());
  if (!d) return null;
  return `${formatDuration(d)} (${d}m)`;
});

function handleSubmit() {
  error.value = "";
  const trimmed = input.value.trim();
  if (!trimmed) return;
  const d = parseDurationFromText(trimmed);
  if (!d) {
    error.value = "Could not parse duration. Examples: 1.5h, 30m, 45";
    return;
  }
  const item = stripDurations(trimmed);
  emit("submit", item, d);
  input.value = "";
}
</script>

<template>
  <div>
    <div class="flex gap-2">
      <input
        v-model="input"
        type="text"
        class="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-sm"
        placeholder="What did you work on? (e.g. Sprint planning 1.5h)"
        @keydown.enter="handleSubmit"
      />
      <button
        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 text-sm font-medium"
        :disabled="!input.trim()"
        @click="handleSubmit"
      >
        Log
      </button>
    </div>
    <div class="flex justify-between mt-1 min-h-[1.25rem]">
      <span v-if="parsedPreview" class="text-xs text-gray-500">Duration: {{ parsedPreview }}</span>
      <span v-if="error" class="text-xs text-red-500">{{ error }}</span>
    </div>
  </div>
</template>
