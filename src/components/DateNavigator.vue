<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../stores/useStore";
import type { DayFile } from "../types";

const store = useStore();
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
  return new Date(store.currentDate + "T00:00:00");
}

const displayDate = computed(() => {
  const d = dateObj();
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const target = new Date(d);
  target.setHours(0, 0, 0, 0);
  const diff = Math.round((target.getTime() - today.getTime()) / 86400000);

  const s = d.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" });
  if (diff === 0) return `Today — ${s}`;
  if (diff === -1) return `Yesterday — ${s}`;
  if (diff === 1) return `Tomorrow — ${s}`;
  return s;
});

function shiftDate(days: number) {
  const d = dateObj();
  d.setDate(d.getDate() + days);
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
  } catch (e) {
    console.error("get_entries failed:", e);
  }
}

async function saveNote() {
  const text = noteRef.value?.textContent || "";
  try {
    await invoke("set_day_note", { rootPath: store.rootPath, date: store.currentDate, note: text });
  } catch (e) {
    console.error("set_day_note failed:", e);
  }
}
</script>

<template>
  <div class="flex items-center justify-between">
    <button class="px-2 py-1 text-gray-500 hover:bg-gray-100 rounded transition-colors text-sm" @click="shiftDate(-1)">←</button>
    <div class="text-center">
      <div class="text-sm font-semibold text-gray-700">{{ displayDate }}</div>
      <div
        ref="noteRef"
        class="text-xs text-gray-500 font-normal mt-0.5 outline-none rounded px-1.5 -mx-1.5 hover:bg-gray-100 focus:bg-white focus:ring-2 focus:ring-blue-500 cursor-text min-w-[60px]"
        contenteditable="true"
        data-placeholder="Add a note…"
        @blur="saveNote"
      ></div>
    </div>
    <button class="px-2 py-1 text-gray-500 hover:bg-gray-100 rounded transition-colors text-sm" @click="shiftDate(1)">→</button>
  </div>
</template>

<style scoped>
[contenteditable]:empty::before {
  content: attr(data-placeholder);
  color: #cbd5e1;
}
</style>
