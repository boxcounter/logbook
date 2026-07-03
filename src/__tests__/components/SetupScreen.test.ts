import { describe, it, expect, vi, beforeEach} from "vitest";
import { mount } from "@vue/test-utils";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import { makeDimensions, makeCommitment } from "../mocks/fixtures";
import SetupScreen from "../../components/SetupScreen.vue";

// ============================================================
// SetupScreen uses invoke(), listen(), open() from Tauri packages.
// We mock them at the module level (Vitest hoists vi.mock).
// ============================================================

const { mockInvoke, mockOpen, mockLogError } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockOpen: vi.fn(),
  mockLogError: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mockOpen }));
vi.mock("../../utils/errorLog", () => ({ logError: mockLogError }));

function mountSetup(storeOverrides?: Parameters<typeof createTestStore>[0]) {
  const store = createTestStore(storeOverrides);
  const wrapper = mount(SetupScreen, {
    global: {
      provide: { [STORE_KEY as symbol]: store },
      stubs: { transition: true },
    },
  });
  return { wrapper, store };
}

// ============================================================

describe("SetupScreen", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockOpen.mockResolvedValue(null); // user cancels by default
  });

  it("renders welcome heading and description", () => {
    const { wrapper } = mountSetup();
    expect(wrapper.text()).toContain("Welcome to Logbook");
    expect(wrapper.text()).toContain("Choose a folder to store your data");
  });

  it('renders "Choose Data Folder" button', () => {
    const { wrapper } = mountSetup();
    const btn = wrapper.find("button");
    expect(btn.exists()).toBe(true);
    expect(btn.text()).toContain("Choose Data Folder");
  });

  it("click button: calls open dialog with directory option", async () => {
    const { wrapper } = mountSetup();
    await wrapper.find("button").trigger("click");
    expect(mockOpen).toHaveBeenCalledWith({
      directory: true,
      multiple: false,
      title: "Select Logbook data folder",
    });
  });

  it("dialog cancelled: does not call invoke", async () => {
    mockOpen.mockResolvedValue(null);
    const { wrapper } = mountSetup();
    await wrapper.find("button").trigger("click");
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("dialog selected: calls invoke set_root_path", async () => {
    mockOpen.mockResolvedValue("/Users/test/logbook-data");
    mockInvoke.mockResolvedValue({ status: "NeedsSetup" }); // default: haven't configured yet

    const { wrapper } = mountSetup();
    await wrapper.find("button").trigger("click");

    expect(mockInvoke).toHaveBeenCalledWith("set_root_path", { path: "/Users/test/logbook-data" });
  });

  it("Ready result: updates store and navigates to ready screen", async () => {
    mockOpen.mockResolvedValue("/my/path");
    const dimensions = makeDimensions();
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: {
        root_path: "/my/path",
        dimensions,
        usingDefaultDimensions: false,
        today: { note: null, entries: [] },
        commitments: [makeCommitment()],
      },
    });

    const { wrapper, store } = mountSetup();
    await wrapper.find("button").trigger("click");

    expect(store.status).toBe("ready");
    expect(store.rootPath).toBe("/my/path");
    expect(store.dimensions).toEqual(dimensions);
    // Commitments are now set from init result (applyInitResult Ready branch).
    expect(store.commitments).toEqual([makeCommitment()]);
  });

  it("ConfigError result: updates store to error screen", async () => {
    mockOpen.mockResolvedValue("/bad/path");
    const errors = [{ kind: "MissingName", message: "Dimension 0 has an empty name" }];
    mockInvoke.mockResolvedValue({
      status: "ConfigError",
      data: { errors, scan_warnings: [] },
    });

    const { wrapper, store } = mountSetup();
    await wrapper.find("button").trigger("click");

    expect(store.status).toBe("error");
    expect(store.configErrors).toEqual(errors);
  });

  it("folder chosen but no template: ConfigError(config_missing) routes to error, no confirm", async () => {
    // Behavior change (Task 7): "no template.yaml" is no longer detected by
    // string-matching a thrown error + confirm()/create_starter_files. The
    // backend now classifies it as ConfigError(config_missing); the picker
    // just applies the InitResult. RecoveryScreen handles it downstream.
    mockOpen.mockResolvedValue("/empty/path");
    const confirmSpy = vi.spyOn(window, "confirm");
    mockInvoke.mockResolvedValue({
      status: "ConfigError",
      data: {
        category: "config_missing",
        root_path: "/empty/path",
        errors: [{ kind: "ConfigMissing", message: "no template.yaml" }],
        scan_warnings: [],
      },
    });

    const { wrapper, store } = mountSetup();
    await wrapper.find("button").trigger("click");
    await new Promise(r => setTimeout(r, 0));

    expect(confirmSpy).not.toHaveBeenCalled();
    expect(mockInvoke).not.toHaveBeenCalledWith("create_starter_files", expect.anything());
    expect(store.status).toBe("error");
    expect(store.configCategory).toBe("config_missing");
    confirmSpy.mockRestore();
  });

  it("generic error: shows error screen without asking confirm", async () => {
    mockOpen.mockResolvedValue("/bad");
    mockInvoke.mockRejectedValue("Permission denied");

    const { wrapper, store } = mountSetup();
    await wrapper.find("button").trigger("click");

    // No confirm dialog for generic errors
    expect(store.status).toBe("error");
    expect(store.configErrors[0].kind).toBe("SetupError");
  });
});
