import { createStore, type AppStore } from "../../stores/useStore";

// ============================================================
// Test store factory. Wraps the real createStore() and allows
// overriding any property for a specific test scenario.
//
// Usage:
//   import { createTestStore } from "../mocks/store";
//   const store = createTestStore({ status: "ready", config: makeConfig() });
//
//   // Mount component with store provided:
//   mount(Component, {
//     global: { provide: { [STORE_KEY]: store } },
//   });
// ============================================================

export function createTestStore(overrides?: Partial<AppStore>): AppStore {
  const store = createStore();
  if (overrides) {
    Object.assign(store, overrides);
  }
  return store;
}
