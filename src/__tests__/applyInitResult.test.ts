import { describe, it, expect } from "vitest";
import { applyInitResult } from "../utils/applyInitResult";
import { createStore } from "../stores/useStore";
import type { InitResult } from "../types";

describe("applyInitResult", () => {
  it("NeedsSetup → status setup", () => {
    const store = createStore();
    const warnings = applyInitResult(store, { status: "NeedsSetup" });
    expect(store.status).toBe("setup");
    expect(warnings).toEqual([]);
  });

  it("ConfigError → status error, category + rootPath + errors set", () => {
    const store = createStore();
    const result: InitResult = {
      status: "ConfigError",
      data: {
        category: "root_missing",
        root_path: "/data/logbook",
        errors: [{ kind: "RootMissing", message: "gone" }],
        scan_warnings: [{ kind: "OrphanedTemp", path: "x.tmp", message: "t" }],
      },
    };
    const warnings = applyInitResult(store, result);
    expect(store.status).toBe("error");
    expect(store.configCategory).toBe("root_missing");
    expect(store.rootPath).toBe("/data/logbook");
    expect(store.configErrors).toEqual([{ kind: "RootMissing", message: "gone" }]);
    expect(warnings).toHaveLength(1);
  });

  it("Ready → status ready, dimensions + fromTemplate set, category cleared", () => {
    const store = createStore();
    store.configCategory = "in_place";
    const result: InitResult = {
      status: "Ready",
      data: {
        root_path: "/data/logbook",
        dimensions: [{ name: "Goal", key: "goal", source: "monthly", required: false, deleted: false }],
        from_template: true,
        today: { note: null, entries: [] },
        commitments: [],
        scan_warnings: [],
      },
    };
    applyInitResult(store, result);
    expect(store.status).toBe("ready");
    expect(store.rootPath).toBe("/data/logbook");
    expect(store.dimensions).toHaveLength(1);
    expect(store.fromTemplate).toBe(true);
    expect(store.configCategory).toBeNull();
  });
});
