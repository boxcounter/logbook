<!-- src/components/composite/EntryRowEdit.vue -->
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
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
const submitAttempted = ref(false);
const rootEl = ref<HTMLElement>();
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
function onEnter() {
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
function onDocMousedown(e: MouseEvent) {
  if (rootEl.value && !rootEl.value.contains(e.target as Node)) dismissFromOutside();
}
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
onMounted(() => {
  document.addEventListener("mousedown", onDocMousedown, true);
  document.addEventListener("focusin", onDocFocusin, true);
  document.addEventListener("keydown", onDocKeydown);
});
onUnmounted(() => {
  document.removeEventListener("mousedown", onDocMousedown, true);
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
  props.dimensions.filter(d => d.required && !dimValues.value[d.key])
);

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
           shadow-[var(--shadow-focus-ring)] px-[14px] py-[9px] flex flex-col gap-[4px] relative"
    @keydown.esc.stop="onEsc"
  >
    <div class="flex gap-[8px] items-center">
      <input
        v-model="item"
        class="flex-1 text-[length:var(--app-text-base)] font-medium text-[var(--color-text-primary)] border-none outline-none bg-transparent py-[1px]"
        @keydown.enter.prevent="onEnter"
        @input="confirming = false"
      />
      <input
        v-model="durText"
        class="mono w-[56px] text-right text-[length:var(--app-text-sm)] text-[var(--color-text-primary)]
               border border-[var(--color-border-form)] rounded-[var(--radius-form)] px-[8px] py-[2px]
               outline-none focus:border-[var(--color-brand-solid)]"
        @keydown.enter.prevent="onEnter"
        @input="confirming = false"
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
        @click="openPopover"
      >+ tag</span>
      <span
        v-if="submitAttempted && missingRequired.length"
        data-test="required-hint"
        class="text-[length:var(--app-text-micro)] text-[var(--color-warning)]"
      >Required: {{ missingRequired.map(d => d.name).join(", ") }}</span>
    </div>

    <div class="flex gap-[8px] mt-[4px] items-center">
      <template v-if="!confirming">
        <button data-test="save" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] bg-[var(--color-brand-solid)] text-white hover:bg-[var(--color-brand-link)]" @click="save">Save</button>
        <button data-test="cancel" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]" @click="emit('cancel')">Cancel</button>
        <button data-test="delete" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] text-[var(--color-text-disabled)] hover:text-[var(--color-danger)] ml-auto" @click="emit('delete')">Delete</button>
      </template>
      <template v-else>
        <span data-test="discard-prompt" class="text-[length:var(--app-text-micro)] text-[var(--color-text-secondary)]">放弃修改？</span>
        <button data-test="discard" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] text-[var(--color-danger)] hover:underline" @click="emit('cancel')">放弃</button>
        <button data-test="keep-editing" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] bg-[var(--color-brand-solid)] text-white hover:bg-[var(--color-brand-link)]" @click="confirming = false">继续编辑</button>
      </template>
    </div>

    <DimensionPopover
      v-if="popoverOpen"
      :dimensions="dimensions"
      :commitments="commitments"
      :dim-values="dimValues"
      class="absolute left-0 z-10"
      :class="popoverUp ? 'bottom-full mb-[4px]' : 'top-full mt-[4px]'"
      @select="onSelect"
      @close="popoverOpen = false"
    />
  </div>
</template>
