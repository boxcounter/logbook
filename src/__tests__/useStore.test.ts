import { describe, it, expect } from "vitest";
import { createStore } from "../stores/useStore";

describe("createStore", () => {
  it("defaults", () => {
    const store = createStore();
    expect(store.status).toBe("loading");
    expect(store.rootPath).toBe("");
    expect(store.config).toBeNull();
    expect(store.today).toBeNull();
    expect(store.commitments).toEqual([]);
    expect(store.commitmentProgress).toEqual([]);
    expect("lastDimensions" in store).toBe(false);
    expect(store.monthEntries).toEqual({});
    expect(store.availableMonths).toBeNull();
  });

  it("currentDate is today", () => {
    const store = createStore();
    const now = new Date();
    const expected = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
    expect(store.currentDate).toBe(expected);
  });
});
