<!-- src/components/DimensionPopover.vue -->
<script setup lang="ts">
import { ref, computed } from "vue";
import type { Dimension, Commitment } from "../types";

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
  dimValues: Record<string, string>;
}>();

const emit = defineEmits<{
  select: [dimKey: string, value: string];
  close: [];
}>();

const phase = ref<"dim" | "val">("dim");
const activeDimKey = ref<string | null>(null);

const goalOptions = computed(() => {
  const goals = new Set<string>();
  for (const c of props.commitments) for (const g of c.goals) goals.add(g);
  return [...goals];
});

const activeDim = computed(() => props.dimensions.find(d => d.key === activeDimKey.value) || null);

const activeValues = computed(() => {
  const d = activeDim.value;
  if (!d) return [];
  return d.source === "monthly" ? goalOptions.value : (d.values || []);
});

// Map a dimension key to its left-bar token class.
function barClass(key: string): string {
  const map: Record<string, string> = {
    category: "bg-[var(--dim-bar-cat)]",
    "business-line": "bg-[var(--dim-bar-biz)]",
    "importance-urgency": "bg-[var(--dim-bar-imp)]",
    goal: "bg-[var(--dim-bar-goal)]",
  };
  return map[key] || "bg-[var(--dim-bar-cat)]";
}

function selectDim(key: string) {
  activeDimKey.value = key;
  phase.value = "val";
}

function selectVal(value: string) {
  if (!activeDimKey.value) return;
  const justFilledKey = activeDimKey.value;
  emit("select", justFilledKey, value);
  const allFilled = props.dimensions
    .filter(d => d.required)
    .every(d => d.key === justFilledKey || props.dimValues[d.key]);
  if (allFilled) {
    emit("close");
  } else {
    phase.value = "dim";
    activeDimKey.value = null;
  }
}

function goBack() {
  phase.value = "dim";
  activeDimKey.value = null;
}
</script>

<template>
  <div
    class="w-[240px] bg-[var(--color-surface)] border border-[var(--color-border-form)]
           rounded-[var(--radius-card)] shadow-[var(--shadow-popover)] overflow-hidden"
  >
    <!-- Dim phase -->
    <template v-if="phase === 'dim'">
      <div
        class="px-[14px] py-[8px] text-[var(--app-text-micro)] font-bold uppercase tracking-wider
               text-[var(--color-popover-dim-header-text)] bg-[var(--color-popover-dim-header-bg)]
               border-b border-[var(--color-divider)] flex items-center gap-[8px]"
      >
        <span class="bg-[var(--color-brand-solid)] text-white px-[6px] py-[1px] rounded-[var(--radius-sm)] text-[var(--app-text-2xs)]">DIM</span>
        Pick a dimension
      </div>
      <div
        v-for="d in dimensions" :key="d.key"
        data-test="dim-item"
        class="px-[14px] py-[9px] text-[var(--app-text-sm)]
               flex items-center gap-[10px] cursor-pointer border-b border-[var(--color-surface-muted)]
               last:border-b-0 hover:bg-[var(--color-divider)]"
        :class="dimValues[d.key] ? 'bg-[var(--color-popover-item-selected-bg)] text-[var(--color-brand-solid)] font-semibold' : 'text-[var(--color-text-primary)]'"
        @click="selectDim(d.key)"
      >
        <span class="w-[3px] h-[18px] rounded-[var(--radius-sm)] flex-shrink-0" :class="barClass(d.key)"></span>
        {{ d.name }}
        <span
          class="ml-auto text-[var(--app-text-micro)]"
          :class="d.required ? 'text-[var(--color-warning)] font-medium' : 'text-[var(--color-text-disabled)]'"
        >{{ d.required ? "required" : "optional" }}</span>
      </div>
      <div
        class="px-[14px] py-[6px] text-[var(--app-text-2xs)] text-[var(--color-text-disabled)]
               border-t border-[var(--color-divider)] flex gap-[12px]"
      >
        <span><kbd class="mono px-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">↵</kbd> select</span>
        <span><kbd class="mono px-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">esc</kbd> close</span>
      </div>
    </template>

    <!-- Val phase -->
    <template v-else>
      <div
        class="px-[14px] py-[8px] text-[var(--app-text-micro)] font-bold uppercase tracking-wider
               text-[var(--color-popover-val-header-text)] bg-[var(--color-popover-val-header-bg)]
               border-b border-[var(--color-divider)] flex items-center gap-[8px]"
      >
        <button data-test="back-btn" class="font-bold cursor-pointer leading-none" @click="goBack">←</button>
        {{ activeDim?.name }}
      </div>
      <div
        v-for="v in activeValues" :key="v"
        data-test="val-item"
        class="px-[14px] py-[9px] text-[var(--app-text-sm)]
               cursor-pointer border-b border-[var(--color-surface-muted)] last:border-b-0
               hover:bg-[var(--color-divider)]"
        :class="activeDimKey && dimValues[activeDimKey] === v ? 'bg-[var(--color-popover-item-selected-bg)] text-[var(--color-brand-solid)] font-semibold' : 'text-[var(--color-text-primary)]'"
        @click="selectVal(v)"
      >{{ v }}</div>
      <div
        class="px-[14px] py-[6px] text-[var(--app-text-2xs)] text-[var(--color-text-disabled)]
               border-t border-[var(--color-divider)] flex gap-[12px]"
      >
        <span><kbd class="mono px-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">↵</kbd> select</span>
        <span><kbd class="mono px-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">esc</kbd> back to dims</span>
      </div>
    </template>
  </div>
</template>
