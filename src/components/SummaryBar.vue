<script setup lang="ts">
import { computed } from "vue";
import type { Entry } from "../types";
import { formatDuration } from "../utils/format";

const props = defineProps<{ entries: Entry[] }>();

const totalMinutes = computed(() => props.entries.reduce((s, e) => s + e.duration, 0));
const entryCount = computed(() => props.entries.length);
</script>

<template>
  <div v-if="entryCount > 0" class="flex justify-between text-xs text-gray-500 px-1">
    <span>{{ entryCount }} {{ entryCount === 1 ? "entry" : "entries" }}</span>
    <span class="font-medium text-gray-700">{{ formatDuration(totalMinutes) }}</span>
  </div>
</template>
