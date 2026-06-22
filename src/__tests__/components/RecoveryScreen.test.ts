import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import RecoveryScreen from "../../components/RecoveryScreen.vue";

const { mockInvoke, mockPick } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockPick: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("../../composables/useRootFolderPicker", () => ({
  useRootFolderPicker: () => ({ pick: mockPick, applyRootPath: vi.fn() }),
}));
vi.mock("../../utils/errorLog", () => ({ logError: vi.fn() }));

function mountWith(category: string, reload = vi.fn()) {
  const store = createTestStore({
    status: "error",
    configCategory: category as never,
    rootPath: "/data/logbook",
    configErrors: [{ kind: "ConfigReadError", message: "boom" }],
  });
  const wrapper = mount(RecoveryScreen, {
    props: { reload },
    global: { provide: { [STORE_KEY as symbol]: store }, stubs: { Teleport: true } },
  });
  return { wrapper, store, reload };
}

describe("RecoveryScreen", () => {
  beforeEach(() => vi.clearAllMocks());

  it("in_place: shows error list, NO Retry button", () => {
    const { wrapper } = mountWith("in_place");
    expect(wrapper.findComponent({ name: "ConfigErrorBanner" }).exists()).toBe(true);
    expect(wrapper.text()).not.toContain("Retry");
    expect(wrapper.text()).toContain("Reveal");
  });

  it("in_place: Reveal calls reveal_template_file", async () => {
    const { wrapper } = mountWith("in_place");
    await wrapper.get('[data-testid="reveal-config"]').trigger("click");
    expect(mockInvoke).toHaveBeenCalledWith("reveal_template_file", { rootPath: "/data/logbook" });
  });

  it("config_missing: Recreate calls create_starter_files then reload", async () => {
    mockInvoke.mockResolvedValue(undefined);
    const { wrapper, reload } = mountWith("config_missing");
    await wrapper.get('[data-testid="recreate-config"]').trigger("click");
    await Promise.resolve();
    expect(mockInvoke).toHaveBeenCalledWith("create_starter_files", { path: "/data/logbook" });
    expect(reload).toHaveBeenCalled();
  });

  it("config_missing: failed Recreate shows error, does NOT reload", async () => {
    mockInvoke.mockRejectedValue("disk full");
    const { wrapper, reload } = mountWith("config_missing");
    await wrapper.get('[data-testid="recreate-config"]').trigger("click");
    await Promise.resolve();
    await wrapper.vm.$nextTick();
    expect(reload).not.toHaveBeenCalled();
    expect(wrapper.get('[data-testid="recreate-error"]').text()).toContain("disk full");
  });

  it("root_missing: shows Retry, Retry calls reload", async () => {
    const { wrapper, reload } = mountWith("root_missing");
    expect(wrapper.text()).toContain("Retry");
    await wrapper.get('[data-testid="retry"]').trigger("click");
    expect(reload).toHaveBeenCalled();
  });

  it("root_missing: Start-fresh requires a second confirm", async () => {
    mockInvoke.mockResolvedValue(undefined);
    const { wrapper } = mountWith("root_missing");
    // first click only reveals the confirm sub-panel, does not call create
    await wrapper.get('[data-testid="start-fresh"]').trigger("click");
    expect(mockInvoke).not.toHaveBeenCalledWith("create_starter_files", expect.anything());
    // confirm
    await wrapper.get('[data-testid="start-fresh-confirm"]').trigger("click");
    await Promise.resolve();
    expect(mockInvoke).toHaveBeenCalledWith("create_starter_files", { path: "/data/logbook" });
  });

  it("both config_missing and root_missing offer Choose-folder", async () => {
    for (const cat of ["config_missing", "root_missing"]) {
      vi.clearAllMocks();
      const { wrapper } = mountWith(cat);
      await wrapper.get('[data-testid="choose-folder"]').trigger("click");
      expect(mockPick).toHaveBeenCalled();
    }
  });
});
