import { describe, it, expect } from "vitest";
import { createStore } from "../stores/useStore";
import { formatDate } from "../utils/dates";

describe("createStore", () => {
  it("defaults", () => {
    const store = createStore();
    expect(store.status).toBe("loading");
    expect(store.rootPath).toBe("");
    expect(store.dimensions).toEqual([]);
    expect(store.today).toBeNull();
    expect(store.commitments).toEqual([]);
    expect(store.commitmentProgress).toEqual([]);
    expect("lastDimensions" in store).toBe(false);
    expect(store.monthEntries).toEqual({});
    expect(store.availableMonths).toBeNull();
  });

  it("createStore 默认 configCategory 为 null", () => {
    const store = createStore();
    expect(store.configCategory).toBeNull();
  });

  it("currentDate is today", () => {
    const store = createStore();
    const expected = formatDate(new Date());
    expect(store.currentDate).toBe(expected);
  });
});
