<script setup lang="ts">
import { inject } from "vue";
import { useStore } from "../stores/useStore";
import { invoke } from "@tauri-apps/api/core";
import DateNavigator from "./DateNavigator.vue";
import CommitmentsPanel from "./CommitmentsPanel.vue";
import QuickEntry from "./QuickEntry.vue";
import EntryList from "./EntryList.vue";
import SummaryBar from "./SummaryBar.vue";
import type { Entry, DayFile } from "../types";
import { logError, logInfo } from "../utils/errorLog";
import { computed } from "vue";
import { datesInPeriod } from "../utils/dates";

async function loadPeriod() {
  const dates = datesInPeriod(store.currentDate, store.granularity);
  const map: Record<string, Entry[]> = {};
  for (const date of dates) {
    try {
      const df = (await invoke("get_entries", { rootPath: store.rootPath, date })) as DayFile;
      map[date] = df.entries;
    } catch (e) {
      logError("TodayView.loadPeriod", e);
      map[date] = [];
    }
  }
  store.periodEntries = map;
  if (store.currentDate in map) {
    store.today = { note: null, entries: map[store.currentDate] };
  }
}

const store = useStore();

// Inject undo toast trigger from App.vue
const triggerUndoToast = inject<(undoFn: () => void) => void>("triggerUndoToast", () => {});

async function handleUpdateDimensions(entryId: string, dimensions: Record<string, string>) {
  logInfo("TodayView.handleUpdateDimensions", `id=${entryId} dims=${JSON.stringify(dimensions)}`);
  try {
    const df = (await invoke("update_entry", {
      rootPath: store.rootPath,
      date: store.currentDate,
      entryId,
      update: { dimensions },
    })) as DayFile;
    store.today = df;
  } catch (e) {
    logError("TodayView.handleUpdateDimensions", e);
  }
}

async function handleUpdateEntry(entryId: string, item: string, durationMinutes: number) {
  const entry = store.today?.entries.find(e => e.id === entryId);
  if (!entry) return;

  // Only send changed fields
  const update: Record<string, unknown> = {};
  if (item !== entry.item) update.item = item;
  if (durationMinutes !== entry.duration) update.duration = String(durationMinutes);

  if (Object.keys(update).length === 0) return;

  logInfo("TodayView.handleUpdateEntry", `id=${entryId} fields=${Object.keys(update).join(",")}`);
  try {
    const df = (await invoke("update_entry", {
      rootPath: store.rootPath,
      date: store.currentDate,
      entryId,
      update,
    })) as DayFile;
    store.today = df;
  } catch (e) {
    logError("TodayView.handleUpdateEntry", e);
  }
}

let pendingDeleteTimer: ReturnType<typeof setTimeout> | null = null;

async function handleDeleteEntry(entryId: string) {
  const entries = store.today?.entries;
  if (!entries) return;

  // Optimistic: remove from UI immediately
  const idx = entries.findIndex(e => e.id === entryId);
  if (idx === -1) return;
  const [removed] = entries.splice(idx, 1);

  // Schedule persistence
  let cancelled = false;
  pendingDeleteTimer = setTimeout(async () => {
    if (cancelled) return;
    try {
      await invoke("delete_entry", { rootPath: store.rootPath, date: store.currentDate, entryId });
    } catch (e) {
      logError("TodayView.handleDeleteEntry", e);
      // Re-insert at original position if still valid
      const currentIdx = entries.findIndex(e => e.id === entryId);
      if (currentIdx === -1) {
        entries.splice(idx, 0, removed);
      }
    }
  }, 5000);

  // Show undo toast
  triggerUndoToast(() => {
    cancelled = true;
    if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
    const currentIdx = entries.findIndex(e => e.id === entryId);
    if (currentIdx === -1) {
      entries.splice(idx, 0, removed);
    }
  });
}

// #5: file path + open in system editor
const dayFilePath = computed(() => {
  if (!store.rootPath) return "";
  const d = store.currentDate;
  const year = d.slice(0, 4);
  const month = d.slice(5, 7);
  return `${year}/${month}/${d}.md`;
});

const displayPath = computed(() => {
  if (!store.rootPath) return "";
  // Show last 3 segments: …/year/month/file.md
  return `…/${dayFilePath.value}`;
});

async function openInEditor() {
  if (!store.rootPath) return;
  try {
    await invoke("open_in_editor", { rootPath: store.rootPath, date: store.currentDate });
  } catch (e) {
    logError("TodayView.openInEditor", e);
  }
}

// Only show QuickEntry on today's date
const isToday = (): boolean => {
  const now = new Date();
  const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  return store.currentDate === today;
};
</script>

<template>
  <div class="flex gap-4 p-4 max-w-7xl mx-auto items-start">
    <!-- Left 2/3 -->
    <div class="flex-[2] min-w-0 flex flex-col gap-3">
      <DateNavigator @navigate="loadPeriod" />
      <QuickEntry v-if="isToday()" @appended="loadPeriod" />
      <EntryList
        :entries="store.today?.entries || []"
        :granularity="store.granularity"
        :periodEntries="store.periodEntries"
        @update="(entryId, item, dur) => handleUpdateEntry(entryId, item, dur)"
        @delete="(entryId) => handleDeleteEntry(entryId)"
        @update-dimensions="(entryId, dims) => handleUpdateDimensions(entryId, dims)"
      />
      <SummaryBar
        :entries="store.today?.entries || []"
        :granularity="store.granularity"
        :periodEntries="store.periodEntries"
      />
      <!-- #5: file path → click to open in system editor -->
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
    <!-- Right 1/3 -->
    <div class="flex-1 min-w-[180px] flex flex-col gap-3 sticky top-4">
      <CommitmentsPanel
        :commitments="store.commitments"
        :entries="store.today?.entries || []"
      />
    </div>
  </div>
</template>
