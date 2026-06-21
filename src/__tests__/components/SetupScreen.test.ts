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
        from_template: false,
        today: { note: null, entries: [] },
        commitments: [makeCommitment()],
      },
    });

    const { wrapper, store } = mountSetup();
    await wrapper.find("button").trigger("click");

    expect(store.status).toBe("ready");
    expect(store.rootPath).toBe("/my/path");
    expect(store.dimensions).toEqual(dimensions);
    // SetupScreen no longer owns commitments — loadMonth (in MonthView) loads them.
    expect(store.commitments).toEqual([]);
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

  it("error with 'No such file': shows confirm dialog", async () => {
    mockOpen.mockResolvedValue("/empty/path");
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);
    // Use mockImplementation to ensure the rejection happens
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "set_root_path") throw "No such file or directory";
      return undefined;
    });

    const { wrapper } = mountSetup();
    await wrapper.find("button").trigger("click");
    // Need to wait for async handler chain via setTimeout
    await new Promise(r => setTimeout(r, 50));

    expect(confirmSpy).toHaveBeenCalled();
    // User said no → screen stays loading (the catch path for 'no' doesn't set screen)
    confirmSpy.mockRestore();
  });

  it("confirm yes to create starter files: calls create_starter_files and retries", async () => {
    mockOpen.mockResolvedValue("/empty/path");
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);

    const callOrder: string[] = [];
    mockInvoke.mockImplementation(async (cmd: string, args?: any) => {
      callOrder.push(cmd);
      if (cmd === "set_root_path" && callOrder.filter(c => c === "set_root_path").length === 1) {
        // First set_root_path call fails
        throw "No such file or directory";
      }
      if (cmd === "create_starter_files") return undefined;
      if (cmd === "set_root_path" && callOrder.filter(c => c === "set_root_path").length === 2) {
        // Second set_root_path call succeeds
        return {
          status: "Ready",
          data: { root_path: args!.path, dimensions: makeDimensions(), from_template: false, today: { note: null, entries: [] }, commitments: [] },
        };
      }
      return undefined;
    });

    const { wrapper, store } = mountSetup();
    await wrapper.find("button").trigger("click");
    await new Promise(r => setTimeout(r, 50));

    expect(mockInvoke).toHaveBeenCalledWith("create_starter_files", { path: "/empty/path" });
    expect(store.status).toBe("ready");
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
