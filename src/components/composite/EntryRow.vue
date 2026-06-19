<!-- src/components/composite/EntryRow.vue -->
<script setup lang="ts">
import { ref, computed } from "vue";
import type { Entry } from "../../types";
import { formatDuration } from "../../utils/format";
import { useStore } from "../../stores/useStore";
import EntryRowEdit from "./EntryRowEdit.vue";

const props = defineProps<{ entry: Entry; index: number }>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
  updateDimensions: [entryId: string, dimensions: Record<string, string>];
}>();

const store = useStore();
const editing = ref(false);

const dimensions = computed(() => store.config?.dimensions || []);
const filledDims = computed(() => dimensions.value.filter(d => props.entry.dimensions[d.key]));

function chipClass(key: string): string {
  const map: Record<string, string> = {
    category: "bg-[var(--color-chip-cat-bg)] text-[var(--color-chip-cat-text)]",
    "business-line": "bg-[var(--color-chip-biz-bg)] text-[var(--color-chip-biz-text)]",
    "importance-urgency": "bg-[var(--color-chip-imp-bg)] text-[var(--color-chip-imp-text)]",
    goal: "bg-[var(--color-chip-goal-bg)] text-[var(--color-chip-goal-text)]",
  };
  return map[key] || map.category;
}

function onSave(item: string, durationMinutes: number, dims: Record<string, string>) {
  const itemChanged = item !== props.entry.item;
  const durChanged = durationMinutes !== props.entry.duration;
  const dimsChanged = JSON.stringify(dims) !== JSON.stringify(props.entry.dimensions);
  if (itemChanged || durChanged) emit("update", props.entry.id, item, durationMinutes);
  if (dimsChanged) emit("updateDimensions", props.entry.id, dims);
  editing.value = false;
}
</script>

<template>
  <EntryRowEdit
    v-if="editing"
    :entry="entry"
    :dimensions="dimensions"
    :commitments="store.commitments"
    @save="onSave"
    @cancel="editing = false"
    @delete="emit('delete', entry.id); editing = false"
  />
  <div
    v-else
    data-test="entry-row"
    class="group flex justify-between items-start gap-[8px] px-[14px] py-[9px] rounded-[var(--radius-form-lg)]
           border border-transparent hover:bg-[var(--color-surface-muted)] hover:border-[var(--color-divider)] transition-all"
    @dblclick="editing = true"
  >
    <div class="flex-1 min-w-0">
      <div
        class="text-[var(--app-text-base)] font-medium text-[var(--color-text-primary)] leading-[1.4] break-words overflow-hidden [display:-webkit-box] [-webkit-line-clamp:2] [-webkit-box-orient:vertical]"
        :title="entry.item"
      >{{ entry.item }}</div>
      <div v-if="filledDims.length" class="flex gap-[3px] mt-[3px] flex-wrap">
        <span
          v-for="d in filledDims" :key="d.key"
          class="text-[var(--app-text-micro)] font-[450] px-[6px] rounded-[var(--radius-sm)] leading-[1.7] max-w-[100px] overflow-hidden text-ellipsis whitespace-nowrap"
          :class="chipClass(d.key)"
          :title="entry.dimensions[d.key]"
        >{{ entry.dimensions[d.key] }}</span>
      </div>
    </div>
    <span class="mono text-[var(--app-text-sm)] text-[var(--color-text-primary)] flex-shrink-0 ml-[16px] pt-[1px]">
      {{ entry.duration > 0 ? formatDuration(entry.duration) : "—" }}
    </span>
    <span
      data-test="edit-trigger"
      class="text-[var(--color-text-secondary)] hover:text-[var(--color-brand-solid)] text-[14px] leading-none flex-shrink-0 ml-[8px] px-[2px] cursor-pointer opacity-0 group-hover:opacity-100 transition-opacity"
      title="Edit"
      @click="editing = true"
    >⋯</span>
  </div>
</template>
