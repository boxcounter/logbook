<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../stores/useStore";
import type { DayFile, Granularity } from "../types";
import { logError } from "../utils/errorLog";
import { weekLabel, parseDate } from "../utils/dates";

const store = useStore();
const emit = defineEmits<{ navigate: [] }>();
const noteRef = ref<HTMLDivElement>();

// Sync note from store → DOM (not via template interpolation to avoid VDOM conflict)
watch(
  () => store.today?.note,
  (n) => {
    if (noteRef.value && noteRef.value.textContent !== (n || "")) {
      noteRef.value.textContent = n || "";
    }
  },
  { immediate: true }
);

function dateObj(): Date {
  return parseDate(store.currentDate);
}

const displayDate = computed(() => {
  const d = dateObj();
  const fmt = d.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" });
  if (store.granularity === "day") {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const target = new Date(d);
    target.setHours(0, 0, 0, 0);
    const diff = Math.round((target.getTime() - today.getTime()) / 86400000);
    if (diff === 0) return `Today — ${fmt}`;
    if (diff === -1) return `Yesterday — ${fmt}`;
    if (diff === 1) return `Tomorrow — ${fmt}`;
    return fmt;
  }
  if (store.granularity === "week") {
    return weekLabel(d);
  }
  return d.toLocaleDateString("en-US", { month: "long", year: "numeric" });
});

function shift(delta: number) {
  const d = dateObj();
  if (store.granularity === "day") {
    d.setDate(d.getDate() + delta);
  } else if (store.granularity === "week") {
    d.setDate(d.getDate() + delta * 7);
  } else {
    d.setMonth(d.getMonth() + delta);
  }
  store.currentDate = [
    d.getFullYear(),
    String(d.getMonth() + 1).padStart(2, "0"),
    String(d.getDate()).padStart(2, "0"),
  ].join("-");
  loadDay();
}

async function loadDay() {
  try {
    const df = (await invoke("get_entries", { rootPath: store.rootPath, date: store.currentDate })) as DayFile;
    store.today = df;
    emit("navigate");
  } catch (e) {
    logError("DateNavigator.loadDay", e);
  }
}

function onNotePaste(e: ClipboardEvent) {
  e.preventDefault();
  const text = e.clipboardData?.getData("text/plain") || "";
  const selection = window.getSelection();
  if (selection && selection.rangeCount > 0) {
    const range = selection.getRangeAt(0);
    range.deleteContents();
    range.insertNode(document.createTextNode(text));
    range.collapse(false);
  }
}

function onNoteInput() {
  if (noteRef.value && noteRef.value.innerHTML !== noteRef.value.textContent) {
    noteRef.value.textContent = noteRef.value.textContent || "";
  }
}

async function saveNote() {
  const text = noteRef.value?.textContent || "";
  try {
    await invoke("set_day_note", { rootPath: store.rootPath, date: store.currentDate, note: text });
  } catch (e) {
    logError("DateNavigator.saveNote", e);
  }
}
</script>

<template>
  <div class="flex items-center justify-between">
    <button class="px-2 py-1 text-gray-500 hover:bg-gray-100 rounded transition-colors text-sm" @click="shift(-1)">←</button>
    <div class="text-center flex flex-col items-center gap-1">
      <div class="text-sm font-semibold text-gray-700">{{ displayDate }}</div>
      <select
        :value="store.granularity"
        class="text-xs border border-gray-300 rounded px-1.5 py-0.5 focus:outline-none focus:ring-1 focus:ring-blue-500"
        @change="store.granularity = ($event.target as HTMLSelectElement).value as Granularity; loadDay()"
      >
        <option value="day">Day</option>
        <option value="week">Week</option>
        <option value="month">Month</option>
      </select>
      <div
        ref="noteRef"
        class="text-xs text-gray-500 font-normal mt-0.5 outline-none rounded px-1.5 -mx-1.5 hover:bg-gray-100 focus:bg-white focus:ring-2 focus:ring-blue-500 cursor-text min-w-[60px]"
        contenteditable="true"
        data-placeholder="Add a note…"
        @blur="saveNote"
        @paste="onNotePaste"
        @input="onNoteInput"
      ></div>
    </div>
    <button class="px-2 py-1 text-gray-500 hover:bg-gray-100 rounded transition-colors text-sm" @click="shift(1)">→</button>
  </div>
</template>

<style scoped>
[contenteditable]:empty::before {
  content: attr(data-placeholder);
  color: #cbd5e1;
}
</style>
