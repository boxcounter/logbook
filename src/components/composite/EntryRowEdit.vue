<!-- src/components/composite/EntryRowEdit.vue -->
<script setup lang="ts">
import { ref } from "vue";
import type { Entry, Dimension, Commitment } from "../../types";
import { resolveDelta } from "../../utils/format";
import DimensionPopover from "../DimensionPopover.vue";

const props = defineProps<{
  entry: Entry;
  dimensions: Dimension[];
  commitments: Commitment[];
}>();

const emit = defineEmits<{
  save: [item: string, durationMinutes: number, dimensions: Record<string, string>];
  cancel: [];
  delete: [];
}>();

const item = ref(props.entry.item);
const durText = ref(String(props.entry.duration));
const dimValues = ref<Record<string, string>>({ ...props.entry.dimensions });
const popoverOpen = ref(false);

function chipClass(key: string): string {
  const map: Record<string, string> = {
    category: "bg-[var(--color-token-cat-bg)] text-[var(--color-token-cat-text)]",
    "business-line": "bg-[var(--color-token-biz-bg)] text-[var(--color-token-biz-text)]",
    "importance-urgency": "bg-[var(--color-token-imp-bg)] text-[var(--color-token-imp-text)]",
    goal: "bg-[var(--color-token-goal-bg)] text-[var(--color-token-goal-text)]",
  };
  return map[key] || map.category;
}

function filled() {
  return props.dimensions.filter(d => dimValues.value[d.key]);
}

function removeDim(key: string) {
  const next = { ...dimValues.value };
  delete next[key];
  dimValues.value = next;
}

function onSelect(dimKey: string, value: string) {
  dimValues.value = { ...dimValues.value, [dimKey]: value };
}

function save() {
  const minutes = resolveDelta(durText.value, props.entry.duration);
  emit("save", item.value.trim() || "(untitled)", minutes, { ...dimValues.value });
}
</script>

<template>
  <div
    class="bg-[var(--color-surface)] border border-[var(--color-brand-solid)] rounded-[var(--radius-form-lg)]
           shadow-[var(--shadow-focus-ring)] px-[14px] py-[9px] flex flex-col gap-[4px] relative"
  >
    <div class="flex gap-[8px] items-center">
      <input
        v-model="item"
        class="flex-1 text-[length:var(--app-text-base)] font-medium text-[var(--color-text-primary)] border-none outline-none bg-transparent py-[1px]"
        @keydown.enter.prevent="save"
      />
      <input
        v-model="durText"
        class="mono w-[56px] text-right text-[length:var(--app-text-sm)] text-[var(--color-text-primary)]
               border border-[var(--color-border-form)] rounded-[var(--radius-form)] px-[8px] py-[2px]
               outline-none focus:border-[var(--color-brand-solid)]"
        @keydown.enter.prevent="save"
      />
      <span class="text-[length:var(--app-text-xs-alt)] text-[var(--color-text-secondary)]">min</span>
    </div>

    <div class="flex gap-[3px] flex-wrap mt-[2px] items-center">
      <span
        v-for="d in filled()" :key="d.key"
        class="text-[length:var(--app-text-micro)] font-medium px-[7px] py-[1px] rounded-[var(--radius-sm)] inline-flex items-center gap-[5px]"
        :class="chipClass(d.key)"
      >
        {{ dimValues[d.key] }}
        <span data-test="chip-remove" class="cursor-pointer opacity-50 hover:opacity-100 text-[length:var(--app-text-xs-alt)] leading-none" @click="removeDim(d.key)">×</span>
      </span>
      <span
        data-test="add-tag"
        class="text-[length:var(--app-text-micro)] font-medium px-[7px] py-[1px] rounded-[var(--radius-sm)]
               border border-dashed border-[var(--color-border-form)] text-[var(--color-text-secondary)]
               cursor-pointer hover:border-[var(--color-text-muted)]"
        @click="popoverOpen = true"
      >+ tag</span>
    </div>

    <div class="flex gap-[8px] mt-[4px] items-center">
      <button data-test="save" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] bg-[var(--color-brand-solid)] text-white hover:bg-[var(--color-brand-link)]" @click="save">Save</button>
      <button data-test="cancel" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]" @click="emit('cancel')">Cancel</button>
      <button data-test="delete" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] text-[var(--color-text-disabled)] hover:text-[var(--color-danger)] ml-auto" @click="emit('delete')">Delete</button>
    </div>

    <DimensionPopover
      v-if="popoverOpen"
      :dimensions="dimensions"
      :commitments="commitments"
      :dim-values="dimValues"
      class="absolute left-0 top-full mt-[4px] z-10"
      @select="onSelect"
      @close="popoverOpen = false"
    />
  </div>
</template>
