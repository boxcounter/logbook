import { reactive, computed, inject, type InjectionKey } from "vue";
import type { Dimension, DayFile, Commitment, CommitmentProgress, ConfigErrorDetail, AppStatus, Entry, RecoveryCategory, IntegrityIssue } from "../types";
import { formatDate } from "../utils/dates";

// 单个有记录的月份；数组见 AppStore.availableMonths。命名审查 2026-06-21 确认无需改名。
export interface AvailableMonth {
  year: number;
  month: number;
}

// Reactive store shared via provide/inject. store.today is a COMPUTED view
// derived from currentDate + monthEntries + dayNotes — those three are the
// single source of truth; never assign store.today directly. Other fields
// (commitments, etc.) are written by composables in sequential user actions —
// no concurrent access. Concurrent watcher-triggered reloads and user edits
// are guarded by ad-hoc currentDate checks at call sites.
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

  const store = reactive({
    status: "loading" as AppStatus,
    rootPath: "",
    dimensions: [] as Dimension[],
    usingDefaultDimensions: false,
    configErrors: [] as ConfigErrorDetail[],
    configCategory: null as RecoveryCategory | null,
    dataVersionMessage: null as string | null,
    commitments: [] as Commitment[],
    commitmentProgress: [] as CommitmentProgress[],
    currentDate: dateStr,
    monthEntries: {} as Record<string, Entry[]>,
    dayNotes: {} as Record<string, string | null>,
    availableMonths: null as AvailableMonth[] | null,
    integrityIssues: [] as IntegrityIssue[],
    // Derived view of the selected day. null until any month data has been
    // loaded (matches the pre-derivation "initial null"); once the caches hold
    // anything, a day with no day-file reads as an empty DayFile ({note: null,
    // entries: []}) — the same shape loadMonth/handleSelectDay used to build.
    today: computed((): DayFile | null => {
      if (Object.keys(store.monthEntries).length === 0 && Object.keys(store.dayNotes).length === 0) return null;
      const d = store.currentDate;
      return { note: store.dayNotes[d] ?? null, entries: store.monthEntries[d] ?? [] };
    }),
  });
  return store;
}

export function useStore(): AppStore {
  const store = inject(STORE_KEY);
  if (!store) throw new Error("AppStore not provided. Call provideStore() in root component.");
  return store;
}
