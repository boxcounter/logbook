import { reactive, inject, type InjectionKey } from "vue";
import type { Dimension, DayFile, Commitment, CommitmentProgress, ConfigErrorDetail, AppStatus, Entry, RecoveryCategory, IntegrityIssue } from "../types";
import { formatDate } from "../utils/dates";

// 单个有记录的月份；数组见 AppStore.availableMonths。命名审查 2026-06-21 确认无需改名。
export interface AvailableMonth {
  year: number;
  month: number;
}

// Reactive store shared via provide/inject. Multiple composables write to
// store.today, store.monthEntries, store.commitments, etc. Writes are
// sequential user actions — no concurrent access. Concurrent watcher-triggered
// reloads and user edits are guarded by ad-hoc currentDate checks at call sites.
// A unified write mediator would add ceremony for a single-user desktop app.
export interface AppStore {
  status: AppStatus;
  rootPath: string;
  dimensions: Dimension[];
  usingDefaultDimensions: boolean;
  configErrors: ConfigErrorDetail[];
  configCategory: RecoveryCategory | null;
  dataVersionMessage: string | null;
  today: DayFile | null;
  commitments: Commitment[];
  commitmentProgress: CommitmentProgress[];
  currentDate: string;
  monthEntries: Record<string, Entry[]>;
  dayNotes: Record<string, string | null>;
  availableMonths: AvailableMonth[] | null; // null = not yet loaded
  integrityIssues: IntegrityIssue[];
}

export const STORE_KEY: InjectionKey<AppStore> = Symbol("AppStore");

export function createStore(): AppStore {
  const dateStr = formatDate(new Date());

  return reactive<AppStore>({
    status: "loading",
    rootPath: "",
    dimensions: [],
    usingDefaultDimensions: false,
    configErrors: [],
    configCategory: null,
    dataVersionMessage: null,
    today: null,
    commitments: [],
    commitmentProgress: [],
    currentDate: dateStr,
    monthEntries: {},
    dayNotes: {},
    availableMonths: null,
    integrityIssues: [],
  });
}

export function useStore(): AppStore {
  const store = inject(STORE_KEY);
  if (!store) throw new Error("AppStore not provided. Call provideStore() in root component.");
  return store;
}
