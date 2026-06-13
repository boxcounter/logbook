<script setup lang="ts">
import { ref, computed, inject, watch, nextTick, type Ref } from "vue";
import type { Dimension, Commitment } from "../types";
import { parseDurationFromText, stripDurations, formatDuration } from "../utils/format";
import { logInfo } from "../utils/errorLog";
import { dimBarColor, getValueCount, firstUnfilledRequiredIndex } from '../utils/mentionHelpers';

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
  initialValues: Record<string, string>;
}>();

const input = ref("");
const error = ref("");
const inputEl = ref<HTMLInputElement | null>(null);
const menuEl = ref<HTMLElement | null>(null);

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

const DIM_ALIASES = computed<Record<string, string>>(() => {
  const map: Record<string, string> = {};
  for (const d of props.dimensions) {
    map[d.key] = d.name;
  }
  return map;
});

const staticDimensions = computed(() =>
  props.dimensions.filter((d) => d.source !== "monthly")
);
const monthlyDimension = computed(() =>
  props.dimensions.find((d) => d.source === "monthly")
);
const goalOptions = computed(() => {
  const goals = new Set<string>();
  for (const c of props.commitments) {
    for (const g of c.goals) goals.add(g);
  }
  return [...goals];
});

const allRequiredFilled = computed(() => {
  return props.dimensions
    .filter(d => d.required)
    .every(d => dimValues.value[d.key]);
});

const requiredRemaining = computed(() => {
  return props.dimensions
    .filter(d => d.required && !dimValues.value[d.key])
    .length;
});

const totalRequiredDims = computed(() =>
  props.dimensions.filter(d => d.required).length
);

const missingRequired = computed(() => {
  return props.dimensions
    .filter(d => d.required && !dimValues.value[d.key])
    .map(d => ({ key: d.key, name: d.name }));
});

const chipClass = (key: string): string => {
  const map: Record<string, string> = {
    goal: "bg-blue-50 text-blue-800",
    "business-line": "bg-amber-50 text-amber-800",
    "importance-urgency": "bg-pink-50 text-pink-800",
    category: "bg-green-50 text-green-800",
  };
  return map[key] || "bg-gray-50 text-gray-600";
};

// ---- @mention menu (all reactive refs) ----
interface MenuItem {
  label: string;
  sub?: string | null;
  key?: string;
  value?: string;
  required?: boolean;
}

const menuVisible = ref(false);
const menuPhase = ref<"dim" | "val" | null>(null);
const activeDimKey = ref<string | null>(null);
const selectedIndex = ref(0);
const filterText = ref("");

function getMenuItems(): MenuItem[] {
  const q = filterText.value.toLowerCase();
  if (menuPhase.value === "dim") {
    return props.dimensions
      .filter((d) => d.name.toLowerCase().includes(q) || d.key.toLowerCase().includes(q))
      .map((d) => ({
        label: d.name,
        sub: DIM_ALIASES.value[d.key] || d.key,
        key: d.key,
        required: d.required,
      }));
  }
  if (menuPhase.value === "val" && activeDimKey.value) {
    if (activeDimKey.value === monthlyDimension.value?.key) {
      return goalOptions.value
        .filter((v) => v.toLowerCase().includes(q))
        .map((v) => ({ label: v, value: v }));
    }
    const dim = staticDimensions.value.find((d) => d.key === activeDimKey.value);
    if (!dim?.values) return [];
    return dim.values
      .filter((v) => v.toLowerCase().includes(q))
      .map((v) => ({ label: v, value: v }));
  }
  return [];
}

function openDimMenu(skipFilled: boolean = false) {
  menuPhase.value = 'dim';
  activeDimKey.value = null;
  filterText.value = '';
  menuVisible.value = true;
  if (skipFilled) {
    const items = getMenuItems();
    selectedIndex.value = firstUnfilledRequiredIndex(items, dimValues.value);
  } else {
    selectedIndex.value = 0;
  }
}

/// Open the @ menu directly at value selection for a specific dimension.
/// Used when clicking a missing-required red chip — no @mention to replace.
function openValMenuDirect(dimKey: string) {
  menuPhase.value = "val";
  activeDimKey.value = dimKey;
  selectedIndex.value = 0;
  filterText.value = "";
  menuVisible.value = true;
  // Focus the input so keyboard navigation works
  inputEl.value?.focus();
}

/// Insert a bare @ at cursor position, so `extractFilterFromInput`
/// can still extract filter text when the menu loops back to dim phase.
function insertAtChar() {
  const cursorPos = inputEl.value?.selectionStart ?? input.value.length;
  input.value = input.value.slice(0, cursorPos) + "@" + input.value.slice(cursorPos);
}

function replaceMentionWithDimKey(dimKey: string) {
  // Replace "@filterText" in input with "@dimKey " so val-phase filter has a space delimiter
  const val = input.value;
  const cursorPos = inputEl.value?.selectionStart ?? val.length;
  const textBefore = val.slice(0, cursorPos);
  const lastAt = textBefore.lastIndexOf("@");
  if (lastAt === -1) return;
  const afterMention = val.slice(lastAt);
  const spaceIdx = afterMention.indexOf(" ");
  const mentionEnd = spaceIdx === -1 ? val.length : lastAt + spaceIdx;
  input.value = val.slice(0, lastAt) + "@" + dimKey + " " + val.slice(mentionEnd);
}

function openValMenu(dimKey: string) {
  replaceMentionWithDimKey(dimKey);
  menuPhase.value = "val";
  activeDimKey.value = dimKey;
  selectedIndex.value = 0;
  filterText.value = "";
  menuVisible.value = true;
}

function closeMenu() {
  menuVisible.value = false;
  menuPhase.value = null;
  activeDimKey.value = null;
  selectedIndex.value = 0;
  filterText.value = "";
}

/// Go back from val phase to dim phase (reverse of replaceMentionWithDimKey)
function goBackToDim() {
  const val = input.value;
  const cursorPos = inputEl.value?.selectionStart ?? val.length;
  const textBefore = val.slice(0, cursorPos);
  const lastAt = textBefore.lastIndexOf('@');
  if (lastAt === -1) return;
  const afterAt = val.slice(lastAt);
  const spaceIdx = afterAt.indexOf(' ');
  if (spaceIdx !== -1) {
    // Remove dimKey and space: "@dimKey rest" → "@rest"
    input.value = val.slice(0, lastAt) + '@' + afterAt.slice(spaceIdx + 1);
  }
  menuPhase.value = 'dim';
  activeDimKey.value = null;
  filterText.value = '';
  selectedIndex.value = 0;
}

function confirmSelection() {
  const items = getMenuItems();
  if (items.length === 0) return;
  const item = items[selectedIndex.value];
  if (menuPhase.value === "dim" && item.key) {
    openValMenu(item.key);
  } else if (menuPhase.value === "val" && activeDimKey.value && item.value) {
    dimValues.value = { ...dimValues.value, [activeDimKey.value]: item.value };
    removeMentionFromInput();
    if (allRequiredFilled.value) {
      closeMenu();
      inputEl.value?.focus();
    } else {
      skipIndexReset = true;
      insertAtChar();     // re-insert @ so filter works in dim phase
      openDimMenu(true);      // loop back to dimension list
    }
  }
}

function selectByIndex(idx: number) {
  const items = getMenuItems();
  if (idx < 0 || idx >= items.length) return;
  selectedIndex.value = idx;
  if (menuPhase.value === "dim" && items[idx].key) {
    openValMenu(items[idx].key!);
  } else if (menuPhase.value === "val" && activeDimKey.value) {
    dimValues.value = { ...dimValues.value, [activeDimKey.value]: items[idx].value || items[idx].label };
    removeMentionFromInput();
    if (allRequiredFilled.value) {
      closeMenu();
      inputEl.value?.focus();
    } else {
      skipIndexReset = true;
      insertAtChar();
      openDimMenu(true);
    }
  }
}

function extractFilterFromInput(): string {
  const val = input.value;
  const cursorPos = inputEl.value?.selectionStart ?? val.length;
  const textBeforeCursor = val.slice(0, cursorPos);
  const lastAt = textBeforeCursor.lastIndexOf("@");
  if (lastAt === -1) return "";
  const afterAt = textBeforeCursor.slice(lastAt + 1);
  if (menuPhase.value === "val") {
    // In val phase, the mention format is "@dimKey filterText" — extract after space
    const spaceIdx = afterAt.indexOf(" ");
    return spaceIdx === -1 ? "" : afterAt.slice(spaceIdx + 1);
  }
  return afterAt;
}

function removeMentionFromInput() {
  const val = input.value;
  const cursorPos = inputEl.value?.selectionStart ?? val.length;
  const textBefore = val.slice(0, cursorPos);
  const lastAt = textBefore.lastIndexOf("@");
  if (lastAt === -1) return;

  const afterAt = val.slice(lastAt);
  const spaceIdx = afterAt.indexOf(" ");
  // Remove from @ through the trailing space (if any), so no residue
  const removeEnd = spaceIdx === -1 ? val.length : lastAt + spaceIdx + 1;
  input.value = (val.slice(0, lastAt) + val.slice(removeEnd)).trim();
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
    inputEl.value?.focus();
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
let menuPending = false;
let skipIndexReset = false;

function onKeydown(e: KeyboardEvent) {
  if (!menuVisible.value) {
    if (e.key === "@") {
      e.preventDefault();
      input.value += "@";
      menuPending = true;
      nextTick(() => {
        menuPending = false;
        openDimMenu();
      });
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      if (menuPending) return;
      handleSubmit();
      return;
    }
    return;
  }

  const items = getMenuItems();

  // Arrow keys
  if (e.key === "ArrowDown") {
    e.preventDefault();
    selectedIndex.value = Math.min(selectedIndex.value + 1, Math.max(0, items.length - 1));
    return;
  }
  if (e.key === "ArrowUp") {
    e.preventDefault();
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
    return;
  }

  // macOS native: Ctrl+N / Ctrl+P / Ctrl+J / Ctrl+[
  if (e.ctrlKey && !e.altKey && !e.metaKey) {
    if (e.key === "n" || e.key === "N") {
      e.preventDefault();
      selectedIndex.value = Math.min(selectedIndex.value + 1, Math.max(0, items.length - 1));
      return;
    }
    if (e.key === "p" || e.key === "P") {
      e.preventDefault();
      selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
      return;
    }
    if (e.key === "j" || e.key === "J") {
      e.preventDefault();
      confirmSelection();
      return;
    }
    if (e.key === "[") {
      e.preventDefault();
      removeMentionFromInput();
      closeMenu();
      inputEl.value?.focus();
      return;
    }
  }

  // Enter / Tab: confirm
  if (e.key === "Enter" || e.key === "Tab") {
    e.preventDefault();
    confirmSelection();
    return;
  }

  // Escape
  if (e.key === "Escape") {
    e.preventDefault();
    removeMentionFromInput();
    closeMenu();
    inputEl.value?.focus();
    return;
  }

  // Backspace: go back from val → dim when filter is empty
  if (e.key === "Backspace" && menuPhase.value === "val" && filterText.value === "") {
    const cursorPos = inputEl.value?.selectionStart ?? 0;
    const val = input.value;
    const textBefore = val.slice(0, cursorPos);
    const lastAt = textBefore.lastIndexOf("@");
    if (lastAt !== -1) {
      const afterDim = textBefore.slice(lastAt + 1);
      if (!afterDim.includes(" ")) {
        menuPhase.value = "dim";
        activeDimKey.value = null;
        filterText.value = afterDim.slice(0, -1);
        selectedIndex.value = 0;
        return;
      }
    }
  }
}

// ---- Track filter text on every input change ----
function onInput() {
  if (menuVisible.value) {
    filterText.value = extractFilterFromInput();
    if (!skipIndexReset) {
      selectedIndex.value = 0;
    }
    skipIndexReset = false;
  }
}

// ---- Click outside to close ----
function onMenuMouseDown(e: Event) {
  e.preventDefault(); // prevent input blur
  const itemEl = (e.target as HTMLElement).closest<HTMLElement>(".mention-item");
  if (!itemEl) return;
  const idx = parseInt(itemEl.dataset.idx || "");
  if (isNaN(idx)) return;
  selectByIndex(idx);
}

function onInputBlur() {
  setTimeout(() => {
    if (menuVisible.value && menuEl.value && !menuEl.value.contains(document.activeElement)) {
      closeMenu();
    }
  }, 100);
}
</script>

<template>
  <div>
    <!-- Input row -->
    <div class="flex gap-2 relative">
      <input
        ref="inputEl"
        v-model="input"
        type="text"
        class="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-sm"
        placeholder="Sprint planning 1.5h  (type @ to set dimensions)"
        @keydown="onKeydown"
        @input="onInput"
        @blur="onInputBlur"
      />
      <button
        type="button"
        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 text-sm font-medium"
        :disabled="!input.trim() || submitting"
        @click="handleSubmit"
      >
        Log
      </button>

      <!-- @mention menu -->
      <div
        v-if="menuVisible"
        ref="menuEl"
        class="absolute left-0 right-0 bg-white border border-gray-200 rounded-lg shadow-lg z-20 text-sm max-h-52 overflow-y-auto"
        style="top: 100%; margin-top: 4px;"
        @mousedown="onMenuMouseDown"
      >
        <!-- dim phase header -->
        <div
          v-if="menuPhase === 'dim'"
          class="px-3 py-1.5 text-[10px] uppercase tracking-wide border-b border-gray-100 bg-gray-800 text-gray-200 flex items-center gap-2"
        >
          <span class="bg-gray-600 px-1.5 py-0.5 rounded text-[9px] font-medium">DIM</span>
          Pick a dimension
        </div>
        <!-- val phase header -->
        <div
          v-else-if="menuPhase === 'val'"
          class="px-3 py-1.5 text-[10px] border-b border-gray-100 bg-blue-50 text-blue-600 flex items-center gap-2"
        >
          <button type="button" class="font-bold text-xs hover:text-blue-800 leading-none" @click="goBackToDim">&larr;</button>
          <span>Pick a value for <b class="text-blue-800">{{ activeDimKey ? (DIM_ALIASES[activeDimKey] || activeDimKey) : '' }}</b></span>
        </div>
        <template v-for="(item, i) in getMenuItems()" :key="i">
          <div
            class="mention-item flex items-center gap-2 px-3 py-1.5 cursor-pointer"
            :class="{ 'bg-blue-50': i === selectedIndex }"
            :data-idx="i"
          >
            <!-- Dim phase: colored bar; Val phase: no bar -->
            <span
              v-if="menuPhase === 'dim'"
              class="w-[3px] h-[22px] rounded-full flex-shrink-0"
              :class="dimBarColor(item.key || '')"
            ></span>
            <span
              class="flex-1"
              :class="{ 'pl-1': menuPhase === 'val' }"
            >{{ item.label }}</span>
            <!-- Dim phase right-side info: value count or filled checkmark -->
            <span
              v-if="menuPhase === 'dim' && item.required && !dimValues[item.key || '']"
              class="text-[10px] text-gray-400"
            >{{ getValueCount(props.dimensions, item.key || '', goalOptions) }} values</span>
            <span
              v-else-if="menuPhase === 'dim' && item.required && dimValues[item.key || '']"
              class="text-[10px] text-green-500"
            >{{ dimValues[item.key || ''] }} ✓</span>
          </div>
        </template>
        <div v-if="getMenuItems().length === 0" class="px-3 py-2 text-gray-400 text-xs">
          No matches
        </div>
        <!-- Dim phase footer: dot progress indicator -->
        <div
          v-if="menuPhase === 'dim' && totalRequiredDims > 0"
          class="px-3 py-1.5 text-[10px] border-t border-gray-100 flex items-center gap-1"
          :class="allRequiredFilled ? 'text-green-600' : 'text-gray-400'"
        >
          <template v-if="allRequiredFilled">
            All required ✓
          </template>
          <template v-else>
            <span
              v-for="n in totalRequiredDims"
              :key="n"
              class="inline-block w-[6px] h-[6px] rounded-full"
              :class="n <= (totalRequiredDims - requiredRemaining) ? 'bg-green-400' : 'bg-gray-300'"
            ></span>
            <span class="ml-1">{{ requiredRemaining }} to go</span>
          </template>
        </div>
        <!-- Val phase footer: navigation hint -->
        <div
          v-if="menuPhase === 'val'"
          class="px-3 py-1.5 text-[10px] text-gray-400 border-t border-gray-100"
        >
          &larr; Back to dimensions · Type to filter
        </div>
      </div>
    </div>

    <div class="flex justify-between mt-1 min-h-[1.25rem]">
      <span v-if="parsedPreview" class="text-xs text-gray-500">Duration: {{ parsedPreview }}</span>
      <span v-if="error" class="text-xs text-red-500">{{ error }}</span>
    </div>

    <!-- #6: Dimension chips row -->
    <div class="flex flex-wrap gap-1.5 mt-2 min-h-[24px] items-center">
      <span
        v-for="dim in dimensions"
        :key="dim.key"
        v-show="dimValues[dim.key]"
        class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs cursor-pointer border"
        :class="chipClass(dim.key)"
        @click="dimValues = { ...dimValues, [dim.key]: '' }"
      >
        {{ dim.name }}: {{ dimValues[dim.key] }}
        <span class="opacity-40 hover:opacity-100 leading-none">&times;</span>
      </span>
      <!-- Missing required chips (red dashed) -->
      <span
        v-for="m in missingRequired"
        :key="'missing-' + m.key"
        class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs cursor-pointer border border-dashed border-red-400 bg-red-50 text-red-700"
        @click="openValMenuDirect(m.key)"
      >
        + {{ m.name }}
      </span>
      <span v-if="Object.values(dimValues).every(v => !v) && missingRequired.length === 0" class="text-xs text-gray-400 italic">
        @ to set dimensions
      </span>
    </div>
  </div>
</template>
