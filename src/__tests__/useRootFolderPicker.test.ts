import { describe, it, expect, vi, beforeEach } from "vitest";

const { mockInvoke, mockOpen } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockOpen: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mockOpen }));
vi.mock("../utils/errorLog", () => ({ logError: vi.fn() }));

import { useRootFolderPicker } from "../composables/useRootFolderPicker";
import { createStore } from "../stores/useStore";

describe("useRootFolderPicker", () => {
  beforeEach(() => vi.clearAllMocks());

  it("cancel dialog → no invoke, store untouched", async () => {
    mockOpen.mockResolvedValue(null);
    const store = createStore();
    const { pick } = useRootFolderPicker(store);
    await pick();
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(store.status).toBe("loading");
  });

  it("pick → set_root_path Ready maps to store", async () => {
    mockOpen.mockResolvedValue("/data/logbook");
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: {
        root_path: "/data/logbook",
        dimensions: [{ name: "Goal", key: "goal", source: "monthly", required: false, deleted: false }],
        from_template: true,
        today: { note: null, entries: [] },
        commitments: [],
        scan_warnings: [],
      },
    });
    const store = createStore();
    const { pick } = useRootFolderPicker(store);
    await pick();
    expect(mockInvoke).toHaveBeenCalledWith("set_root_path", { path: "/data/logbook" });
    expect(store.status).toBe("ready");
    expect(store.rootPath).toBe("/data/logbook");
    expect(store.fromTemplate).toBe(true);
  });

  it("pick → set_root_path ConfigError(config_missing) routes to error", async () => {
    mockOpen.mockResolvedValue("/data/logbook");
    mockInvoke.mockResolvedValue({
      status: "ConfigError",
      data: { category: "config_missing", root_path: "/data/logbook", errors: [{ kind: "ConfigMissing", message: "no template" }], scan_warnings: [] },
    });
    const store = createStore();
    const { pick } = useRootFolderPicker(store);
    await pick();
    expect(store.status).toBe("error");
    expect(store.configCategory).toBe("config_missing");
  });

  it("invoke throws → error state with SetupError", async () => {
    mockOpen.mockResolvedValue("/data/logbook");
    mockInvoke.mockRejectedValue("boom");
    const store = createStore();
    const { pick } = useRootFolderPicker(store);
    await pick();
    expect(store.status).toBe("error");
    expect(store.configErrors[0].kind).toBe("SetupError");
  });
});
