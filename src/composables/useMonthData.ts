import { invoke } from "@tauri-apps/api/core";
import { watch } from "vue";
import type { AppStore, AvailableMonth } from "../stores/useStore";
import type { DayFile, Commitment, CommitmentProgress, MonthDimensions } from "../types";
import { logError } from "../utils/errorLog";
import { yearMonthFromDate, formatDate } from "../utils/dates";

function isConfigError(msg: string): boolean {
  return /dimensions|commitments|\.yaml|corrupt|not configured|parse/i.test(msg);
}

export function useMonthData(store: AppStore, guardUnsaved: () => boolean) {
  // The month currently held by the monthEntries/dayNotes caches. Set at the
  // START of loadMonth so the rollover watch below doesn't re-enter mid-load.
  let loadedYear: number | null = null;
  let loadedMonth: number | null = null;

  async function loadMonth(year: number, month: number, defaultDay?: number) {
    loadedYear = year;
    loadedMonth = month;
    store.configErrors = [];
    store.commitments = [];
    store.commitmentProgress = [];
    const now = new Date();
    const isCurrentMonth = year === now.getFullYear() && month === now.getMonth() + 1;
    let day: number;
    if (defaultDay !== undefined) day = defaultDay;
    else if (isCurrentMonth) day = now.getDate();
    else day = new Date(year, month, 0).getDate();

    const dateStr = `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
    store.currentDate = dateStr;

    try {
      const monthDays = await invoke<Record<string, DayFile>>("get_month_entries", { rootPath: store.rootPath, year, month });
      store.monthEntries = {};
      store.dayNotes = {};
      for (const [d, df] of Object.entries(monthDays)) {
        store.monthEntries[d] = df.entries;
        store.dayNotes[d] = df.note;
      }
    } catch (e) {
      logError("useMonthData.loadMonth", e);
      store.monthEntries = {};
      store.dayNotes = {};
      const msg = String(e);
      if (isConfigError(msg)) {
        store.configErrors = [{ kind: "ConfigError", message: msg }];
        store.configCategory = "in_place";
      }
    }
    await loadCommitmentProgress(year, month);
    await loadCommitments(year, month);
    await loadMonthDimensions(year, month);
  }

  async function loadCommitmentProgress(year: number, month: number) {
    try {
      store.commitmentProgress = await invoke<CommitmentProgress[]>("get_commitment_progress", { rootPath: store.rootPath, year, month });
    } catch (e) {
      logError("useMonthData.loadCommitmentProgress", e);
      store.commitmentProgress = [];
      const msg = String(e);
      if (isConfigError(msg)) {
        store.configErrors = [{ kind: "ConfigError", message: msg }];
        store.configCategory = "in_place";
      }
    }
  }

  async function loadCommitments(year: number, month: number) {
    try {
      store.commitments = await invoke<Commitment[]>("get_commitments", { rootPath: store.rootPath, year, month });
    } catch (e) { logError("useMonthData.loadCommitments", e); store.commitments = []; }
  }

  async function loadMonthDimensions(year: number, month: number) {
    try {
      const md = await invoke<MonthDimensions>("get_month_dimensions", { rootPath: store.rootPath, year, month });
      if (md && Array.isArray(md.dimensions)) {
        store.dimensions = md.dimensions;
        store.usingDefaultDimensions = md.usingDefaultDimensions;
      }
    } catch (e) {
      logError("useMonthData.loadMonthDimensions", e);
      const msg = String(e);
      if (isConfigError(msg)) {
        store.configErrors = [{ kind: "ConfigError", message: msg }];
        store.configCategory = "in_place";
      }
    }
  }

  async function onCommitmentsSaved(commitments: Commitment[]) {
    store.commitments = commitments;
    await loadCommitmentProgress(
      yearMonthFromDate(store.currentDate).year,
      yearMonthFromDate(store.currentDate).month,
    );
    await reloadMonthEntries(
      yearMonthFromDate(store.currentDate).year,
      yearMonthFromDate(store.currentDate).month,
    );
  }

  async function reloadMonthEntries(year: number, month: number) {
    try {
      const monthDays = await invoke<Record<string, DayFile>>("get_month_entries", { rootPath: store.rootPath, year, month });
      store.monthEntries = {};
      for (const [d, df] of Object.entries(monthDays)) {
        store.monthEntries[d] = df.entries;
      }
    } catch (e) {
      logError("useMonthData.reloadMonthEntries", e);
    }
    // Only entries are reloaded here (commitments edits don't touch notes);
    // store.today re-derives from monthEntries + the untouched dayNotes.
  }

  async function handleSelectDay(dateStr: string) {
    if (!guardUnsaved()) return;
    store.currentDate = dateStr;
  }

  async function handleNavigate({ year, month }: { year: number; month: number }) {
    if (!guardUnsaved()) return;
    await loadMonth(year, month);
  }

  async function handleRequestMonths() {
    if (store.availableMonths !== null) return;
    try {
      store.availableMonths = await invoke<AvailableMonth[]>("get_available_months", { rootPath: store.rootPath });
    } catch (e) { logError("useMonthData.handleRequestMonths", e); store.availableMonths = []; }
  }

  // Midnight rollover (App.maybeRollover) advances store.currentDate directly,
  // without going through loadMonth. When that crosses a month boundary, the
  // caches still hold the OLD month's keys and the new month renders empty
  // until a manual month switch. Detect it here and load the new month through
  // the normal channel. Guarded to the real today: manual currentDate writes
  // (navigation) already go through loadMonth and must not trigger reloads.
  watch(
    () => store.currentDate.slice(0, 7),
    () => {
      const todayStr = formatDate(new Date());
      if (store.currentDate !== todayStr) return;
      const { year, month } = yearMonthFromDate(todayStr);
      if (year === loadedYear && month === loadedMonth) return;
      loadMonth(year, month);
    },
  );

  return {
    loadMonth,
    loadCommitmentProgress,
    loadCommitments,
    loadMonthDimensions,
    onCommitmentsSaved,
    handleSelectDay,
    handleNavigate,
    handleRequestMonths,
  };
}
