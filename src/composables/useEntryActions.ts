import { ref, onMounted, onUnmounted, inject, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppStore } from "../stores/useStore";
import type { Entry, DayFile, CommitmentProgress } from "../types";
import { UNDO_TOAST_KEY, SAVED_TOAST_KEY } from "../types";
import { logError } from "../utils/errorLog";
import { yearMonthFromDate } from "../utils/dates";
import { HIGHLIGHT_DURATION, UNDO_DELETE_DELAY } from "../utils/constants";

interface ComposerRef {
  clearInput(): void;
}

export function useEntryActions(store: AppStore, inputRef: Ref<ComposerRef | null>) {
  const triggerUndoToast = inject(UNDO_TOAST_KEY, (_undoFn: () => void) => {});
  const triggerSavedToast = inject(SAVED_TOAST_KEY, (_msg: string) => {});

  const justAddedId = ref<string | null>(null);
  let highlightTimer: ReturnType<typeof setTimeout> | null = null;
  let pendingDeleteTimer: ReturnType<typeof setTimeout> | null = null;
  const pendingDeleteInfo = ref<{ date: string; idx: number; entry: Entry } | null>(null);

  function sanitizeValues(vals: Record<string, string>): Record<string, string> {
    const validKeys = new Set(store.dimensions.map((d) => d.key));
    const cleaned: Record<string, string> = {};
    for (const [k, v] of Object.entries(vals)) if (validKeys.has(k) && v) cleaned[k] = v;
    return cleaned;
  }

  async function refreshProgress(date?: string) {
    const ym = yearMonthFromDate(date ?? store.currentDate);
    try {
      store.commitmentProgress = await invoke<CommitmentProgress[]>("get_commitment_progress", {
        rootPath: store.rootPath,
        year: ym.year,
        month: ym.month,
      });
    } catch (e) {
      logError("useEntryActions.refreshProgress", e);
      store.commitmentProgress = [];
      const msg = String(e);
      if (msg.includes("dimensions")) {
        store.configErrors = [{ kind: "ConfigError", message: msg }];
        store.configCategory = "in_place";
      }
    }
  }

  async function handleSubmit(item: string, durationMinutes: number, dimensions: Record<string, string>) {
    const finalDimensions = sanitizeValues(dimensions);
    const newEntry = { item, duration: String(durationMinutes) + "m", dimensions: finalDimensions };
    // Capture the target day up front: the user may navigate while the IPC is
    // in flight, and the result must land on the day it was submitted from.
    const date = store.currentDate;
    try {
      const result = await invoke("append_entry", {
        rootPath: store.rootPath,
        date,
        entry: newEntry,
      });
      const added = result as Entry;
      const entries = [...(store.monthEntries[date] ?? []), added];
      store.monthEntries[date] = entries;
      justAddedId.value = added.id;
      if (highlightTimer) clearTimeout(highlightTimer);
      highlightTimer = setTimeout(() => {
        justAddedId.value = null;
      }, HIGHLIGHT_DURATION);
      await refreshProgress();
      inputRef.value?.clearInput();
    } catch (e) {
      logError("useEntryActions.handleSubmit", e);
      triggerSavedToast("Failed to save entry");
    }
  }

  async function handleUpdateEntry(entryId: string, item: string, durationMinutes: number) {
    const date = store.currentDate; // capture: user may navigate during the IPC
    const entries = store.monthEntries[date];
    if (!entries) return;
    const entry = entries.find((e) => e.id === entryId);
    if (!entry) return;
    const update: Record<string, unknown> = {};
    if (item !== entry.item) update.item = item;
    if (durationMinutes !== entry.duration) update.duration = String(durationMinutes) + "m";
    if (Object.keys(update).length === 0) return;
    try {
      const df = await invoke<DayFile>("update_entry", {
        rootPath: store.rootPath,
        date,
        entryId,
        update,
      });
      store.monthEntries[date] = df.entries;
      store.dayNotes[date] = df.note;
      await refreshProgress();
      triggerSavedToast("Saved");
    } catch (e) {
      logError("useEntryActions.handleUpdateEntry", e);
      triggerSavedToast("Failed to save entry");
    }
  }

  async function handleUpdateDimensions(entryId: string, dimensions: Record<string, string>) {
    const date = store.currentDate; // capture: user may navigate during the IPC
    try {
      const df = await invoke<DayFile>("update_entry", {
        rootPath: store.rootPath,
        date,
        entryId,
        update: { dimensions },
      });
      store.monthEntries[date] = df.entries;
      store.dayNotes[date] = df.note;
      await refreshProgress();
      triggerSavedToast("Saved");
    } catch (e) {
      logError("useEntryActions.handleUpdateDimensions", e);
    }
  }

  async function handleDeleteEntry(entryId: string) {
    const entries = store.today?.entries;
    if (!entries) return;
    const idx = entries.findIndex((e) => e.id === entryId);
    if (idx === -1) return;
    const date = store.currentDate;
    const [removed] = entries.splice(idx, 1);
    let cancelled = false;
    pendingDeleteInfo.value = { date, idx, entry: removed };
    pendingDeleteTimer = setTimeout(async () => {
      pendingDeleteInfo.value = null;
      if (cancelled) return;
      try {
        const df = await invoke<DayFile>("delete_entry", {
          rootPath: store.rootPath,
          date,
          entryId,
        });
        if (store.currentDate === date) {
          store.dayNotes[date] = df.note;
        }
        store.monthEntries[date] = df.entries;
        await refreshProgress(date);
      } catch (e) {
        logError("useEntryActions.handleDeleteEntry", e);
        if (store.currentDate === date) {
          const curEntries = store.today?.entries;
          if (curEntries && curEntries.findIndex((e) => e.id === entryId) === -1) {
            curEntries.splice(idx, 0, removed);
          }
        }
      }
    }, UNDO_DELETE_DELAY);
    triggerUndoToast(() => {
      cancelled = true;
      if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
      if (store.currentDate === date) {
        const curEntries = store.today?.entries;
        if (curEntries && curEntries.findIndex((e) => e.id === entryId) === -1) {
          curEntries.splice(idx, 0, removed);
          store.monthEntries[date] = [...curEntries];
        }
      }
    });
  }

  function onBeforeUnload(e: BeforeUnloadEvent) {
    if (pendingDeleteTimer) {
      e.preventDefault();
    }
  }

  onMounted(() => {
    window.addEventListener("beforeunload", onBeforeUnload);
  });

  onUnmounted(() => {
    window.removeEventListener("beforeunload", onBeforeUnload);
    if (highlightTimer) clearTimeout(highlightTimer);
    if (pendingDeleteTimer) {
      clearTimeout(pendingDeleteTimer);
      const pd = pendingDeleteInfo.value;
      if (pd && store.currentDate === pd.date) {
        const curEntries = store.today?.entries;
        if (curEntries && curEntries.findIndex((e) => e.id === pd.entry.id) === -1) {
          curEntries.splice(pd.idx, 0, pd.entry);
          store.monthEntries[pd.date] = [...curEntries];
        }
      }
    }
    pendingDeleteInfo.value = null;
  });

  return { handleSubmit, handleUpdateEntry, handleUpdateDimensions, handleDeleteEntry, justAddedId };
}
