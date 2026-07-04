import { invoke } from "@tauri-apps/api/core";
import type { AppStore } from "../stores/useStore";
import type { Entry, DayFile, Commitment, CommitmentProgressResult, MonthDimensions } from "../types";
import { logError } from "../utils/errorLog";
import { yearMonthFromDate } from "../utils/dates";

export function useMonthData(store: AppStore, guardUnsaved: () => boolean) {
  async function loadMonth(year: number, month: number, defaultDay?: number) {
    store.configErrors = [];
    store.commitments = [];
    store.commitmentProgress = [];
    store.commitmentProgressResult = null;
    const now = new Date();
    const isCurrentMonth = year === now.getFullYear() && month === now.getMonth() + 1;
    let day: number;
    if (defaultDay !== undefined) day = defaultDay;
    else if (isCurrentMonth) day = now.getDate();
    else day = new Date(year, month, 0).getDate();

    const dateStr = `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
    store.currentDate = dateStr;

    try {
      store.monthEntries = await invoke<Record<string, Entry[]>>("get_month_entries", { rootPath: store.rootPath, year, month });
    } catch (e) {
      logError("useMonthData.loadMonth", e);
      store.monthEntries = {};
      const msg = String(e);
      if (msg.includes("dimensions")) {
        store.configErrors = [{ kind: "ConfigError", message: msg }];
        store.configCategory = "in_place";
        return;
      }
    }
    await loadCommitmentProgress(year, month);
    await loadCommitments(year, month);
    await loadMonthDimensions(year, month);
    if (store.currentDate in store.monthEntries) {
      store.today = { note: null, entries: store.monthEntries[store.currentDate] };
      loadDayNote(store.currentDate);
    }
  }

  async function loadCommitmentProgress(year: number, month: number) {
    try {
      const result = await invoke<CommitmentProgressResult>("get_commitment_progress", { rootPath: store.rootPath, year, month });
      store.commitmentProgress = result.roles;
      store.commitmentProgressResult = result;
    } catch (e) {
      logError("useMonthData.loadCommitmentProgress", e);
      store.commitmentProgress = [];
      store.commitmentProgressResult = null;
      const msg = String(e);
      if (msg.includes("dimensions")) {
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
      if (msg.includes("dimensions")) {
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
  }

  async function loadDayNote(dateStr: string) {
    try {
      const df = await invoke<DayFile>("get_entries", { rootPath: store.rootPath, date: dateStr });
      if (store.today) store.today.note = df.note;
    } catch (e) { logError("useMonthData.loadDayNote", e); }
  }

  async function handleSelectDay(dateStr: string) {
    if (!guardUnsaved()) return;
    store.currentDate = dateStr;
    if (dateStr in store.monthEntries) {
      store.today = { note: null, entries: store.monthEntries[dateStr] };
      await loadDayNote(dateStr);
    }
  }

  async function handleNavigate({ year, month }: { year: number; month: number }) {
    if (!guardUnsaved()) return;
    await loadMonth(year, month);
  }

  async function handleRequestMonths() {
    if (store.availableMonths !== null) return;
    try {
      store.availableMonths = (await invoke("get_available_months", { rootPath: store.rootPath })) as { year: number; month: number }[];
    } catch (e) { logError("useMonthData.handleRequestMonths", e); store.availableMonths = []; }
  }

  return {
    loadMonth,
    loadCommitmentProgress,
    loadCommitments,
    loadMonthDimensions,
    onCommitmentsSaved,
    loadDayNote,
    handleSelectDay,
    handleNavigate,
    handleRequestMonths,
  };
}
