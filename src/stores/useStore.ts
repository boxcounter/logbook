import { reactive, inject, provide, type InjectionKey } from "vue";
import type { Config, DayFile, Commitment, CommitmentProgress, ConfigErrorDetail, Screen, Entry } from "../types";

export interface AvailableMonth {
  year: number;
  month: number;
}

export interface AppStore {
  screen: Screen;
  rootPath: string;
  config: Config | null;
  configErrors: ConfigErrorDetail[];
  today: DayFile | null;
  commitments: Commitment[];
  commitmentProgress: CommitmentProgress[];
  lastDimensions: Record<string, string>;
  currentDate: string;
  monthEntries: Record<string, Entry[]>;
  availableMonths: AvailableMonth[] | null; // null = not yet loaded
}

export const STORE_KEY: InjectionKey<AppStore> = Symbol("AppStore");

export function createStore(): AppStore {
  const now = new Date();
  const dateStr = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;

  return reactive<AppStore>({
    screen: "loading",
    rootPath: "",
    config: null,
    configErrors: [],
    today: null,
    commitments: [],
    commitmentProgress: [],
    lastDimensions: {},
    currentDate: dateStr,
    monthEntries: {},
    availableMonths: null,
  });
}

export function provideStore(store: AppStore): void {
  provide(STORE_KEY, store);
}

export function useStore(): AppStore {
  const store = inject(STORE_KEY);
  if (!store) throw new Error("AppStore not provided. Call provideStore() in root component.");
  return store;
}
