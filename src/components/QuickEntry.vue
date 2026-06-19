<script setup lang="ts">
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../stores/useStore";
import EntryInput from "./EntryInput.vue";
import DimensionPanel from "./DimensionPanel.vue";
import AppButton from "./base/AppButton.vue";
import { logError, logInfo } from "../utils/errorLog";

const emit = defineEmits<{
  appended: [];
}>();

const store = useStore();
const showDimensions = ref(false);
const dimValues = ref<Record<string, string>>({});
const entryInputRef = ref<InstanceType<typeof EntryInput> | null>(null);

watch(
  () => store.lastDimensions,
  (ld) => {
    if (Object.keys(ld).length > 0) dimValues.value = { ...ld };
  },
  { immediate: true }
);

// Sync dimValues between EntryInput (chips) and DimensionPanel
function sanitizeValues(vals: Record<string, string>): Record<string, string> {
  const validKeys = new Set((store.config?.dimensions || []).map((d) => d.key));
  const cleaned: Record<string, string> = {};
  for (const [k, v] of Object.entries(vals)) {
    if (validKeys.has(k) && v) cleaned[k] = v;
  }
  return cleaned;
}

async function handleSubmit(item: string, durationMinutes: number, dimensions: Record<string, string>) {
  const finalDimensions = sanitizeValues(dimensions);
  const newEntry = { item, duration: String(durationMinutes), dimensions: finalDimensions };
  logInfo("QuickEntry.handleSubmit", `invoking append_entry date=${store.currentDate} item="${item}" dur=${durationMinutes}`);

  try {
    const result = await invoke("append_entry", { rootPath: store.rootPath, date: store.currentDate, entry: newEntry });
    logInfo("QuickEntry.handleSubmit", `append_entry OK id=${(result as any).id}`);
    store.lastDimensions = { ...finalDimensions };
    dimValues.value = { ...finalDimensions };
    entryInputRef.value?.clearInput();
    // Optimistic: append returned entry to store.today
    if (store.today) {
      store.today = { ...store.today, entries: [...store.today.entries, result as any] };
    }
    emit("appended");
  } catch (e) {
    logError("QuickEntry.handleSubmit", e);
  }
}
</script>

<template>
  <div class="bg-[var(--color-surface)] rounded-[var(--radius-card)] shadow-[var(--shadow-card)] p-4 space-y-3">
    <EntryInput
      ref="entryInputRef"
      :dimensions="store.config?.dimensions || []"
      :commitments="store.commitments"
      :initialValues="dimValues"
      @submit="handleSubmit"
    />
    <AppButton variant="outline" size="sm" @click="showDimensions = !showDimensions">
      {{ showDimensions ? "Hide" : "Show" }} Dimensions
    </AppButton>
    <DimensionPanel
      v-if="showDimensions"
      :dimensions="store.config?.dimensions || []"
      :commitments="store.commitments"
      :values="dimValues"
      @update:values="(v) => (dimValues = v)"
    />
  </div>
</template>
