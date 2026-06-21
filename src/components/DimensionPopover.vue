<!-- src/components/DimensionPopover.vue -->
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
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
const activeIndex = ref(0);

// First dimension still missing a value. `justFilled` lets callers treat a
// key as filled before props.dimValues reflects the just-emitted select.
function firstUnfilledIndex(justFilled?: string): number {
  const idx = props.dimensions.findIndex(
    (d) => d.key !== justFilled && !props.dimValues[d.key]
  );
  return idx === -1 ? 0 : idx;
}

function listLength(): number {
  return phase.value === "dim" ? props.dimensions.length : activeValues.value.length;
}

function move(delta: number) {
  const n = listLength();
  if (!n) return;
  activeIndex.value = (activeIndex.value + delta + n) % n;
}

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

function defaultValIndex(): number {
  const cur = activeDimKey.value ? props.dimValues[activeDimKey.value] : undefined;
  const i = cur ? activeValues.value.indexOf(cur) : -1;
  return i >= 0 ? i : 0;
}

function selectDim(key: string) {
  activeDimKey.value = key;
  phase.value = "val";
  activeIndex.value = defaultValIndex();
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
    activeIndex.value = firstUnfilledIndex(justFilledKey);
  }
}

function goBack() {
  phase.value = "dim";
  activeDimKey.value = null;
  activeIndex.value = firstUnfilledIndex();
}

// Window-level capture-phase handler (spec §5.1/§5.2 + keyboard nav design):
// Esc — val→dim / dim→close. Arrows / Ctrl+N/P move the highlight. Enter selects
// the highlighted item. capture + stopPropagation makes the popover own these keys
// regardless of focus, ahead of the parent's handlers.
function onWindowKeydown(e: KeyboardEvent) {
  if (e.isComposing) return;
  if (e.key === "Escape") {
    e.preventDefault();
    e.stopPropagation();
    if (phase.value === "val") goBack();
    else emit("close");
    return;
  }
  const down = e.key === "ArrowDown" || (e.ctrlKey && (e.key === "n" || e.key === "N"));
  const up = e.key === "ArrowUp" || (e.ctrlKey && (e.key === "p" || e.key === "P"));
  if (down) { e.preventDefault(); e.stopPropagation(); move(1); return; }
  if (up) { e.preventDefault(); e.stopPropagation(); move(-1); return; }
  if (e.key === "Enter") {
    e.preventDefault();
    e.stopPropagation();
    if (phase.value === "dim") {
      const d = props.dimensions[activeIndex.value];
      if (d) selectDim(d.key);
    } else {
      const v = activeValues.value[activeIndex.value];
      if (v !== undefined) selectVal(v);
    }
    return;
  }
}
onMounted(() => {
  activeIndex.value = firstUnfilledIndex();
  window.addEventListener("keydown", onWindowKeydown, true);
});
onUnmounted(() => window.removeEventListener("keydown", onWindowKeydown, true));
</script>

<template>
  <div
    class="w-[240px] bg-[var(--color-surface)] border border-[var(--color-border-form)]
           rounded-[var(--radius-card)] shadow-[var(--shadow-popover)] overflow-hidden"
  >
    <!-- Dim phase -->
    <template v-if="phase === 'dim'">
      <div
        class="px-md py-sm text-micro font-bold uppercase tracking-wider
               text-[var(--color-popover-dim-header-text)] bg-[var(--color-popover-dim-header-bg)]
               border-b border-[var(--color-divider)] flex items-center gap-sm"
      >
        <span class="bg-[var(--color-brand-solid)] text-white px-sm py-2xs rounded-[var(--radius-sm)] text-micro">DIM</span>
        Pick a dimension
      </div>
      <div
        v-for="(d, i) in dimensions" :key="d.key"
        data-test="dim-item"
        :data-active="i === activeIndex"
        class="px-md py-sm text-secondary
               flex items-center gap-sm cursor-pointer border-b border-[var(--color-surface-muted)]
               last:border-b-0"
        :class="[
          i === activeIndex
            ? 'bg-[var(--color-popover-item-active-bg)]'
            : (dimValues[d.key] ? 'bg-[var(--color-popover-item-selected-bg)]' : ''),
          dimValues[d.key] ? 'text-[var(--color-brand-solid)] font-semibold' : 'text-[var(--color-text-primary)]',
        ]"
        @mouseenter="activeIndex = i"
        @click="selectDim(d.key)"
      >
        <span class="w-[3px] h-[18px] rounded-[var(--radius-sm)] flex-shrink-0" :class="barClass(d.key)"></span>
        {{ d.name }}
        <span
          class="ml-auto text-micro"
          :class="d.required ? 'text-[var(--color-warning)] font-medium' : 'text-[var(--color-text-disabled)]'"
        >{{ d.required ? "required" : "optional" }}</span>
      </div>
      <div
        class="px-md py-sm text-micro text-[var(--color-text-disabled)]
               border-t border-[var(--color-divider)] flex gap-md"
      >
        <span><kbd class="mono px-xs border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">↵</kbd> select</span>
        <span><kbd class="mono px-xs border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">⌃N/⌃P</kbd> move</span>
        <span><kbd class="mono px-xs border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">esc</kbd> close</span>
      </div>
    </template>

    <!-- Val phase -->
    <template v-else>
      <div
        class="px-md py-sm text-micro font-bold uppercase tracking-wider
               text-[var(--color-popover-val-header-text)] bg-[var(--color-popover-val-header-bg)]
               border-b border-[var(--color-divider)] flex items-center gap-sm"
      >
        <button data-test="back-btn" class="font-bold cursor-pointer leading-none" @click="goBack">←</button>
        {{ activeDim?.name }}
      </div>
      <div
        v-for="(v, i) in activeValues" :key="v"
        data-test="val-item"
        :data-active="i === activeIndex"
        class="px-md py-sm text-secondary
               cursor-pointer border-b border-[var(--color-surface-muted)] last:border-b-0"
        :class="[
          i === activeIndex
            ? 'bg-[var(--color-popover-item-active-bg)]'
            : (activeDimKey && dimValues[activeDimKey] === v ? 'bg-[var(--color-popover-item-selected-bg)]' : ''),
          activeDimKey && dimValues[activeDimKey] === v ? 'text-[var(--color-brand-solid)] font-semibold' : 'text-[var(--color-text-primary)]',
        ]"
        @mouseenter="activeIndex = i"
        @click="selectVal(v)"
      >{{ v }}</div>
      <div
        class="px-md py-sm text-micro text-[var(--color-text-disabled)]
               border-t border-[var(--color-divider)] flex gap-md"
      >
        <span><kbd class="mono px-xs border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">↵</kbd> select</span>
        <span><kbd class="mono px-xs border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">⌃N/⌃P</kbd> move</span>
        <span><kbd class="mono px-xs border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">esc</kbd> back to dims</span>
      </div>
    </template>
  </div>
</template>
