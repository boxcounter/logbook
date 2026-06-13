import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount } from "@vue/test-utils";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import { makeEntry, makeConfig, makeDayFile } from "../mocks/fixtures";
import TodayView from "../../components/TodayView.vue";

const { mockInvoke, mockLogError, mockLogInfo } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockLogError: vi.fn(),
  mockLogInfo: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("../../utils/errorLog", () => ({ logError: mockLogError, logInfo: mockLogInfo }));

const TODAY = new Date();
const todayStr = `${TODAY.getFullYear()}-${String(TODAY.getMonth() + 1).padStart(2, "0")}-${String(TODAY.getDate()).padStart(2, "0")}`;

function mountToday(overrides?: Parameters<typeof createTestStore>[0]) {
  const store = createTestStore({
    screen: "ready",
    rootPath: "/test",
    currentDate: todayStr,
    today: makeDayFile({ entries: [] }),
    config: makeConfig(),
    commitments: [],
    ...overrides,
  });
  const triggerUndoToast = vi.fn();
  const wrapper = mount(TodayView, {
    global: {
      provide: { [STORE_KEY as symbol]: store, triggerUndoToast },
      stubs: { transition: true, Teleport: true },
    },
  });
  return { wrapper, store, triggerUndoToast };
}

// ============================================================

describe("TodayView", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_commitment_progress") return [];
      return makeDayFile();
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders child components", () => {
    const { wrapper } = mountToday();
    expect(wrapper.findComponent({ name: "DateNavigator" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "EntryList" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "SummaryBar" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "CommitmentsPanel" }).exists()).toBe(true);
  });

  it("shows QuickEntry when currentDate is today", () => {
    const { wrapper } = mountToday({ currentDate: todayStr });
    expect(wrapper.findComponent({ name: "QuickEntry" }).exists()).toBe(true);
  });

  it("hides QuickEntry when currentDate is not today", () => {
    const { wrapper } = mountToday({ currentDate: "2020-01-15" });
    expect(wrapper.findComponent({ name: "QuickEntry" }).exists()).toBe(false);
  });

  it("loadPeriod: calls get_entries for each date in day period", async () => {
    const { wrapper } = mountToday({ currentDate: todayStr, granularity: "day" });
    // Trigger loadPeriod via DateNavigator's navigate event
    const nav = wrapper.findComponent({ name: "DateNavigator" });
    await nav.vm.$emit("navigate");

    expect(mockInvoke).toHaveBeenCalledWith("get_entries", expect.objectContaining({
      rootPath: "/test",
      date: todayStr,
    }));
  });

  it("loadPeriod: populates periodEntries after loading", async () => {
    mockInvoke.mockResolvedValue({ note: null, entries: [makeEntry({ item: "Loaded" })] });
    const { wrapper, store } = mountToday({ currentDate: todayStr, granularity: "day" });
    const nav = wrapper.findComponent({ name: "DateNavigator" });
    await nav.vm.$emit("navigate");

    expect(store.periodEntries[todayStr]).toBeDefined();
    expect(store.periodEntries[todayStr].length).toBe(1);
  });

  it("loadPeriod: calls get_commitment_progress after loading entries", async () => {
    const { wrapper } = mountToday({ currentDate: todayStr, granularity: "day" });
    const nav = wrapper.findComponent({ name: "DateNavigator" });
    await nav.vm.$emit("navigate");

    expect(mockInvoke).toHaveBeenCalledWith("get_commitment_progress", expect.objectContaining({
      rootPath: "/test",
      year: TODAY.getFullYear(),
      month: TODAY.getMonth() + 1,
    }));
  });

  it("loadPeriod: stores commitment progress in store", async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_commitment_progress") return [
        { role: "Dev", allocation_minutes: 1800, spent_minutes: 300, goals: [] },
      ];
      return makeDayFile();
    });

    const { wrapper, store } = mountToday({ currentDate: todayStr, granularity: "day" });
    const nav = wrapper.findComponent({ name: "DateNavigator" });
    await nav.vm.$emit("navigate");

    expect(store.commitmentProgress).toEqual([
      { role: "Dev", allocation_minutes: 1800, spent_minutes: 300, goals: [] },
    ]);
  });

  it("handleUpdateEntry: calls invoke with only changed fields", async () => {
    const entry = makeEntry({ id: "e1", item: "Old item", duration: 30 });
    const { wrapper } = mountToday({
      currentDate: todayStr,
      today: makeDayFile({ entries: [entry] }),
    });

    const entryList = wrapper.findComponent({ name: "EntryList" });
    await entryList.vm.$emit("update", "e1", "New item", 30); // only item changed

    expect(mockInvoke).toHaveBeenCalledWith("update_entry", expect.objectContaining({
      entryId: "e1",
      update: { item: "New item" },
    }));
  });

  it("handleUpdateEntry: no-op when nothing changed", async () => {
    const entry = makeEntry({ id: "e1", item: "Same", duration: 30 });
    const { wrapper } = mountToday({
      currentDate: todayStr,
      today: makeDayFile({ entries: [entry] }),
    });

    const entryList = wrapper.findComponent({ name: "EntryList" });
    await entryList.vm.$emit("update", "e1", "Same", 30);

    // No invoke call because nothing changed
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("handleUpdateDimensions: calls invoke update_entry with dimensions", async () => {
    const entry = makeEntry({ id: "e1" });
    const { wrapper } = mountToday({
      currentDate: todayStr,
      today: makeDayFile({ entries: [entry] }),
    });

    const entryList = wrapper.findComponent({ name: "EntryList" });
    await entryList.vm.$emit("update-dimensions", "e1", { goal: "Code review" });

    expect(mockInvoke).toHaveBeenCalledWith("update_entry", expect.objectContaining({
      entryId: "e1",
      update: { dimensions: { goal: "Code review" } },
    }));
  });

  it("handleDeleteEntry: removes from UI immediately (optimistic)", async () => {
    const entry = makeEntry({ id: "e1" });
    const { wrapper, store } = mountToday({
      currentDate: todayStr,
      today: makeDayFile({ entries: [entry] }),
    });

    expect(store.today!.entries.length).toBe(1);

    const entryList = wrapper.findComponent({ name: "EntryList" });
    await entryList.vm.$emit("delete", "e1");

    expect(store.today!.entries.length).toBe(0);
  });

  it("handleDeleteEntry: calls triggerUndoToast with undo function", async () => {
    const entry = makeEntry({ id: "e1" });
    const { wrapper, triggerUndoToast } = mountToday({
      currentDate: todayStr,
      today: makeDayFile({ entries: [entry] }),
    });

    const entryList = wrapper.findComponent({ name: "EntryList" });
    await entryList.vm.$emit("delete", "e1");

    expect(triggerUndoToast).toHaveBeenCalledWith(expect.any(Function));
  });

  it("handleDeleteEntry: undo restores entry and cancels timer", async () => {
    const entry = makeEntry({ id: "e1" });
    const { wrapper, store, triggerUndoToast } = mountToday({
      currentDate: todayStr,
      today: makeDayFile({ entries: [entry] }),
    });

    const entryList = wrapper.findComponent({ name: "EntryList" });
    await entryList.vm.$emit("delete", "e1");
    expect(store.today!.entries.length).toBe(0);

    // Call the undo function that was passed to triggerUndoToast
    const undoFn = triggerUndoToast.mock.calls[0][0];
    undoFn();
    expect(store.today!.entries.length).toBe(1);

    // Advance past 5 seconds — should NOT call delete_entry (cancelled)
    await vi.advanceTimersByTimeAsync(6000);
    expect(mockInvoke).not.toHaveBeenCalledWith("delete_entry", expect.anything());
  });

  it("handleDeleteEntry: calls delete_entry after 5 seconds", async () => {
    const entry = makeEntry({ id: "e1" });
    const { wrapper } = mountToday({
      currentDate: todayStr,
      today: makeDayFile({ entries: [entry] }),
    });

    const entryList = wrapper.findComponent({ name: "EntryList" });
    await entryList.vm.$emit("delete", "e1");

    // Not called immediately
    expect(mockInvoke).not.toHaveBeenCalledWith("delete_entry", expect.anything());

    // Advance past 5 seconds (async version to flush pending promises)
    await vi.advanceTimersByTimeAsync(6000);

    expect(mockInvoke).toHaveBeenCalledWith("delete_entry", expect.objectContaining({
      entryId: "e1",
    }));
  });

  it("handleDeleteEntry: re-inserts entry on backend failure", async () => {
    // Set up mock to reject for delete_entry specifically
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "delete_entry") throw "Delete failed";
      return makeDayFile();
    });
    const entry = makeEntry({ id: "e1" });
    const { wrapper, store } = mountToday({
      currentDate: todayStr,
      today: makeDayFile({ entries: [entry] }),
    });

    const entryList = wrapper.findComponent({ name: "EntryList" });
    await entryList.vm.$emit("delete", "e1");
    expect(store.today!.entries.length).toBe(0);

    await vi.advanceTimersByTimeAsync(6000);

    // After the timer fires and invoke fails, entry should be restored
    expect(store.today!.entries.length).toBe(1);
  });

  it("openInEditor: calls invoke open_in_editor", async () => {
    const { wrapper } = mountToday();
    // Find the file path button
    const btn = wrapper.find(".text-right button");
    await btn.trigger("click");

    expect(mockInvoke).toHaveBeenCalledWith("open_in_editor", expect.objectContaining({
      rootPath: "/test",
    }));
  });

  it("shows truncated file path when rootPath is set", () => {
    const { wrapper } = mountToday({ rootPath: "/test/data" });
    expect(wrapper.text()).toContain("…/");
    expect(wrapper.text()).toContain(".md");
  });

  it("hides file path button when no rootPath", () => {
    const { wrapper } = mountToday({ rootPath: "" });
    expect(wrapper.find(".text-right button").exists()).toBe(false);
  });

  it("passes commitmentProgress and selected date to CommitmentsPanel", () => {
    const progress = [{ role: "Dev", allocation_minutes: 1800, spent_minutes: 300, goals: [] }];
    const { wrapper } = mountToday({ commitmentProgress: progress });

    const panel = wrapper.findComponent({ name: "CommitmentsPanel" });
    expect(panel.props("progress")).toEqual(progress);
    expect(panel.props("selectedYear")).toBe(TODAY.getFullYear());
    expect(panel.props("selectedMonth")).toBe(TODAY.getMonth() + 1);
  });
});
