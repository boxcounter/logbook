<!-- src/components/DimensionPopover.vue -->
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import type { Dimension, Commitment } from "../types";
import { dimensionHues, dimBar } from "../utils/dimensionColor";
import { isIMEEvent } from "../utils/ime";

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
  dimValues: Record<string, string>;
}>();

const emit = defineEmits<{
  select: [dimKey: string, value: string];
  close: [];
}>();

const stage = ref<"dim" | "val">("dim");
const selectedDimKey = ref<string | null>(null);
const highlightedIndex = ref(0);

// Hover latch: mouseenter updates the highlight only after a real mousemove.
// WebKit dispatches synthetic mouse events when new content appears under a
// stationary cursor (popover open, stage swap); without the latch that
// synthetic mouseenter steals the highlight from the keyboard position.
// Within one hit-test update mouseenter fires before mousemove, so the
// synthetic enter is ignored and the trailing mousemove re-arms the latch.
// Reset on every list swap (mount, selectDim, selectVal→dim, goBack).
let hoverArmed = false;
function armHover() {
  hoverArmed = true;
}

const visibleDims = computed(() => props.dimensions.filter(d => !d.deleted));

// First dimension still missing a value. `justFilled` lets callers treat a
// key as filled before props.dimValues reflects the just-emitted select.
function firstUnfilledIndex(justFilled?: string): number {
  const idx = visibleDims.value.findIndex(
    (d) => d.key !== justFilled && !props.dimValues[d.key]
  );
  return idx === -1 ? 0 : idx;
}

// Whether a dimension has any value the user could pick. Mirrors the value
// sources in `activeValues` (role from commitments, goals from commitments,
// static from d.values) — used only for auto-close judgement, not rendering.
function hasFillableValues(d: Dimension): boolean {
  if (d.source === "commitments:role") {
    return props.commitments.length > 0;
  }
  if (d.source === "commitments:role:goals") {
    return goalOptions.value.length > 0;
  }
  return (d.values ?? []).length > 0;
}

function listLength(): number {
  if (stage.value === "dim") {
    return visibleDims.value.length + (!hasRoleDimension.value && props.commitments.length > 0 ? 1 : 0);
  }
  return activeValues.value.length;
}

function move(delta: number) {
  const n = listLength();
  if (!n) return;
  highlightedIndex.value = (highlightedIndex.value + delta + n) % n;
}

const goalOptions = computed(() => {
  const goals = new Set<string>();
  for (const c of props.commitments) for (const g of c.goals) goals.add(g);
  return [...goals];
});

const goalKey = computed(() => {
  const monthly = props.dimensions.find(d => d.source === "commitments:role:goals");
  return monthly?.key ?? null;
});

const roleKey = computed(() => {
  const r = props.dimensions.find(d => d.source === "commitments:role");
  return r?.key ?? null;
});

const hasRoleDimension = computed(() =>
  props.dimensions.some(d => d.source === "commitments:role")
);

const activeValues = computed(() => {
  if (stage.value !== "val") return [];

  // Role dimension: values from commitments
  if (selectedDimKey.value === roleKey.value || (selectedDimKey.value === "role" && !hasRoleDimension.value)) {
    let roles = props.commitments.map(c => c.role);
    // Cross-filter: if goal is already selected, only show roles containing that goal
    const existingGoal = goalKey.value ? props.dimValues[goalKey.value] : undefined;
    if (existingGoal) {
      roles = roles.filter(r =>
        props.commitments.find(c => c.role === r)?.goals.includes(existingGoal)
      );
    }
    return roles;
  }

  // Goal dimension: values from commitments:role:goals source
  if (selectedDimKey.value === goalKey.value) {
    let goals = goalOptions.value;
    // Cross-filter: if role is already selected, only show goals under that role
    const existingRole = props.dimValues[roleKey.value ?? "role"];
    if (existingRole) {
      const roleCommitment = props.commitments.find(c => c.role === existingRole);
      if (roleCommitment) goals = roleCommitment.goals;
    }
    return goals;
  }

  // Other dimensions: values from template
  const d = props.dimensions.find(d => d.key === selectedDimKey.value);
  if (!d) return [];
  if (d.source === "commitments:role:goals") return goalOptions.value;
  return d.values ?? [];
});

const valHeaderName = computed(() => {
  if (stage.value !== "val") return "";
  if (selectedDimKey.value === roleKey.value || (selectedDimKey.value === "role" && !hasRoleDimension.value)) {
    const d = visibleDims.value.find(d => d.key === roleKey.value);
    return d?.name ?? "Role";
  }
  return visibleDims.value.find(d => d.key === selectedDimKey.value)?.name ?? "";
});

const hues = computed(() => dimensionHues(props.dimensions));
function barColor(key: string): string {
  return dimBar(hues.value.get(key) ?? null);
}

function defaultValIndex(): number {
  const cur = selectedDimKey.value ? props.dimValues[selectedDimKey.value] : undefined;
  const i = cur ? activeValues.value.indexOf(cur) : -1;
  return i >= 0 ? i : 0;
}

function selectDim(key: string) {
  const d = props.dimensions.find(d => d.key === key);
  if (d && d.deleted) return;
  selectedDimKey.value = key;
  stage.value = "val";
  highlightedIndex.value = defaultValIndex();
  hoverArmed = false; // list swapped under a possibly-stationary cursor
}

function selectVal(value: string) {
  if (!selectedDimKey.value) return;
  const justFilledKey = selectedDimKey.value;
  emit("select", justFilledKey, value);
  const allFillableFilled = visibleDims.value
    .filter(d => d.key !== justFilledKey && !props.dimValues[d.key])
    .every(d => !hasFillableValues(d));
  if (allFillableFilled) {
    emit("close");
  } else {
    stage.value = "dim";
    selectedDimKey.value = null;
    highlightedIndex.value = firstUnfilledIndex(justFilledKey);
    hoverArmed = false; // list swapped under a possibly-stationary cursor
  }
}

function goBack() {
  stage.value = "dim";
  selectedDimKey.value = null;
  highlightedIndex.value = firstUnfilledIndex();
  hoverArmed = false; // list swapped under a possibly-stationary cursor
}

// Window-level capture-phase handler (spec §5.1/§5.2 + keyboard nav design):
// Esc — val→dim / dim→close. Arrows / Ctrl+N/P move the highlight. Enter selects
// the highlighted item. capture + stopPropagation makes the popover own these keys
// regardless of focus, ahead of the parent's handlers.
//
// IME guarding is per key type: navigation keys check isComposing only — during
// an active composition the IME owns them (candidate paging), but the keyCode-229
// guard must NOT apply to them, because WebKit marks IME pass-through events 229
// with a CJK input source active even with no composition, which would eat the
// key. Enter/Esc keep the full isIMEEvent guard: 229 is the only reliable WebKit
// signal for candidate confirm/cancel.
function onWindowKeydown(e: KeyboardEvent) {
  const down = e.key === "ArrowDown" || (e.ctrlKey && (e.key === "n" || e.key === "N"));
  const up = e.key === "ArrowUp" || (e.ctrlKey && (e.key === "p" || e.key === "P"));
  if (down || up) {
    if (e.isComposing) return;
    e.preventDefault();
    e.stopPropagation();
    move(down ? 1 : -1);
    return;
  }
  if (isIMEEvent(e)) return;
  if (e.key === "Escape") {
    e.preventDefault();
    e.stopPropagation();
    if (stage.value === "val") goBack();
    else emit("close");
    return;
  }
  if (e.key === "Enter") {
    e.preventDefault();
    e.stopPropagation();
    if (stage.value === "dim") {
      if (highlightedIndex.value < visibleDims.value.length) {
        const d = visibleDims.value[highlightedIndex.value];
        if (d) selectDim(d.key);
      } else if (!hasRoleDimension.value && props.commitments.length > 0) {
        selectDim("role");
      }
    } else {
      const v = activeValues.value[highlightedIndex.value];
      if (v !== undefined) selectVal(v);
    }
    return;
  }
}
onMounted(() => {
  highlightedIndex.value = firstUnfilledIndex();
  window.addEventListener("keydown", onWindowKeydown, true);
  window.addEventListener("mousemove", armHover);
});
onUnmounted(() => {
  window.removeEventListener("keydown", onWindowKeydown, true);
  window.removeEventListener("mousemove", armHover);
});
</script>

<template>
  <div
    class="w-[240px] bg-[var(--color-surface)] border border-[var(--color-border-form)]
           rounded-[var(--radius-card)] shadow-[var(--shadow-popover)] overflow-hidden"
  >
    <!-- Dim stage -->
    <template v-if="stage === 'dim'">
      <div
        class="px-md py-sm text-micro font-bold uppercase tracking-wider
               text-[var(--color-popover-dim-header-text)] bg-[var(--color-popover-dim-header-bg)]
               border-b border-[var(--color-divider)] flex items-center gap-sm"
      >
        <span class="bg-[var(--color-brand-solid)] text-white px-sm py-2xs rounded-[var(--radius-sm)] text-micro">DIM</span>
        Pick a dimension
      </div>
      <template v-for="(d, i) in visibleDims" :key="d.key">
      <div
        v-if="!d.deleted"
        data-test="dim-item"
        :data-active="i === highlightedIndex"
        class="px-md py-sm text-secondary
               flex items-center gap-sm cursor-pointer border-b border-[var(--color-surface-muted)]
               last:border-b-0"
        :class="
          i === highlightedIndex
            ? 'bg-[var(--color-brand-solid)] text-white'
            : (dimValues[d.key]
                ? 'text-[var(--color-brand-solid)] font-semibold'
                : 'text-[var(--color-text-primary)]')
        "
        @mouseenter="hoverArmed && (highlightedIndex = i)"
        @click="selectDim(d.key)"
      >
        <span class="w-[3px] h-[18px] rounded-[var(--radius-sm)] flex-shrink-0" :style="{ background: barColor(d.key) }"></span>
        {{ d.name }}
        <span
          v-if="dimValues[d.key]"
          class="ml-auto flex items-center gap-xs text-micro max-w-[110px]"
        >
          <span class="truncate">{{ dimValues[d.key] }}</span>
          <span class="flex-shrink-0">✓</span>
        </span>
        <span
          v-else
          class="ml-auto text-micro"
          :class="i === highlightedIndex
            ? 'text-white'
            : (d.required ? 'text-[var(--color-warning)] font-medium' : 'text-[var(--color-text-disabled)]')"
        >{{ d.required ? "required" : "optional" }}</span>
      </div>
      </template>
      <div
        v-if="!hasRoleDimension && commitments.length > 0"
        data-test="dim-role"
        :data-active="visibleDims.length === highlightedIndex"
        class="px-md py-sm text-secondary
               flex items-center gap-sm cursor-pointer border-b border-[var(--color-surface-muted)]"
        :class="
          visibleDims.length === highlightedIndex
            ? 'bg-[var(--color-brand-solid)] text-white'
            : (dimValues['role']
                ? 'text-[var(--color-brand-solid)] font-semibold'
                : 'text-[var(--color-text-primary)]')
        "
        @mouseenter="hoverArmed && (highlightedIndex = visibleDims.length)"
        @click="selectDim('role')"
      >
        <span class="w-[3px] h-[18px] rounded-[var(--radius-sm)] flex-shrink-0" :style="{ background: barColor('role') }"></span>
        Role
        <span
          v-if="dimValues['role']"
          class="ml-auto flex items-center gap-xs text-micro max-w-[110px]"
        >
          <span class="truncate">{{ dimValues['role'] }}</span>
          <span class="flex-shrink-0">✓</span>
        </span>
        <span
          v-else
          class="ml-auto text-micro"
          :class="visibleDims.length === highlightedIndex
            ? 'text-white'
            : 'text-[var(--color-text-disabled)]'"
        >optional</span>
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

    <!-- Val stage -->
    <template v-else>
      <div
        class="px-md py-sm text-micro font-bold uppercase tracking-wider
               text-[var(--color-popover-val-header-text)] bg-[var(--color-popover-val-header-bg)]
               border-b border-[var(--color-divider)] flex items-center gap-sm"
      >
        <button data-test="back-btn" class="font-bold cursor-pointer leading-none" @click="goBack">←</button>
        {{ valHeaderName }}
      </div>
      <div
        v-for="(v, i) in activeValues" :key="v"
        data-test="val-item"
        :data-active="i === highlightedIndex"
        class="px-md py-sm text-secondary
               flex items-center cursor-pointer border-b border-[var(--color-surface-muted)] last:border-b-0"
        :class="
          i === highlightedIndex
            ? 'bg-[var(--color-brand-solid)] text-white'
            : (selectedDimKey && dimValues[selectedDimKey] === v
                ? 'text-[var(--color-brand-solid)] font-semibold'
                : 'text-[var(--color-text-primary)]')
        "
        @mouseenter="hoverArmed && (highlightedIndex = i)"
        @click="selectVal(v)"
      >
        <span class="truncate min-w-0">{{ v }}</span>
        <span v-if="selectedDimKey && dimValues[selectedDimKey] === v" class="ml-auto flex-shrink-0">✓</span>
      </div>
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
