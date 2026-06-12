<script setup lang="ts">
import { ref, computed, inject, watch, nextTick, type Ref } from "vue";
import type { Dimension, Commitment } from "../types";
import { parseDurationFromText, stripDurations, formatDuration } from "../utils/format";
import { logInfo } from "../utils/errorLog";

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
      .map((d) => ({ label: d.name, sub: DIM_ALIASES.value[d.key] || d.key, key: d.key }));
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

function openDimMenu() {
  menuPhase.value = "dim";
  activeDimKey.value = null;
  selectedIndex.value = 0;
  filterText.value = "";
  menuVisible.value = true;
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

function confirmSelection() {
  const items = getMenuItems();
  if (items.length === 0) return;
  const item = items[selectedIndex.value];
  if (menuPhase.value === "dim" && item.key) {
    openValMenu(item.key);
  } else if (menuPhase.value === "val" && activeDimKey.value && item.value) {
    dimValues.value = { ...dimValues.value, [activeDimKey.value]: item.value };
    removeMentionFromInput();
    closeMenu();
    inputEl.value?.focus();
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
    closeMenu();
    inputEl.value?.focus();
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

  // Number keys: quick-select (1-9)
  if (e.key >= "1" && e.key <= "9" && !e.ctrlKey && !e.metaKey) {
    e.preventDefault();
    selectByIndex(parseInt(e.key) - 1);
    return;
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
    selectedIndex.value = 0;
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
        <div class="px-3 py-1.5 text-[10px] text-gray-400 uppercase tracking-wide border-b border-gray-100">
          <template v-if="menuPhase === 'dim'">Pick a dimension</template>
          <template v-else>Pick a value for <b>{{ activeDimKey ? (DIM_ALIASES[activeDimKey] || activeDimKey) : '' }}</b></template>
        </div>
        <template v-for="(item, i) in getMenuItems()" :key="i">
          <div
            class="mention-item flex items-center gap-2 px-3 py-1.5 cursor-pointer"
            :class="{ 'bg-blue-50': i === selectedIndex }"
            :data-idx="i"
          >
            <span
              class="text-[10px] rounded w-[18px] h-[18px] inline-flex items-center justify-center flex-shrink-0 tabular-nums"
              :class="i === selectedIndex ? 'bg-blue-600 text-white' : 'bg-gray-100 text-gray-400'"
            >{{ i + 1 }}</span>
            <span class="flex-1">{{ item.label }}</span>
            <span v-if="menuPhase === 'dim' && item.sub" class="text-[10px] text-gray-400 bg-gray-100 px-1.5 py-0.5 rounded">{{ item.sub }}</span>
          </div>
        </template>
        <div v-if="getMenuItems().length === 0" class="px-3 py-2 text-gray-400 text-xs">
          No matches
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
      <span v-if="Object.values(dimValues).every(v => !v)" class="text-xs text-gray-400 italic">
        @ to set dimensions
      </span>
    </div>
  </div>
</template>
