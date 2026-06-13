import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import { makeConfig, makeEntry, makeDayFile } from "../mocks/fixtures";
import QuickEntry from "../../components/QuickEntry.vue";

const { mockInvoke, mockLogError, mockLogInfo } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockLogError: vi.fn(),
  mockLogInfo: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("../../utils/errorLog", () => ({ logError: mockLogError, logInfo: mockLogInfo }));

function mountQuick(overrides?: Parameters<typeof createTestStore>[0]) {
  const store = createTestStore({
    rootPath: "/test",
    config: makeConfig(),
    today: makeDayFile(),
    commitments: [],
    ...overrides,
  });
  const wrapper = mount(QuickEntry, {
    global: {
      provide: { [STORE_KEY as symbol]: store },
      stubs: { transition: true },
    },
  });
  return { wrapper, store };
}

// ============================================================

describe("QuickEntry", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(makeEntry());
  });

  it("renders EntryInput with correct props", () => {
    const { wrapper } = mountQuick();
    const entryInput = wrapper.findComponent({ name: "EntryInput" });
    expect(entryInput.exists()).toBe(true);
    expect(entryInput.props("dimensions")).toEqual(makeConfig().dimensions);
    expect(entryInput.props("commitments")).toEqual([]);
  });

  function findToggleBtn(wrapper: ReturnType<typeof mountQuick>["wrapper"]) {
    // QuickEntry has its own button "Show/Hide Dimensions" after EntryInput (which has "Log")
    const buttons = wrapper.findAll("button");
    for (const btn of buttons) {
      if (btn.text().includes("Dimensions")) return btn;
    }
    return buttons[buttons.length - 1]; // fallback
  }

  it('renders "Show/Hide Dimensions" toggle button', () => {
    const { wrapper } = mountQuick();
    const btn = findToggleBtn(wrapper);
    expect(btn.text()).toContain("Dimensions");
  });

  it("DimensionPanel hidden by default", () => {
    const { wrapper } = mountQuick();
    expect(wrapper.findComponent({ name: "DimensionPanel" }).exists()).toBe(false);
  });

  it("click toggle shows DimensionPanel", async () => {
    const { wrapper } = mountQuick();
    const btn = findToggleBtn(wrapper);
    await btn.trigger("click");
    expect(wrapper.findComponent({ name: "DimensionPanel" }).exists()).toBe(true);
    expect(btn.text()).toContain("Hide");
  });

  it("click toggle again hides DimensionPanel", async () => {
    const { wrapper } = mountQuick();
    const btn = findToggleBtn(wrapper);
    await btn.trigger("click");
    await btn.trigger("click");
    expect(wrapper.findComponent({ name: "DimensionPanel" }).exists()).toBe(false);
  });

  it("handleSubmit calls invoke append_entry with correct args", async () => {
    const { wrapper, store } = mountQuick();
    const entryInput = wrapper.findComponent({ name: "EntryInput" });

    await entryInput.vm.$emit("submit", "Write tests", 90, { goal: "Code review" });

    await wrapper.vm.$nextTick();

    expect(mockInvoke).toHaveBeenCalledWith("append_entry", expect.objectContaining({
      rootPath: "/test",
      date: store.currentDate,
      entry: expect.objectContaining({
        item: "Write tests",
        duration: "90",
        dimensions: { goal: "Code review" },
      }),
    }));
  });

  it("handleSubmit success: stores lastDimensions", async () => {
    const { wrapper, store } = mountQuick();
    const entryInput = wrapper.findComponent({ name: "EntryInput" });

    await entryInput.vm.$emit("submit", "Task", 30, { goal: "Ship" });

    await wrapper.vm.$nextTick();
    // lastDimensions should be updated
    expect(store.lastDimensions).toEqual({ goal: "Ship" });
  });

  it("handleSubmit success: emits appended", async () => {
    const { wrapper } = mountQuick();
    const entryInput = wrapper.findComponent({ name: "EntryInput" });

    await entryInput.vm.$emit("submit", "Task", 30, {});

    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("appended")).toBeTruthy();
  });

  it("handleSubmit success: optimistically appends entry to store.today", async () => {
    const { wrapper, store } = mountQuick({ today: makeDayFile({ entries: [] }) });
    const entryInput = wrapper.findComponent({ name: "EntryInput" });

    await entryInput.vm.$emit("submit", "New task", 45, {});

    await wrapper.vm.$nextTick();
    expect(store.today!.entries.length).toBe(1);
    expect(store.today!.entries[0].item).toBe("Test entry"); // from mockInvoke result
  });

  it("sanitizeValues: removes invalid dimension keys", async () => {
    const { wrapper } = mountQuick();
    const entryInput = wrapper.findComponent({ name: "EntryInput" });

    await entryInput.vm.$emit("submit", "Task", 30, {
      goal: "valid",
      "nonexistent-key": "should-be-removed",
    });

    await wrapper.vm.$nextTick();
    // Check that only valid keys are in the invoke args
    const callArgs = mockInvoke.mock.calls[0][1];
    expect(callArgs.entry.dimensions).not.toHaveProperty("nonexistent-key");
    expect(callArgs.entry.dimensions).toHaveProperty("goal", "valid");
  });

  it("error on submit: logs error without crashing", async () => {
    mockInvoke.mockRejectedValue("Append failed");
    const { wrapper } = mountQuick();
    const entryInput = wrapper.findComponent({ name: "EntryInput" });

    // Should not throw
    await entryInput.vm.$emit("submit", "Task", 30, {});

    await wrapper.vm.$nextTick();
    expect(mockLogError).toHaveBeenCalled();
  });
});
