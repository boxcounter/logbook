import { reactive, inject, provide, type InjectionKey } from "vue";
import type { Config, DayFile, Commitment, ConfigErrorDetail, Screen, Granularity, Entry } from "../types";

export interface AppStore {
  screen: Screen;
  rootPath: string;
  config: Config | null;
  configErrors: ConfigErrorDetail[];
  today: DayFile | null;
  commitments: Commitment[];
  lastDimensions: Record<string, string>;
  currentDate: string;
  granularity: Granularity;
  periodEntries: Record<string, Entry[]>;
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
    lastDimensions: {},
    currentDate: dateStr,
    granularity: "day",
    periodEntries: {},
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
