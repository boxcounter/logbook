import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import DataVersionScreen from "../../components/DataVersionScreen.vue";

// Use the REAL useRootFolderPicker composable; mock only the Tauri boundary
// (invoke + dialog) so the full pick → set_root_path → applyInitResult flow
// is exercised, same as production.
const { mockInvoke, mockOpen } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockOpen: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mockOpen }));
vi.mock("../../utils/errorLog", () => ({ logError: vi.fn() }));

const READY_RESULT = {
  status: "Ready",
  data: {
    root_path: "/new/path",
    dimensions: [],
    usingDefaultDimensions: true,
    today: { note: null, entries: [] },
    commitments: [],
    scan_warnings: [],
  },
};

const MESSAGE =
  "Data format version mismatch. Expected version 2, found version 1. Please run the Logbook migration tool to update your data directory.";

function mountScreen() {
  const store = createTestStore({
    status: "migration_needed",
    rootPath: "/old/path",
    dataVersionMessage: MESSAGE,
  });
  const wrapper = mount(DataVersionScreen, {
    props: { message: MESSAGE, rootPath: store.rootPath },
    global: { provide: { [STORE_KEY as symbol]: store } },
  });
  return { wrapper, store };
}

describe("DataVersionScreen", () => {
  beforeEach(() => vi.clearAllMocks());

  it("shows the migration guidance, the data dir, and a folder-picker exit", () => {
    const { wrapper } = mountScreen();
    expect(wrapper.text()).toContain("migration tool"); // existing guidance kept
    expect(wrapper.text()).toContain("/old/path");
    expect(wrapper.find('[data-testid="choose-folder"]').exists()).toBe(true);
  });

  it("Choose-folder runs the picker: dialog → set_root_path → Ready leaves the screen", async () => {
    mockOpen.mockResolvedValue("/new/path");
    mockInvoke.mockResolvedValue(READY_RESULT);
    const { wrapper, store } = mountScreen();

    await wrapper.get('[data-testid="choose-folder"]').trigger("click");
    await flushPromises();

    expect(mockOpen).toHaveBeenCalledWith(expect.objectContaining({ directory: true }));
    expect(mockInvoke).toHaveBeenCalledWith("set_root_path", { path: "/new/path" });
    expect(store.status).toBe("ready");
    expect(store.rootPath).toBe("/new/path");
  });

  it("dialog cancel does not call set_root_path", async () => {
    mockOpen.mockResolvedValue(null);
    const { wrapper, store } = mountScreen();

    await wrapper.get('[data-testid="choose-folder"]').trigger("click");
    await flushPromises();

    expect(mockInvoke).not.toHaveBeenCalled();
    expect(store.status).toBe("migration_needed");
  });

  it("set_root_path returning a version error again keeps the screen with a refreshed message", async () => {
    // Backend may reject the newly picked folder with a version error instead
    // of stamping it; the screen must stay up with the new message/rootPath.
    mockOpen.mockResolvedValue("/older/path");
    mockInvoke.mockResolvedValue({
      status: "DataVersionMismatch",
      data: { root_path: "/older/path", expected: 2, found: 1 },
    });
    const { wrapper, store } = mountScreen();

    await wrapper.get('[data-testid="choose-folder"]').trigger("click");
    await flushPromises();

    expect(store.status).toBe("migration_needed");
    expect(store.rootPath).toBe("/older/path");
    expect(store.dataVersionMessage).toContain("Expected version 2, found version 1");
  });
});
