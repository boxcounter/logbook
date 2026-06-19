<script setup lang="ts">
import { ref, computed, inject, watch, type Ref } from "vue";
import type { Dimension, Commitment } from "../types";
import { parseDurationFromText, stripDurations, formatDuration } from "../utils/format";
import { logInfo } from "../utils/errorLog";
import AppInput from './base/AppInput.vue';
import AppButton from './base/AppButton.vue';
import AppChip from './base/AppChip.vue';
import MentionMenu from './composite/MentionMenu.vue';

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
  initialValues: Record<string, string>;
}>();

const input = ref("");
const error = ref("");
const inputEl = ref<InstanceType<typeof AppInput> | null>(null);

const emit = defineEmits<{
  submit: [item: string, durationMinutes: number, dimensions: Record<string, string>];
}>();

// ---- Dimension state (synced with chips) ----
const dimValues = ref<Record<string, string>>({ ...props.initialValues });

watch(
  () => props.initialValues,
  (vals) => {
    if (Object.keys(vals).length > 0) {
      dimValues.value = { ...vals };
    }
  },
  { immediate: true }
);

const allRequiredFilled = computed(() => {
  return props.dimensions
    .filter(d => d.required)
    .every(d => dimValues.value[d.key]);
});

const missingRequired = computed(() => {
  return props.dimensions
    .filter(d => d.required && !dimValues.value[d.key])
    .map(d => ({ key: d.key, name: d.name }));
});

function chipColor(key: string): 'category' | 'biz' | 'importance' | 'goal' | 'missing' {
  const map: Record<string, 'category' | 'biz' | 'importance' | 'goal' | 'missing'> = {
    goal: 'goal',
    'business-line': 'biz',
    'importance-urgency': 'importance',
    category: 'category',
  };
  return map[key] || 'missing';
}

// ---- @mention menu ----
const menuVisible = ref(false);

function insertAtChar() {
  const cursorPos = inputEl.value?.inputEl?.selectionStart ?? input.value.length;
  input.value = input.value.slice(0, cursorPos) + "@" + input.value.slice(cursorPos);
}

function removeMentionFromInput() {
  const val = input.value;
  const cursorPos = inputEl.value?.inputEl?.selectionStart ?? val.length;
  const textBefore = val.slice(0, cursorPos);
  const lastAt = textBefore.lastIndexOf("@");
  if (lastAt === -1) return;

  const afterAt = val.slice(lastAt);
  const spaceIdx = afterAt.indexOf(" ");
  // Remove from @ through the trailing space (if any), so no residue
  const removeEnd = spaceIdx === -1 ? val.length : lastAt + spaceIdx + 1;
  input.value = (val.slice(0, lastAt) + val.slice(removeEnd)).trim();
}

function closeMenu() {
  menuVisible.value = false;
}

function onMentionSelect(dimKey: string, value: string) {
  dimValues.value = { ...dimValues.value, [dimKey]: value };
  removeMentionFromInput();
  if (allRequiredFilled.value) {
    closeMenu();
    inputEl.value?.inputEl?.focus();
  } else {
    // MentionMenu loops back to dim phase internally.
    // Re-insert @ so the user can type filter text.
    insertAtChar();
  }
}

// ---- Duration preview ----
const parsedPreview = computed(() => {
  if (!input.value.trim()) return null;
  const d = parseDurationFromText(input.value.trim());
  if (!d) return null;
  return `${formatDuration(d)} (${d}m)`;
});

// ---- #1: window focus → auto-focus ----
const focusRequestId = inject<Ref<number>>("focusRequestId", ref(0));
watch(focusRequestId, () => {
  const active = document.activeElement;
  if (!active || active === document.body || active.tagName === "BODY") {
    inputEl.value?.inputEl?.focus();
  }
});

// ---- Submit ----
const submitting = ref(false);

function handleSubmit() {
  if (submitting.value) return;
  error.value = "";
  if (!allRequiredFilled.value) {
    error.value = `Missing required: ${missingRequired.value.map(m => m.name).join(", ")}`;
    return;
  }
  const trimmed = input.value.trim();
  if (!trimmed) return;
  let d: number | null;
  try {
    d = parseDurationFromText(trimmed);
  } catch (e) {
    logInfo("EntryInput.parseDuration", `error: ${e} input: ${trimmed}`);
    error.value = "Parse error";
    return;
  }
  if (!d) {
    error.value = "Could not parse duration. Examples: 1.5h, 30m, 45";
    return;
  }
  let item: string;
  try {
    item = stripDurations(trimmed);
  } catch (e) {
    logInfo("EntryInput.stripDurations", `error: ${e}`);
    error.value = "Parse error";
    return;
  }
  logInfo("EntryInput.handleSubmit", `item="${item}" dur=${d}m dims=${JSON.stringify(dimValues.value)}`);
  submitting.value = true;
  try {
    emit("submit", item, d, { ...dimValues.value });
  } catch (e) {
    logInfo("EntryInput.emit", `error: ${e}`);
  } finally {
    submitting.value = false;
  }
}

function clearInput() {
  input.value = "";
}

defineExpose({ clearInput });

// ---- Keyboard handler ----
function onKeydown(e: KeyboardEvent) {
  if (menuVisible.value) {
    // MentionMenu handles its own keyboard navigation internally.
    // Only handle Escape to close the menu.
    if (e.key === 'Escape') {
      e.preventDefault();
      removeMentionFromInput();
      closeMenu();
      inputEl.value?.inputEl?.focus();
    }
    return;
  }
  if (e.key === '@') {
    e.preventDefault();
    input.value += '@';
    menuVisible.value = true;
    return;
  }
  if (e.key === 'Enter') {
    e.preventDefault();
    handleSubmit();
    return;
  }
}

// ---- Input handlers ----
function onInput() {
  // MentionMenu manages filter state internally.
  // Kept as a binding point for future needs.
}

function onInputBlur() {
  setTimeout(() => {
    if (menuVisible.value) {
      closeMenu();
    }
  }, 100);
}
</script>

<template>
  <div>
    <!-- Input row -->
    <div class="flex gap-2 relative">
      <AppInput
        ref="inputEl"
        v-model="input"
        placeholder="Sprint planning 1.5h  (type @ to set dimensions)"
        @keydown="onKeydown"
        @input="onInput"
        @blur="onInputBlur"
      />
      <AppButton
        :disabled="!input.trim() || submitting"
        @click="handleSubmit"
      >
        Log
      </AppButton>

      <MentionMenu
        v-if="menuVisible"
        :dimensions="dimensions"
        :commitments="commitments"
        :dim-values="dimValues"
        @select="onMentionSelect"
        @close="closeMenu(); removeMentionFromInput(); inputEl?.inputEl?.focus()"
      />
    </div>

    <div class="flex justify-between mt-1 min-h-[1.25rem]">
      <span v-if="parsedPreview" class="text-[var(--app-text-sm)] text-[var(--color-text-secondary)]">Duration: {{ parsedPreview }}</span>
      <span v-if="error" class="text-[var(--app-text-sm)] text-[var(--color-danger)]">{{ error }}</span>
    </div>

    <!-- #6: Dimension chips row -->
    <div class="flex flex-wrap gap-1.5 mt-2 min-h-[24px] items-center">
      <AppChip
        v-for="dim in dimensions"
        :key="dim.key"
        v-show="dimValues[dim.key]"
        :color="chipColor(dim.key)"
        :label="dim.name"
        :value="dimValues[dim.key]"
        closable
        @close="dimValues = { ...dimValues, [dim.key]: '' }"
      />
      <!-- Missing required chips (red dashed) -->
      <span
        v-for="m in missingRequired"
        :key="'missing-' + m.key"
        class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs cursor-pointer border border-dashed border-[var(--color-chip-missing-border)] bg-[var(--color-chip-missing-bg)] text-[var(--color-chip-missing-text)]"
        @click="menuVisible = true"
      >
        + {{ m.name }}
      </span>
      <span v-if="Object.values(dimValues).every(v => !v) && missingRequired.length === 0" class="text-[var(--app-text-sm)] text-[var(--color-text-secondary)] italic">
        @ to set dimensions
      </span>
    </div>
  </div>
</template>
