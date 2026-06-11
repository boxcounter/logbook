<script setup lang="ts">
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../stores/useStore";
import EntryInput from "./EntryInput.vue";
import DimensionPanel from "./DimensionPanel.vue";
import type { Dimension, DayFile } from "../types";
import { logError } from "../utils/errorLog";

const emit = defineEmits<{
  submit: [item: string, durationMinutes: number];
  appended: [];
}>();

const store = useStore();
const showDimensions = ref(false);
const dimValues = ref<Record<string, string>>({});

watch(
  () => store.lastDimensions,
  (ld) => { if (Object.keys(ld).length > 0) dimValues.value = { ...ld }; },
  { immediate: true }
);

function sanitizeValues(vals: Record<string, string>, dims: Dimension[]): Record<string, string> {
  const validKeys = new Set(dims.map((d) => d.key));
  const cleaned: Record<string, string> = {};
  for (const [k, v] of Object.entries(vals)) {
    if (validKeys.has(k)) cleaned[k] = v;
  }
  return cleaned;
}

async function handleSubmit(item: string, durationMinutes: number) {
  const dimensions = sanitizeValues(dimValues.value, store.config?.dimensions || []);
  const newEntry = { item, duration: String(durationMinutes), dimensions };

  try {
    await invoke("append_entry", { rootPath: store.rootPath, date: store.currentDate, entry: newEntry });
    store.lastDimensions = { ...dimensions };
    await refreshDay();
    emit("appended");
    dimValues.value = {};
  } catch (e) {
    logError("QuickEntry.handleSubmit", e);
    console.error("append_entry failed:", e);
  }
}

async function refreshDay() {
  const dayFile = (await invoke("get_entries", { rootPath: store.rootPath, date: store.currentDate })) as DayFile;
  store.today = dayFile;
}
</script>

<template>
  <div class="bg-white rounded-lg shadow-sm p-4 space-y-3">
    <EntryInput @submit="handleSubmit" />
    <button class="text-xs text-blue-600 hover:text-blue-800" @click="showDimensions = !showDimensions">
      {{ showDimensions ? "▾ Hide" : "▸ Show" }} Dimensions
    </button>
    <DimensionPanel
      v-if="showDimensions"
      :dimensions="store.config?.dimensions || []"
      :commitments="store.commitments"
      :values="dimValues"
      @update:values="(v) => (dimValues = v)"
    />
  </div>
</template>
