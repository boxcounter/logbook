import { invoke } from "@tauri-apps/api/core";
import type { AppStore, AvailableMonth } from "../stores/useStore";
import type { DayFile, Commitment, CommitmentProgress, MonthDimensions } from "../types";
import { logError } from "../utils/errorLog";
import { yearMonthFromDate } from "../utils/dates";

function isConfigError(msg: string): boolean {
  return /dimensions|commitments|\.yaml|corrupt|not configured|parse/i.test(msg);
}

export function useMonthData(store: AppStore, guardUnsaved: () => boolean) {
  async function loadMonth(year: number, month: number, defaultDay?: number) {
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
    store.today = { note: store.dayNotes[store.currentDate] ?? null, entries: store.monthEntries[store.currentDate] ?? [] };
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
    store.today = { note: store.today?.note ?? null, entries: store.monthEntries[store.currentDate] ?? [] };
  }

  async function handleSelectDay(dateStr: string) {
    if (!guardUnsaved()) return;
    store.currentDate = dateStr;
    store.today = { note: store.dayNotes[dateStr] ?? null, entries: store.monthEntries[dateStr] ?? [] };
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
