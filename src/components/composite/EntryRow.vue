<!-- src/components/composite/EntryRow.vue -->
<script setup lang="ts">
import { ref, computed } from "vue";
import type { Entry } from "../../types";
import { formatDuration } from "../../utils/format";
import { dimensionHues, dimChipStyle } from "../../utils/dimensionColor";
import { useStore } from "../../stores/useStore";
import EntryRowEdit from "./EntryRowEdit.vue";

const props = defineProps<{ entry: Entry; index: number; justAdded?: boolean }>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
  updateDimensions: [entryId: string, dimensions: Record<string, string>];
}>();

const store = useStore();
const editing = ref(false);
const focusTarget = ref<'item' | 'duration'>('item');

function onDblClick(e: MouseEvent) {
  const target = (e.target as HTMLElement).closest('[data-edit-target]') as HTMLElement | null;
  focusTarget.value = (target?.dataset.editTarget as 'item' | 'duration') || 'item';
  editing.value = true;
}

function onEditTrigger() {
  focusTarget.value = 'item';
  editing.value = true;
}

const dimensions = computed(() => store.dimensions);
const filledDims = computed(() => dimensions.value.filter(d => props.entry.dimensions[d.key]));

const isProblemEntry = computed(() =>
  props.entry.attribution === "unattributed" || props.entry.attribution === "mismatch"
);

const chipHues = computed(() => dimensionHues(dimensions.value));
function chipStyle(key: string) {
  return dimChipStyle(chipHues.value.get(key) ?? null);
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
    :focus-target="focusTarget"
    @save="onSave"
    @cancel="editing = false"
    @delete="emit('delete', entry.id); editing = false"
  />
  <div
    v-else
    data-test="entry-row"
    class="group flex justify-between items-start gap-sm px-md py-sm transition-colors"
    :class="[
      { 'just-added': justAdded },
      isProblemEntry
        ? 'bg-[#fffbeb] hover:bg-[#fef3c7]'
        : 'hover:bg-[var(--color-surface-muted)]',
      index > 0 ? 'border-t border-[var(--color-divider)]' : '',
      isProblemEntry && index > 0 ? '!border-[#fde68a]' : '',
    ]"
    @dblclick="onDblClick"
  >
    <span
      v-if="isProblemEntry"
      class="flex-shrink-0 text-[#d97706]"
      style="font-size: 14px; width: 16px; text-align: center;"
      title="未归属任何 role 或 role/goal 不匹配"
    >●</span>
    <div class="flex-1 min-w-0" data-edit-target="item">
      <div
        data-test="item-display"
        class="text-body font-medium text-[var(--color-text-primary)] break-words overflow-hidden [display:-webkit-box] [-webkit-line-clamp:2] [-webkit-box-orient:vertical]"
        :title="entry.item"
      >{{ entry.item }}</div>
      <div v-if="filledDims.length" class="flex gap-xs mt-xs flex-wrap">
        <span
          v-for="d in filledDims" :key="d.key"
          class="text-micro font-[450] px-sm rounded-[var(--radius-sm)] max-w-[100px] overflow-hidden text-ellipsis whitespace-nowrap"
          :style="chipStyle(d.key)"
          :title="entry.dimensions[d.key]"
        >{{ entry.dimensions[d.key] }}</span>
      </div>
    </div>
    <span
      data-test="duration-display"
      data-edit-target="duration"
      class="mono text-secondary flex-shrink-0 ml-lg pt-2xs"
      :class="isProblemEntry ? '!text-[#d97706] font-medium' : 'text-[var(--color-text-primary)]'"
    >
      {{ entry.duration > 0 ? formatDuration(entry.duration) : "—" }}
    </span>
    <span
      data-test="edit-trigger"
      class="text-[var(--color-text-secondary)] hover:text-[var(--color-brand-solid)] text-body leading-none flex-shrink-0 ml-sm px-2xs cursor-pointer opacity-0 group-hover:opacity-100 transition-opacity"
      title="Edit"
      @click="onEditTrigger"
    >⋯</span>
  </div>
</template>

<style scoped>
/* Newly-added entry: blue background that fades over 1.5s (spec §5.2 step 7). */
@keyframes fadeHighlight {
  0% { background-color: var(--anim-highlight-bg); }
  100% { background-color: transparent; }
}
.just-added { animation: fadeHighlight var(--anim-highlight-duration) ease-out forwards; }
</style>
