import { createStore, type AppStore } from "../../stores/useStore";

// ============================================================
// Test store factory. Wraps the real createStore() and allows
// overriding any property for a specific test scenario.
//
// store.today is a derived computed (from currentDate + monthEntries +
// dayNotes), exactly as in production — there is NO writable today field.
// To set up "what the selected day shows", seed the source caches:
//   createTestStore({ currentDate: "2026-06-20", monthEntries: { "2026-06-20": [entry] } })
//
// Usage:
//   import { createTestStore } from "../mocks/store";
//   const store = createTestStore({ status: "ready", dimensions: makeDimensions() });
//
//   // Mount component with store provided:
//   mount(Component, {
//     global: { provide: { [STORE_KEY]: store } },
//   });
// ============================================================

export function createTestStore(overrides?: Partial<AppStore>): AppStore {
  const store = createStore();
  if (overrides) {
    if ("today" in overrides) {
      throw new Error(
        "store.today is derived and read-only; seed monthEntries/dayNotes/currentDate instead of overriding today",
      );
    }
    Object.assign(store, overrides);
  }
  return store;
}
