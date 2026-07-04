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
  const triggerUndoToast = inject(UNDO_TOAST_KEY, () => {});
  const triggerSavedToast = inject(SAVED_TOAST_KEY, () => {});

  const justAddedId = ref<string | null>(null);
  let highlightTimer: ReturnType<typeof setTimeout> | null = null;
  let pendingDeleteTimer: ReturnType<typeof setTimeout> | null = null;

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
    try {
      const result = await invoke("append_entry", {
        rootPath: store.rootPath,
        date: store.currentDate,
        entry: newEntry,
      });
      const added = result as Entry;
      if (store.today) {
        const entries = [...store.today.entries, added];
        store.today = { ...store.today, entries };
        store.monthEntries[store.currentDate] = entries;
      }
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
    const entries = store.today?.entries;
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
        date: store.currentDate,
        entryId,
        update,
      });
      store.today = df;
      store.monthEntries[store.currentDate] = df.entries;
      await refreshProgress();
      triggerSavedToast("Saved");
    } catch (e) {
      logError("useEntryActions.handleUpdateEntry", e);
      triggerSavedToast("Failed to save entry");
    }
  }

  async function handleUpdateDimensions(entryId: string, dimensions: Record<string, string>) {
    try {
      const df = await invoke<DayFile>("update_entry", {
        rootPath: store.rootPath,
        date: store.currentDate,
        entryId,
        update: { dimensions },
      });
      store.today = df;
      store.monthEntries[store.currentDate] = df.entries;
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
    pendingDeleteTimer = setTimeout(async () => {
      if (cancelled) return;
      try {
        const df = await invoke<DayFile>("delete_entry", {
          rootPath: store.rootPath,
          date,
          entryId,
        });
        store.today = df;
        store.monthEntries[date] = df.entries;
        await refreshProgress(date);
      } catch (e) {
        logError("useEntryActions.handleDeleteEntry", e);
        const curEntries = store.today?.entries;
        if (curEntries && curEntries.findIndex((e) => e.id === entryId) === -1) {
          curEntries.splice(idx, 0, removed);
        }
      }
    }, UNDO_DELETE_DELAY);
    triggerUndoToast(() => {
      cancelled = true;
      if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
      const curEntries = store.today?.entries;
      if (curEntries && curEntries.findIndex((e) => e.id === entryId) === -1) {
        curEntries.splice(idx, 0, removed);
        store.monthEntries[date] = [...curEntries];
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
    if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
  });

  return { handleSubmit, handleUpdateEntry, handleUpdateDimensions, handleDeleteEntry, justAddedId };
}
