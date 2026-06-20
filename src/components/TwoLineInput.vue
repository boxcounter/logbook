<!-- src/components/TwoLineInput.vue -->
<script setup lang="ts">
import { ref, computed, inject, watch, onUnmounted, type Ref } from "vue";
import type { Dimension, Commitment } from "../types";
import { parseDurationFromText, stripDurations, formatDuration } from "../utils/format";
import DimensionPopover from "./DimensionPopover.vue";

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
  initialValues: Record<string, string>;
}>();

const emit = defineEmits<{
  submit: [item: string, durationMinutes: number, dimensions: Record<string, string>];
}>();

const text = ref("");
const inputEl = ref<HTMLInputElement | null>(null);
const popoverOpen = ref(false);
const dimValues = ref<Record<string, string>>({ ...props.initialValues });
// Set true after a submit blocked by missing required dimensions, to emphasize
// the missing chips (frontend hard-block; the backend also rejects them).
const submitAttempted = ref(false);

watch(
  () => props.initialValues,
  (vals) => { if (Object.keys(vals).length > 0) dimValues.value = { ...vals }; },
  { immediate: true }
);

const parsedDuration = computed(() => {
  const t = text.value.trim();
  return t ? parseDurationFromText(t) : null;
});

const filledDims = computed(() => props.dimensions.filter(d => dimValues.value[d.key]));
const missingRequired = computed(() => props.dimensions.filter(d => d.required && !dimValues.value[d.key]));

function tokenClass(key: string): string {
  const map: Record<string, string> = {
    category: "bg-[var(--color-token-cat-bg)] text-[var(--color-token-cat-text)]",
    "business-line": "bg-[var(--color-token-biz-bg)] text-[var(--color-token-biz-text)]",
    "importance-urgency": "bg-[var(--color-token-imp-bg)] text-[var(--color-token-imp-text)]",
    goal: "bg-[var(--color-token-goal-bg)] text-[var(--color-token-goal-text)]",
  };
  return map[key] || map.category;
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

// Close the popover on a click anywhere outside the composer (rootEl wraps the
// input, the chips that open it, and the popover itself, so in-composer clicks
// never self-close). Esc is owned by DimensionPopover (phase-aware). Listener
// only lives while the popover is open.
const rootEl = ref<HTMLElement | null>(null);
function onDocMousedown(e: MouseEvent) {
  if (rootEl.value && !rootEl.value.contains(e.target as Node)) {
    popoverOpen.value = false;
  }
}
watch(popoverOpen, (open) => {
  if (open) document.addEventListener("mousedown", onDocMousedown, true);
  else document.removeEventListener("mousedown", onDocMousedown, true);
});
onUnmounted(() => document.removeEventListener("mousedown", onDocMousedown, true));

function onKeydown(e: KeyboardEvent) {
  // Esc when the popover is closed: clear the in-progress entry. While the
  // popover is open its capture-phase listener owns Esc (back/close), and its
  // stopPropagation means this handler won't even see the event.
  if (e.key === "Escape") {
    if (popoverOpen.value) return;
    const hasContent =
      text.value.trim() !== "" ||
      JSON.stringify(dimValues.value) !== JSON.stringify(props.initialValues);
    if (!hasContent) return;
    e.preventDefault();
    text.value = "";
    dimValues.value = { ...props.initialValues };
    submitAttempted.value = false;
    return;
  }
  // Esc is owned by the popover (capture-phase window listener).
  if (e.key === "@") { e.preventDefault(); popoverOpen.value = true; return; }
  // Enter must never be blocked (spec §5.2): submit even with the popover open
  // or required dimensions unfilled. Close the popover first if it is open.
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
}

function focusInput() {
  inputEl.value?.focus();
}

defineExpose({ clearInput, focusInput });

const focusRequestId = inject<Ref<number>>("focusRequestId", ref(0));
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
      class="group bg-[var(--color-surface)] border-2 border-[var(--color-border-form)] rounded-[var(--radius-card)] px-[16px] py-[10px]
             focus-within:border-[var(--color-brand-solid)] focus-within:shadow-[var(--shadow-focus-ring)] transition-all"
    >
      <!-- Line 1: item text -->
      <div class="flex gap-[8px] items-center">
        <span class="text-[length:var(--app-text-lg)] leading-none text-[var(--color-brand-solid)] flex-shrink-0">+</span>
        <input
          ref="inputEl"
          v-model="text"
          placeholder="What did you work on?"
          class="flex-1 border-none outline-none bg-transparent text-[length:var(--app-text-base)]
                 text-[var(--color-text-primary)] placeholder:text-[var(--color-placeholder)]
                 caret-[var(--color-brand-solid)] leading-[1.5] py-[2px]"
          @keydown="onKeydown"
        />
        <span class="mono text-[length:var(--app-text-2xs)] font-semibold text-[var(--color-text-secondary)]
                     border border-[var(--color-border-form)] rounded-[var(--radius-md)] px-[7px] py-[3px] flex-shrink-0
                     opacity-50 group-focus-within:opacity-100 transition-opacity">⏎</span>
      </div>

      <!-- Line 2: tokens + missing indicators -->
      <div class="flex gap-[4px] mt-[6px] flex-wrap items-center min-h-[4px] pl-[2px]">
        <span
          v-for="d in filledDims" :key="d.key"
          data-test="dim-token"
          class="text-[length:var(--app-text-micro)] font-medium px-[7px] py-[1px] rounded-[var(--radius-sm)] inline-flex items-center gap-[4px] leading-[1.6]"
          :class="tokenClass(d.key)"
        >
          {{ dimValues[d.key] }}
          <span data-test="dim-token-remove" class="cursor-pointer opacity-40 hover:opacity-100 text-[length:var(--app-text-xs)] leading-none" @click="removeDim(d.key)">×</span>
        </span>

        <span
          v-if="parsedDuration"
          data-test="dur-token"
          class="mono text-[length:var(--app-text-micro)] font-medium px-[7px] py-[1px] rounded-[var(--radius-sm)] inline-flex items-center gap-[4px] leading-[1.6]
                 bg-[var(--color-token-dur-bg)] text-[var(--color-token-dur-text)]"
        >{{ formatDuration(parsedDuration) }}</span>

        <span
          v-for="m in missingRequired" :key="'missing-' + m.key"
          data-test="missing"
          class="text-[length:var(--app-text-micro)] font-[450] px-[8px] py-[1px] rounded-[var(--radius-sm)]
                 border-[1.5px] border-dashed inline-flex items-center gap-[3px] cursor-pointer transition-colors"
          :class="submitAttempted
            ? 'border-[var(--color-warning)] text-[var(--color-warning)]'
            : 'border-[var(--color-missing-border)] text-[var(--color-missing-text)] hover:border-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)]'"
          @click="popoverOpen = true"
        >
          <span class="w-[5px] h-[5px] rounded-full bg-[var(--color-missing-dot)]"></span>{{ m.name }}
        </span>

        <span v-if="text.trim() && !parsedDuration" class="text-[length:var(--app-text-micro)] text-[var(--color-warning)]">
          Need a duration — type <code class="mono">1h</code>
        </span>
      </div>
    </div>

    <DimensionPopover
      v-if="popoverOpen"
      :dimensions="dimensions"
      :commitments="commitments"
      :dim-values="dimValues"
      class="absolute left-0 bottom-full mb-[4px] z-10"
      @select="onSelect"
      @close="closePopover"
    />

    <!-- Hints -->
    <div class="flex gap-[14px] mt-[4px] text-[length:var(--app-text-micro)] text-[var(--color-text-disabled)] group-focus-within:text-[var(--color-text-muted)] hover:text-[var(--color-text-muted)] transition-colors">
      <span><kbd class="mono px-[5px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)] text-[length:var(--app-text-2xs)]">@</kbd> dim</span>
      <span><kbd class="mono px-[5px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)] text-[length:var(--app-text-2xs)]">#</kbd> time</span>
    </div>
  </div>
</template>
