<!-- src/components/EntryComposer.vue -->
<script setup lang="ts">
import { ref, computed, inject, watch } from "vue";
import { useClickOutside } from "../composables/useClickOutside";
import type { Dimension, Commitment } from "../types";
import { FOCUS_REQUEST_KEY } from "../types";
import { useStore } from "../stores/useStore";
import { parseDurationFromText, stripDurations, formatDuration } from "../utils/format";
import { dimensionHues, dimTokenChipStyle } from "../utils/dimensionColor";
import DimensionPopover from "./DimensionPopover.vue";

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
}>();

const emit = defineEmits<{
  submit: [item: string, durationMinutes: number, dimensions: Record<string, string>];
  editDimensions: [];
}>();

const text = ref("");
const inputEl = ref<HTMLInputElement | null>(null);
const popoverOpen = ref(false);
const dimValues = ref<Record<string, string>>({});
const store = useStore();

// Clear dimension selections when navigating to a different date so
// chips from the previous day don't misleadingly appear in the composer.
watch(() => store.currentDate, () => {
  dimValues.value = {};
  text.value = "";
});
// Set true after a submit blocked by missing required dimensions, to emphasize
// the missing chips (frontend hard-block; the backend also rejects them).
const submitAttempted = ref(false);

const parsedDuration = computed(() => {
  const t = text.value.trim();
  return t ? parseDurationFromText(t) : null;
});

const filledDims = computed(() => props.dimensions.filter(d => !d.deleted && dimValues.value[d.key]));
const missingRequired = computed(() => props.dimensions.filter(d => !d.deleted && d.required && !dimValues.value[d.key]));

const composerHues = computed(() => dimensionHues(props.dimensions));
function tokenChipStyle(key: string) {
  return dimTokenChipStyle(composerHues.value.get(key) ?? null);
}

function removeDim(key: string) {
  const next = { ...dimValues.value };
  delete next[key];
  dimValues.value = next;
}

function onSelect(dimKey: string, value: string) {
  dimValues.value = { ...dimValues.value, [dimKey]: value };
}

function closePopover() {
  popoverOpen.value = false;
  inputEl.value?.focus();
}

const rootEl = ref<HTMLElement | null>(null);
useClickOutside(rootEl, popoverOpen);

function onKeydown(e: KeyboardEvent) {
  // Esc when the popover is closed: clear the in-progress entry. While the
  // popover is open its capture-phase listener owns Esc (back/close), and its
  // stopPropagation means this handler won't even see the event.
  if (e.key === "Escape") {
    if (popoverOpen.value) return;
    const hasContent =
      text.value.trim() !== "" || Object.keys(dimValues.value).length > 0;
    if (!hasContent) return;
    e.preventDefault();
    text.value = "";
    dimValues.value = {};
    submitAttempted.value = false;
    return;
  }
  // Esc is owned by the popover (capture-phase window listener).
  if (e.key === "@") { e.preventDefault(); popoverOpen.value = true; return; }
  // Enter submits the entry. While the popover is open, its capture-phase window
  // listener owns Enter (selects the highlighted item) and stops propagation, so
  // this handler only runs with the popover closed. The closePopover() guard is a
  // defensive fallback in case that listener ever isn't attached.
  if (e.key === "Enter") {
    e.preventDefault();
    if (popoverOpen.value) closePopover();
    handleSubmit();
    return;
  }
}

function handleSubmit() {
  const trimmed = text.value.trim();
  if (!trimmed) return;
  const d = parsedDuration.value;
  if (!d) return; // duration required; template shows the hint
  if (missingRequired.value.length > 0) {
    // Hard-block: the backend rejects entries missing required dimensions, so
    // stop here and flag the missing chips instead of failing silently.
    submitAttempted.value = true;
    return;
  }
  submitAttempted.value = false;
  const item = stripDurations(trimmed);
  emit("submit", item, d, { ...dimValues.value });
}

function clearInput() {
  text.value = "";
  dimValues.value = {};
  submitAttempted.value = false;
}

function focusInput() {
  inputEl.value?.focus();
}

function hasUnsavedContent(): boolean {
  return text.value.trim() !== "" || Object.keys(dimValues.value).length > 0;
}

defineExpose({ clearInput, focusInput, hasUnsavedContent });

const focusRequestId = inject(FOCUS_REQUEST_KEY, ref(0));
watch(focusRequestId, () => {
  // On a window refocus, claim the input unless the user is actively editing
  // another field (the day note, an entry-edit input, etc.). Checking for an
  // editable element rather than only document.body is robust to webviews that
  // restore focus to the <html> element on refocus.
  const active = document.activeElement as HTMLElement | null;
  const editing =
    !!active &&
    (active.tagName === "INPUT" ||
      active.tagName === "TEXTAREA" ||
      active.tagName === "SELECT" ||
      active.isContentEditable);
  if (!editing) inputEl.value?.focus();
});
</script>

<template>
  <div ref="rootEl" class="relative group">
    <div
      class="group bg-[var(--color-surface)] border-2 border-[var(--color-border-form)] rounded-[var(--radius-card)] px-lg py-sm
             focus-within:border-[var(--color-brand-solid)] focus-within:shadow-[var(--shadow-focus-ring)] transition-all"
    >
      <!-- Line 1: item text -->
      <div class="flex gap-sm items-center">
        <span class="text-[length:var(--glyph-plus)] leading-none text-[var(--color-brand-solid)] flex-shrink-0">+</span>
        <input
          ref="inputEl"
          v-model="text"
          placeholder="What did you work on?"
          class="flex-1 border-none outline-none bg-transparent text-body
                 text-[var(--color-text-primary)] placeholder:text-[var(--color-placeholder)]
                 caret-[var(--color-brand-solid)] py-2xs"
          @keydown="onKeydown"
        />
        <span class="mono text-micro font-semibold text-[var(--color-text-secondary)]
                     border border-[var(--color-border-form)] rounded-[var(--radius-md)] px-sm py-xs flex-shrink-0
                     opacity-50 group-focus-within:opacity-100 transition-opacity">⏎</span>
        <span
          class="text-[length:14px] text-[var(--color-text-muted)] hover:text-[var(--color-brand-solid)] cursor-pointer flex-shrink-0 py-2xs px-2xs"
          @click.stop="$emit('editDimensions')"
          title="Edit dimensions"
        >⚙</span>
      </div>

      <!-- Line 2: tokens + missing indicators -->
      <div class="flex gap-xs mt-sm flex-wrap items-center min-h-[4px] pl-2xs">
        <span
          v-for="d in filledDims" :key="d.key"
          data-test="dim-token"
          class="text-micro font-medium px-sm py-2xs rounded-[var(--radius-sm)] inline-flex items-center gap-xs"
          :style="tokenChipStyle(d.key)"
        >
          {{ dimValues[d.key] }}
          <span data-test="dim-token-remove" class="cursor-pointer opacity-40 hover:opacity-100 text-secondary leading-none" @click="removeDim(d.key)">×</span>
        </span>

        <span
          v-if="parsedDuration"
          data-test="dur-token"
          class="mono text-micro font-medium px-sm py-2xs rounded-[var(--radius-sm)] inline-flex items-center gap-xs
                 bg-[var(--color-token-dur-bg)] text-[var(--color-token-dur-text)]"
        >{{ formatDuration(parsedDuration) }}</span>

        <span
          v-for="m in missingRequired" :key="'missing-' + m.key"
          data-test="missing"
          class="text-micro font-[450] px-sm py-2xs rounded-[var(--radius-sm)]
                 border-[1.5px] border-dashed inline-flex items-center gap-xs cursor-pointer transition-colors"
          :class="submitAttempted
            ? 'border-[var(--color-warning)] text-[var(--color-warning)]'
            : 'border-[var(--color-missing-border)] text-[var(--color-missing-text)] hover:border-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)]'"
          @click="popoverOpen = true"
        >
          <span class="w-[5px] h-[5px] rounded-full bg-[var(--color-missing-dot)]"></span>{{ m.name }}
        </span>

        <span v-if="text.trim() && !parsedDuration" class="text-micro text-[var(--color-warning)]">
          Need a duration — type <code class="mono">1h</code>
        </span>
      </div>
    </div>

    <DimensionPopover
      v-if="popoverOpen"
      :dimensions="dimensions"
      :commitments="commitments"
      :dim-values="dimValues"
      class="absolute left-0 bottom-full mb-xs z-10"
      @select="onSelect"
      @close="closePopover"
    />

    <!-- Hints -->
    <div class="flex gap-md mt-xs text-micro text-[var(--color-text-disabled)] group-focus-within:text-[var(--color-text-muted)] hover:text-[var(--color-text-muted)] transition-colors">
      <span><kbd class="mono px-xs border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)] text-micro">@</kbd> dim</span>
      <span><kbd class="mono px-xs border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)] text-micro">#</kbd> time</span>
    </div>
  </div>
</template>
