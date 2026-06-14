<script setup lang="ts">
import { inject, computed, watch, ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../stores/useStore";
import MonthNavigator from "./MonthNavigator.vue";
import CommitmentsPanel from "./CommitmentsPanel.vue";
import DayStrip from "./DayStrip.vue";
import QuickEntry from "./QuickEntry.vue";
import EntryList from "./EntryList.vue";
import type { DayFile, Entry, CommitmentProgress } from "../types";
import { logError } from "../utils/errorLog";
import { datesInMonth, yearMonthFromDate } from "../utils/dates";

const store = useStore();

const selectedYear = computed(() => yearMonthFromDate(store.currentDate).year);
const selectedMonth = computed(() => yearMonthFromDate(store.currentDate).month);

const monthDates = computed(() => datesInMonth(store.currentDate));

const isSelectedToday = computed(() => {
  const now = new Date();
  const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  return store.currentDate === today;
});

const triggerUndoToast = inject<(undoFn: () => void) => void>("triggerUndoToast", () => {});

// ---- Month loading ----

async function loadMonth(year: number, month: number, defaultDay?: number) {
  const now = new Date();
  const isCurrentMonth = year === now.getFullYear() && month === now.getMonth() + 1;

  let day: number;
  if (defaultDay !== undefined) {
    day = defaultDay;
  } else if (isCurrentMonth) {
    day = now.getDate();
  } else {
    day = new Date(year, month, 0).getDate();
  }

  const dateStr = `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
  store.currentDate = dateStr;

  const dates = datesInMonth(dateStr);
  const map: Record<string, Entry[]> = {};
  for (const date of dates) {
    try {
      const df = (await invoke("get_entries", { rootPath: store.rootPath, date })) as DayFile;
      map[date] = df.entries;
    } catch (e) {
      logError("MonthView.loadMonth", e);
      map[date] = [];
    }
  }
  store.monthEntries = map;

  await loadCommitmentProgress(year, month);

  if (store.currentDate in map) {
    store.today = { note: null, entries: map[store.currentDate] };
    loadDayNote(store.currentDate);
  }
}

async function loadCommitmentProgress(year: number, month: number) {
  try {
    store.commitmentProgress = (await invoke("get_commitment_progress", {
      rootPath: store.rootPath,
      year,
      month,
    })) as CommitmentProgress[];
  } catch (e) {
    logError("MonthView.loadCommitmentProgress", e);
    store.commitmentProgress = [];
  }
}

async function loadDayNote(dateStr: string) {
  try {
    const df = (await invoke("get_entries", { rootPath: store.rootPath, date: dateStr })) as DayFile;
    if (store.today) {
      store.today.note = df.note;
    }
  } catch (e) {
    logError("MonthView.loadDayNote", e);
  }
}

// ---- Day selection ----

async function handleSelectDay(dateStr: string) {
  store.currentDate = dateStr;
  if (dateStr in store.monthEntries) {
    store.today = { note: null, entries: store.monthEntries[dateStr] };
    await loadDayNote(dateStr);
  }
}

// ---- Month navigation ----

async function handleNavigate({ year, month }: { year: number; month: number }) {
  await loadMonth(year, month);
}

// ---- Lazy load available months ----

async function handleRequestMonths() {
  if (store.availableMonths !== null) return;
  try {
    const months = (await invoke("get_available_months", { rootPath: store.rootPath })) as { year: number; month: number }[];
    store.availableMonths = months;
  } catch (e) {
    logError("MonthView.handleRequestMonths", e);
    store.availableMonths = [];
  }
}

// ---- Entry mutations ----

async function handleUpdateEntry(entryId: string, item: string, durationMinutes: number) {
  const entries = store.today?.entries;
  if (!entries) return;
  const entry = entries.find(e => e.id === entryId);
  if (!entry) return;

  const update: Record<string, unknown> = {};
  if (item !== entry.item) update.item = item;
  if (durationMinutes !== entry.duration) update.duration = String(durationMinutes);
  if (Object.keys(update).length === 0) return;

  try {
    const df = (await invoke("update_entry", {
      rootPath: store.rootPath,
      date: store.currentDate,
      entryId,
      update,
    })) as DayFile;
    store.today = df;
    store.monthEntries[store.currentDate] = df.entries;
    await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
  } catch (e) {
    logError("MonthView.handleUpdateEntry", e);
  }
}

async function handleUpdateDimensions(entryId: string, dimensions: Record<string, string>) {
  try {
    const df = (await invoke("update_entry", {
      rootPath: store.rootPath,
      date: store.currentDate,
      entryId,
      update: { dimensions },
    })) as DayFile;
    store.today = df;
    store.monthEntries[store.currentDate] = df.entries;
    await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
  } catch (e) {
    logError("MonthView.handleUpdateDimensions", e);
  }
}

let pendingDeleteTimer: ReturnType<typeof setTimeout> | null = null;

async function handleDeleteEntry(entryId: string) {
  const entries = store.today?.entries;
  if (!entries) return;

  const idx = entries.findIndex(e => e.id === entryId);
  if (idx === -1) return;
  const [removed] = entries.splice(idx, 1);

  let cancelled = false;
  pendingDeleteTimer = setTimeout(async () => {
    if (cancelled) return;
    try {
      await invoke("delete_entry", { rootPath: store.rootPath, date: store.currentDate, entryId });
      store.monthEntries[store.currentDate] = [...entries];
      await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
    } catch (e) {
      logError("MonthView.handleDeleteEntry", e);
      const currentIdx = entries.findIndex(e => e.id === entryId);
      if (currentIdx === -1) {
        entries.splice(idx, 0, removed);
      }
    }
  }, 5000);

  triggerUndoToast(() => {
    cancelled = true;
    if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
    const currentIdx = entries.findIndex(e => e.id === entryId);
    if (currentIdx === -1) {
      entries.splice(idx, 0, removed);
    }
  });
}

async function handleAppended() {
  await loadMonth(selectedYear.value, selectedMonth.value, parseInt(store.currentDate.split("-")[2], 10));
}

// ---- Day note ----

const noteRef = ref<HTMLDivElement>();

watch(
  () => store.today?.note,
  (n) => {
    if (noteRef.value && noteRef.value.textContent !== (n || "")) {
      noteRef.value.textContent = n || "";
    }
  },
  { immediate: true }
);

onMounted(async () => {
  if (store.rootPath) {
    const { year, month } = yearMonthFromDate(store.currentDate);
    await loadMonth(year, month);
  }
});

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
    logError("MonthView.saveNote", e);
  }
}

// ---- File path ----

const dayFilePath = computed(() => {
  if (!store.rootPath) return "";
  const d = store.currentDate;
  const year = d.slice(0, 4);
  const month = d.slice(5, 7);
  return `${year}/${month}/${d}.md`;
});

const displayPath = computed(() => {
  if (!store.rootPath) return "";
  return `…/${dayFilePath.value}`;
});

async function openInEditor() {
  if (!store.rootPath) return;
  try {
    await invoke("open_in_editor", { rootPath: store.rootPath, date: store.currentDate });
  } catch (e) {
    logError("MonthView.openInEditor", e);
  }
}
</script>

<template>
  <div class="flex gap-4 p-4 max-w-7xl mx-auto items-start">
    <!-- Left 1/3: Month sidebar -->
    <div class="flex-1 min-w-[200px] flex flex-col gap-3 sticky top-4">
      <MonthNavigator
        :year="selectedYear"
        :month="selectedMonth"
        :availableMonths="store.availableMonths"
        @navigate="handleNavigate"
        @requestMonths="handleRequestMonths"
      />
      <CommitmentsPanel
        :progress="store.commitmentProgress"
        :selectedYear="selectedYear"
        :selectedMonth="selectedMonth"
      />
    </div>

    <!-- Right 2/3: Day detail -->
    <div class="flex-[2] min-w-0 flex flex-col gap-3">
      <DayStrip
        :dates="monthDates"
        :selectedDate="store.currentDate"
        :monthEntries="store.monthEntries"
        @selectDay="handleSelectDay"
      />

      <!-- DayNote -->
      <div
        ref="noteRef"
        class="text-xs text-gray-500 outline-none rounded px-3 py-1.5 bg-white border border-gray-200 hover:bg-gray-50 focus:bg-white focus:ring-2 focus:ring-blue-500 cursor-text min-h-[28px]"
        contenteditable="true"
        data-placeholder="Add a note…"
        @blur="saveNote"
        @paste="onNotePaste"
        @input="onNoteInput"
      ></div>

      <QuickEntry v-if="isSelectedToday" @appended="handleAppended" />

      <EntryList
        :entries="store.today?.entries || []"
        @update="(entryId, item, dur) => handleUpdateEntry(entryId, item, dur)"
        @delete="(entryId) => handleDeleteEntry(entryId)"
        @update-dimensions="(entryId, dims) => handleUpdateDimensions(entryId, dims)"
      />

      <!-- File path link -->
      <div v-if="store.rootPath" class="text-right">
        <button
          class="text-xs text-gray-400 hover:text-gray-600 transition-colors cursor-pointer"
          :title="store.rootPath + '/' + dayFilePath"
          @click="openInEditor"
        >
          {{ displayPath }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
[contenteditable]:empty::before {
  content: attr(data-placeholder);
  color: #cbd5e1;
}
</style>
