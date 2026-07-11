<!-- src/components/composite/EntryRowEdit.vue -->
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import type { Entry, Dimension, Commitment } from "../../types";
import { resolveDelta } from "../../utils/format";
import DimensionPopover from "../DimensionPopover.vue";
import { dimensionHues, dimTokenChipStyle } from "../../utils/dimensionColor";
import { useClickOutside } from "../../composables/useClickOutside";

const props = defineProps<{
  entry: Entry;
  dimensions: Dimension[];
  commitments: Commitment[];
  focusTarget?: 'item' | 'duration';
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
const submitAttempted = ref(false);
const rootEl = ref<HTMLElement | null>(null);
const itemInputEl = ref<HTMLInputElement>();
const durInputEl = ref<HTMLInputElement>();
const popoverUp = ref(false);
const confirming = ref(false);

const isDirty = computed(() =>
  item.value !== props.entry.item ||
  resolveDelta(durText.value, props.entry.duration) !== props.entry.duration ||
  JSON.stringify(dimValues.value) !== JSON.stringify(props.entry.dimensions)
);

function onEsc() {
  if (popoverOpen.value) return;            // popover owns esc
  if (confirming.value || !isDirty.value) { // 2nd esc, or nothing to lose
    emit("cancel");
    return;
  }
  confirming.value = true;                  // dirty: ask before discarding
}

// Enter normally saves; while confirming it means "keep editing".
// Guard against IME composition (e.g. Chinese pinyin candidate selection).
function onEnter(e: KeyboardEvent) {
  if (e.isComposing) return;
  e.preventDefault();
  if (confirming.value) { confirming.value = false; return; }
  save();
}

// Auto-dismiss when the editor is no longer the active focus: a click outside,
// or focus moving to another element (e.g. the entry input claiming focus on a
// window refocus). Same policy as esc — clean exit, but a dirty edit shows the
// discard confirm bar instead of silently dropping changes.
function dismissFromOutside() {
  if (popoverOpen.value) { popoverOpen.value = false; return; }
  if (confirming.value || !isDirty.value) { emit("cancel"); return; }
  confirming.value = true;
}

useClickOutside(rootEl, ref(true), {
  beforeClose: () => {
    dismissFromOutside();
    return false;
  },
});
function onDocFocusin(e: FocusEvent) {
  if (rootEl.value && !rootEl.value.contains(e.target as Node)) dismissFromOutside();
}
// Esc only reaches the root's @keydown.esc when focus is inside the row. When
// focus is elsewhere (the entry input, body), handle esc at the document level
// so it still dismisses. The root handler uses .stop, so the two never overlap.
function onDocKeydown(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  if (popoverOpen.value) return; // popover owns esc
  if (rootEl.value?.contains(document.activeElement)) return; // focus inside → root handler owns it
  dismissFromOutside();
}
onMounted(async () => {
  document.addEventListener("focusin", onDocFocusin, true);
  document.addEventListener("keydown", onDocKeydown);

  await nextTick();
  const target = props.focusTarget === 'duration' ? durInputEl.value : itemInputEl.value;
  target?.focus();
  if (target) {
    target.setSelectionRange(target.value.length, target.value.length);
  }
});
onUnmounted(() => {
  document.removeEventListener("focusin", onDocFocusin, true);
  document.removeEventListener("keydown", onDocKeydown);
});

// Open upward when there isn't room below (the card clips with overflow-hidden).
function openPopover() {
  const rect = rootEl.value?.getBoundingClientRect();
  popoverUp.value = rect ? window.innerHeight - rect.bottom < 260 : false;
  popoverOpen.value = true;
}

const missingRequired = computed(() =>
  props.dimensions.filter(d => !d.deleted && d.required && !dimValues.value[d.key])
);

const unfilledOptional = computed(() =>
  props.dimensions.filter(d => !d.deleted && !d.required && !dimValues.value[d.key])
);

const knownDimKeys = computed(() => new Set(props.dimensions.map(d => d.key)));

function filled() {
  const known = props.dimensions.filter(d => !d.deleted && dimValues.value[d.key]);
  const extra = Object.keys(dimValues.value)
    .filter(k => dimValues.value[k] && !knownDimKeys.value.has(k))
    .map(k => ({
      key: k,
      name: k === 'role' ? 'Role' : k,
      required: false,
      source: 'static' as const,
      deleted: false,
    } as Dimension));
  return [...known, ...extra];
}

const editHues = computed(() => dimensionHues(props.dimensions));
function tokenChipStyle(key: string) {
  return dimTokenChipStyle(editHues.value.get(key) ?? null);
}

function removeDim(key: string) {
  confirming.value = false;
  const next = { ...dimValues.value };
  delete next[key];
  dimValues.value = next;
}

function onSelect(dimKey: string, value: string) {
  confirming.value = false;
  dimValues.value = { ...dimValues.value, [dimKey]: value };
}

function save() {
  // Hard-block: the backend's update_entry rejects entries missing a required
  // dimension, so block here and flag it instead of failing silently.
  if (missingRequired.value.length > 0) {
    submitAttempted.value = true;
    return;
  }
  const minutes = resolveDelta(durText.value, props.entry.duration);
  emit("save", item.value.trim() || "(untitled)", minutes, { ...dimValues.value });
}
</script>

<template>
  <div
    ref="rootEl"
    class="bg-[var(--color-surface)] border border-[var(--color-brand-solid)] rounded-[var(--radius-form-lg)]
           shadow-[var(--shadow-focus-ring)] px-md py-sm flex flex-col gap-xs relative"
    @keydown.esc.stop="onEsc"
  >
    <div class="flex gap-sm items-center">
      <input
        ref="itemInputEl"
        v-model="item"
        class="flex-1 text-body font-medium text-[var(--color-text-primary)] border-none outline-none bg-transparent py-2xs"
        @keydown.enter="onEnter"
        @input="confirming = false"
      />
      <input
        ref="durInputEl"
        v-model="durText"
        class="mono w-[56px] text-right text-secondary text-[var(--color-text-primary)]
               border border-[var(--color-border-form)] rounded-[var(--radius-form)] px-sm py-2xs
               outline-none focus:border-[var(--color-brand-solid)]"
        @keydown.enter="onEnter"
        @input="confirming = false"
      />
      <span class="text-secondary text-[var(--color-text-secondary)]">min</span>
    </div>

    <div class="flex gap-xs flex-wrap mt-2xs items-center">
      <span
        v-for="d in filled()" :key="d.key"
        class="text-micro font-medium px-sm py-2xs rounded-[var(--radius-sm)] inline-flex items-center gap-xs"
        :style="tokenChipStyle(d.key)"
      >
        {{ dimValues[d.key] }}
        <span data-test="chip-remove" class="cursor-pointer opacity-50 hover:opacity-100 text-secondary leading-none" @click="removeDim(d.key)">×</span>
      </span>
      <span
        v-for="m in missingRequired" :key="'missing-' + m.key"
        data-test="missing-required"
        class="text-micro font-[450] px-sm py-2xs rounded-[var(--radius-sm)]
               border-[1.5px] border-dashed inline-flex items-center gap-xs cursor-pointer transition-colors"
        :class="submitAttempted
          ? 'border-[var(--color-warning)] text-[var(--color-warning)]'
          : 'border-[var(--color-missing-border)] text-[var(--color-missing-text)] hover:border-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)]'"
        @click="openPopover"
      >
        <span class="w-[5px] h-[5px] rounded-full bg-[var(--color-missing-dot)]"></span>{{ m.name }}
      </span>
      <span
        v-if="unfilledOptional.length > 0"
        data-test="add-dimension"
        class="text-micro font-medium px-sm py-2xs rounded-[var(--radius-sm)]
               border border-dashed border-[var(--color-border-form)] text-[var(--color-text-secondary)]
               cursor-pointer hover:border-[var(--color-text-muted)]"
        @click="openPopover"
      >+</span>
    </div>

    <div class="flex gap-sm mt-xs items-center">
      <template v-if="!confirming">
        <button data-test="save" class="text-micro font-semibold px-sm py-2xs rounded-[var(--radius-form)] bg-[var(--color-brand-solid)] text-white hover:bg-[var(--color-brand-link)]" @click="save">Save</button>
        <button data-test="cancel" class="text-micro font-semibold px-sm py-2xs rounded-[var(--radius-form)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]" @click="emit('cancel')">Cancel</button>
        <button data-test="delete" class="text-micro font-semibold px-sm py-2xs rounded-[var(--radius-form)] text-[var(--color-text-disabled)] hover:text-[var(--color-danger)] ml-auto" @click="emit('delete')">Delete</button>
      </template>
      <template v-else>
        <span data-test="discard-prompt" class="text-micro text-[var(--color-text-secondary)]">Discard changes?</span>
        <button data-test="discard" class="text-micro font-semibold px-sm py-2xs rounded-[var(--radius-form)] text-[var(--color-danger)] hover:underline" @click="emit('cancel')">Discard</button>
        <button data-test="keep-editing" class="text-micro font-semibold px-sm py-2xs rounded-[var(--radius-form)] bg-[var(--color-brand-solid)] text-white hover:bg-[var(--color-brand-link)]" @click="confirming = false">Keep editing</button>
      </template>
    </div>

    <DimensionPopover
      v-if="popoverOpen"
      :dimensions="dimensions"
      :commitments="commitments"
      :dim-values="dimValues"
      class="absolute left-0 z-10"
      :class="popoverUp ? 'bottom-full mb-xs' : 'top-full mt-xs'"
      @select="onSelect"
      @close="popoverOpen = false"
    />
  </div>
</template>
