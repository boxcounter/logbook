<script setup lang="ts">
import { inject } from "vue";
import { useStore } from "../stores/useStore";
import { invoke } from "@tauri-apps/api/core";
import DateNavigator from "./DateNavigator.vue";
import CommitmentsPanel from "./CommitmentsPanel.vue";
import QuickEntry from "./QuickEntry.vue";
import EntryList from "./EntryList.vue";
import SummaryBar from "./SummaryBar.vue";
import type { Granularity, Entry, DayFile } from "../types";
import { logError } from "../utils/errorLog";

function formatDate(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function datesInPeriod(dateStr: string, granularity: Granularity): string[] {
  const d = new Date(dateStr + "T00:00:00");
  const dates: string[] = [];
  if (granularity === "day") {
    dates.push(dateStr);
  } else if (granularity === "week") {
    const day = d.getDay();
    const monday = new Date(d);
    monday.setDate(d.getDate() - (day === 0 ? 6 : day - 1));
    for (let i = 0; i < 7; i++) {
      const dt = new Date(monday);
      dt.setDate(monday.getDate() + i);
      dates.push(formatDate(dt));
    }
  } else {
    const year = d.getFullYear();
    const month = d.getMonth();
    const lastDay = new Date(year, month + 1, 0).getDate();
    for (let day = 1; day <= lastDay; day++) {
      dates.push(`${year}-${String(month + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`);
    }
  }
  return dates;
}

async function loadPeriod() {
  const dates = datesInPeriod(store.currentDate, store.granularity);
  const map: Record<string, Entry[]> = {};
  for (const date of dates) {
    try {
      const df = (await invoke("get_entries", { rootPath: store.rootPath, date })) as DayFile;
      map[date] = df.entries;
    } catch (e) {
      console.error("loadPeriod failed for", date, e);
      map[date] = [];
    }
  }
  store.periodEntries = map;
  store.today = { note: null, entries: map[store.currentDate] || [] };
}

const store = useStore();

// Inject undo toast trigger from App.vue
const triggerUndoToast = inject<(undoFn: () => void) => void>("triggerUndoToast", () => {});

async function handleUpdateEntry(entryId: string, item: string, durationMinutes: number) {
  const entry = store.today?.entries.find(e => e.id === entryId);
  if (!entry) return;

  // Only send changed fields
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
  } catch (e) {
    logError("TodayView.handleUpdateEntry", e);
    console.error("update_entry failed:", e);
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
      console.error("delete_entry failed:", e);
      entries.splice(idx, 0, removed); // restore on failure
    }
  }, 5000);

  // Show undo toast
  triggerUndoToast(() => {
    cancelled = true;
    if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
    entries.splice(idx, 0, removed);
  });
}

// Only show QuickEntry on today's date
const isToday = (): boolean => {
  const now = new Date();
  const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  return store.currentDate === today;
};
</script>

<template>
  <div class="flex gap-4 p-4 max-w-4xl mx-auto items-start">
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
      />
      <SummaryBar
        :entries="store.today?.entries || []"
        :granularity="store.granularity"
        :periodEntries="store.periodEntries"
      />
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
