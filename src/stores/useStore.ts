import { reactive, inject, type InjectionKey } from "vue";
import type { Dimension, DayFile, Commitment, CommitmentProgress, CommitmentProgressResult, ConfigErrorDetail, AppStatus, Entry, RecoveryCategory } from "../types";

// 单个有记录的月份；数组见 AppStore.availableMonths。命名审查 2026-06-21 确认无需改名。
export interface AvailableMonth {
  year: number;
  month: number;
}

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
  commitmentProgressResult: CommitmentProgressResult | null;
  currentDate: string;
  monthEntries: Record<string, Entry[]>;
  availableMonths: AvailableMonth[] | null; // null = not yet loaded
}

export const STORE_KEY: InjectionKey<AppStore> = Symbol("AppStore");

export function createStore(): AppStore {
  const now = new Date();
  const dateStr = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;

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
    commitmentProgressResult: null,
    currentDate: dateStr,
    monthEntries: {},
    availableMonths: null,
  });
}

export function useStore(): AppStore {
  const store = inject(STORE_KEY);
  if (!store) throw new Error("AppStore not provided. Call provideStore() in root component.");
  return store;
}
