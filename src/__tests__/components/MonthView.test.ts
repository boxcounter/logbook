import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick } from "vue";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import { makeConfig, makeEntry, makeCommitmentProgress, makeDayFile } from "../mocks/fixtures";
import MonthView from "../../components/MonthView.vue";
import DayStrip from "../../components/DayStrip.vue";
import MonthNavigator from "../../components/MonthNavigator.vue";
import CommitmentsPanel from "../../components/CommitmentsPanel.vue";
import QuickEntry from "../../components/QuickEntry.vue";
import EntryList from "../../components/EntryList.vue";

// Hoisted mocks for Tauri invoke
const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));

function mountMonthView(store = createTestStore()) {
  return mount(MonthView, {
    global: {
      provide: { [STORE_KEY as symbol]: store },
    },
  });
}

describe("MonthView", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 5, 14)); // June 14, 2026
    vi.clearAllMocks();

    // Default: get_entries returns empty day files
    mockInvoke.mockImplementation(async (cmd: string, _args: unknown) => {
      if (cmd === "get_entries") return { note: null, entries: [] };
      if (cmd === "get_commitment_progress") return [];
      return {};
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // ============================================================
  // Rendering basics (existing)
  // ============================================================

  it("renders left sidebar with MonthNavigator and CommitmentsPanel", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
      commitmentProgress: [makeCommitmentProgress()],
    });
    const wrapper = mountMonthView(store);
    expect(wrapper.findComponent(MonthNavigator).exists()).toBe(true);
    expect(wrapper.findComponent(CommitmentsPanel).exists()).toBe(true);
  });

  it("renders right panel with DayStrip and EntryList", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
      monthEntries: { "2026-06-14": [makeEntry()] },
    });
    store.today = { note: null, entries: [makeEntry()] };
    const wrapper = mountMonthView(store);
    expect(wrapper.findComponent(DayStrip).exists()).toBe(true);
    expect(wrapper.findComponent(EntryList).exists()).toBe(true);
  });

  it("QuickEntry visible when selected date is today", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });
    const wrapper = mountMonthView(store);
    expect(wrapper.findComponent(QuickEntry).exists()).toBe(true);
  });

  it("QuickEntry hidden when selected date is not today", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-10",
    });
    const wrapper = mountMonthView(store);
    expect(wrapper.findComponent(QuickEntry).exists()).toBe(false);
  });

  it("DayStrip receives monthDates from currentDate", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-02-14",
    });
    const wrapper = mountMonthView(store);
    const strip = wrapper.findComponent(DayStrip);
    expect(strip.props("dates")).toHaveLength(28); // February 2026
  });

  it("clicking a day in DayStrip updates currentDate and today", async () => {
    const entries = [makeEntry({ item: "Test", duration: 30 })];
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
      monthEntries: {
        "2026-06-14": [],
        "2026-06-15": entries,
      },
    });
    store.today = { note: null, entries: [] };
    const wrapper = mountMonthView(store);

    const strip = wrapper.findComponent(DayStrip);
    await strip.vm.$emit("selectDay", "2026-06-15");
    await nextTick();

    expect(store.currentDate).toBe("2026-06-15");
    expect(store.today?.entries).toEqual(entries);
  });

  it("renders day note contenteditable", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });
    store.today = { note: "My note", entries: [] };
    const wrapper = mountMonthView(store);
    expect(wrapper.find('[contenteditable="true"]').exists()).toBe(true);
  });

  // ============================================================
  // Data loading on mount
  // ============================================================

  it("onMounted: loads entries for all days in current month", async () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    mountMonthView(store);
    await flushPromises();

    // 30 days in June + 1 loadDayNote = 31 get_entries calls
    const getEntriesCalls = mockInvoke.mock.calls.filter(
      (c) => c[0] === "get_entries"
    );
    expect(getEntriesCalls).toHaveLength(31);
  });

  it("onMounted: calls get_commitment_progress for current month", async () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    mountMonthView(store);
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("get_commitment_progress", {
      rootPath: "/test",
      year: 2026,
      month: 6,
    });
  });

  it("onMounted: populates store.monthEntries", async () => {
    mockInvoke.mockImplementation(async (cmd: string, args: unknown) => {
      if (cmd === "get_entries") {
        const { date } = args as { date: string };
        if (date === "2026-06-14") {
          return { note: "test note", entries: [makeEntry({ item: "Work", duration: 60 })] };
        }
        return { note: null, entries: [] };
      }
      if (cmd === "get_commitment_progress") return [];
      return {};
    });

    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    mountMonthView(store);
    await flushPromises();

    expect(store.monthEntries["2026-06-14"]).toHaveLength(1);
    expect(store.monthEntries["2026-06-14"][0].item).toBe("Work");
  });

  it("onMounted: sets store.today from loaded data", async () => {
    mockInvoke.mockImplementation(async (cmd: string, args: unknown) => {
      if (cmd === "get_entries") {
        const { date } = args as { date: string };
        if (date === "2026-06-14") {
          return { note: "loaded note", entries: [makeEntry()] };
        }
        return { note: null, entries: [] };
      }
      if (cmd === "get_commitment_progress") return [];
      return {};
    });

    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    mountMonthView(store);
    await flushPromises();

    expect(store.today?.note).toBe("loaded note");
    expect(store.today?.entries).toHaveLength(1);
  });

  it("onMounted: does nothing when rootPath is empty", async () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    mountMonthView(store);
    await flushPromises();

    // No invoke calls since onMounted guards on rootPath
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  // ============================================================
  // Month navigation
  // ============================================================

  it("navigating to a different month via MonthNavigator reloads data", async () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    const wrapper = mountMonthView(store);
    await flushPromises();
    mockInvoke.mockClear();

    // Simulate MonthNavigator emitting navigate to July
    const nav = wrapper.findComponent(MonthNavigator);
    await nav.vm.$emit("navigate", { year: 2026, month: 7 });
    await flushPromises();

    // July has 31 days + 1 loadDayNote = 32 get_entries calls
    const getEntriesCalls = mockInvoke.mock.calls.filter(
      (c) => c[0] === "get_entries"
    );
    expect(getEntriesCalls).toHaveLength(32);

    // Should update currentDate to July
    expect(store.currentDate).toContain("2026-07");

    // Should call get_commitment_progress for July
    expect(mockInvoke).toHaveBeenCalledWith("get_commitment_progress", {
      rootPath: "/test",
      year: 2026,
      month: 7,
    });
  });

  it("navigating to a past month defaults to last day", async () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    const wrapper = mountMonthView(store);
    await flushPromises();
    mockInvoke.mockClear();

    // Navigate to February 2026 (past, not current month, no defaultDay)
    const nav = wrapper.findComponent(MonthNavigator);
    await nav.vm.$emit("navigate", { year: 2026, month: 2 });
    await flushPromises();

    // Past months default to last day (Feb 2026 = 28 days)
    expect(store.currentDate).toBe("2026-02-28");
  });

  // ============================================================
  // Lazy load available months
  // ============================================================

  it("handleRequestMonths: calls get_available_months when null", async () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
      availableMonths: null,
    });

    const sampleMonths = [
      { year: 2026, month: 6 },
      { year: 2026, month: 5 },
    ];
    mockInvoke.mockImplementation(async (cmd: string, _args: unknown) => {
      if (cmd === "get_entries") return { note: null, entries: [] };
      if (cmd === "get_commitment_progress") return [];
      if (cmd === "get_available_months") return sampleMonths;
      return {};
    });

    const wrapper = mountMonthView(store);
    await flushPromises();
    mockInvoke.mockClear();

    // Simulate MonthNavigator requesting months
    const nav = wrapper.findComponent(MonthNavigator);
    await nav.vm.$emit("requestMonths");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("get_available_months", {
      rootPath: "/test",
    });
    expect(store.availableMonths).toEqual(sampleMonths);
  });

  it("handleRequestMonths: skips when already loaded", async () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
      availableMonths: [{ year: 2026, month: 6 }],
    });

    const wrapper = mountMonthView(store);
    await flushPromises();
    mockInvoke.mockClear();

    // Should not call get_available_months again
    const nav = wrapper.findComponent(MonthNavigator);
    await nav.vm.$emit("requestMonths");
    await flushPromises();

    expect(mockInvoke).not.toHaveBeenCalledWith(
      "get_available_months",
      expect.anything()
    );
  });

  it("handleRequestMonths: sets empty array on error", async () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
      availableMonths: null,
    });

    mockInvoke.mockImplementation(async (cmd: string, _args: unknown) => {
      if (cmd === "get_entries") return { note: null, entries: [] };
      if (cmd === "get_commitment_progress") return [];
      if (cmd === "get_available_months") throw new Error("disk error");
      return {};
    });

    const wrapper = mountMonthView(store);
    await flushPromises();
    mockInvoke.mockClear();

    const nav = wrapper.findComponent(MonthNavigator);
    await nav.vm.$emit("requestMonths");
    await flushPromises();

    expect(store.availableMonths).toEqual([]);
  });

  // ============================================================
  // Entry CRUD
  // ============================================================

  it("handleUpdateEntry: calls invoke update_entry with correct params", async () => {
    const entry = makeEntry({ id: "e1", item: "Old", duration: 30 });
    const updatedDayFile = makeDayFile({ entries: [makeEntry({ id: "e1", item: "New", duration: 45 })] });

    // Mock must return the entry during loadMonth so store.today survives
    mockInvoke.mockImplementation(async (cmd: string, args: unknown) => {
      if (cmd === "get_entries") {
        const { date } = args as { date: string };
        if (date === "2026-06-14") return { note: null, entries: [entry] };
        return { note: null, entries: [] };
      }
      if (cmd === "get_commitment_progress") return [];
      if (cmd === "update_entry") return updatedDayFile;
      return {};
    });

    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    const wrapper = mountMonthView(store);
    await flushPromises();
    mockInvoke.mockClear();

    const entryList = wrapper.findComponent(EntryList);
    await entryList.vm.$emit("update", "e1", "New", 45);
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("update_entry", {
      rootPath: "/test",
      date: "2026-06-14",
      entryId: "e1",
      update: { item: "New", duration: "45" },
    });
    expect(store.today?.entries[0].item).toBe("New");
  });

  it("handleUpdateEntry: skips invoke when nothing changed", async () => {
    const entry = makeEntry({ id: "e1", item: "Same", duration: 30 });

    mockInvoke.mockImplementation(async (cmd: string, args: unknown) => {
      if (cmd === "get_entries") {
        const { date } = args as { date: string };
        if (date === "2026-06-14") return { note: null, entries: [entry] };
        return { note: null, entries: [] };
      }
      if (cmd === "get_commitment_progress") return [];
      return {};
    });

    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    const wrapper = mountMonthView(store);
    await flushPromises();
    mockInvoke.mockClear();

    const entryList = wrapper.findComponent(EntryList);
    await entryList.vm.$emit("update", "e1", "Same", 30);
    await flushPromises();

    // No change → no invoke
    expect(mockInvoke).not.toHaveBeenCalledWith("update_entry", expect.anything());
  });

  it("handleUpdateDimensions: calls invoke update_entry with dimensions", async () => {
    const entry = makeEntry({ id: "e1" });
    const updatedDayFile = makeDayFile({ entries: [makeEntry({ id: "e1" })] });

    mockInvoke.mockImplementation(async (cmd: string, args: unknown) => {
      if (cmd === "get_entries") {
        const { date } = args as { date: string };
        if (date === "2026-06-14") return { note: null, entries: [entry] };
        return { note: null, entries: [] };
      }
      if (cmd === "get_commitment_progress") return [];
      if (cmd === "update_entry") return updatedDayFile;
      return {};
    });

    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    const wrapper = mountMonthView(store);
    await flushPromises();
    mockInvoke.mockClear();

    const entryList = wrapper.findComponent(EntryList);
    await entryList.vm.$emit("update-dimensions", "e1", { goal: "Code review" });
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("update_entry", {
      rootPath: "/test",
      date: "2026-06-14",
      entryId: "e1",
      update: { dimensions: { goal: "Code review" } },
    });
  });

  it("handleDeleteEntry: removes from store immediately", async () => {
    const entry = makeEntry({ id: "e1", item: "To delete" });

    mockInvoke.mockImplementation(async (cmd: string, args: unknown) => {
      if (cmd === "get_entries") {
        const { date } = args as { date: string };
        if (date === "2026-06-14") return { note: null, entries: [entry] };
        return { note: null, entries: [] };
      }
      if (cmd === "get_commitment_progress") return [];
      return {};
    });

    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    const wrapper = mountMonthView(store);
    await flushPromises();

    const entryList = wrapper.findComponent(EntryList);
    await entryList.vm.$emit("delete", "e1");
    await nextTick();

    // Entry immediately removed from store
    expect(store.today?.entries).toHaveLength(0);
  });

  it("handleAppend: calls loadMonth to refresh data", async () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    const wrapper = mountMonthView(store);
    await flushPromises();
    mockInvoke.mockClear();

    const quickEntry = wrapper.findComponent(QuickEntry);
    await quickEntry.vm.$emit("appended");
    await flushPromises();

    // 30 June days + 1 loadDayNote = 31 get_entries calls
    const getEntriesCalls = mockInvoke.mock.calls.filter(
      (c) => c[0] === "get_entries"
    );
    expect(getEntriesCalls).toHaveLength(31);
  });

  // ============================================================
  // Day note
  // ============================================================

  it("saveNote: calls set_day_note on blur", async () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });
    store.today = { note: "My note", entries: [] };

    const wrapper = mountMonthView(store);
    await flushPromises();
    mockInvoke.mockClear();

    const noteDiv = wrapper.find('[contenteditable="true"]');
    // Set text content to simulate user typing
    noteDiv.element.textContent = "Updated note";
    await noteDiv.trigger("blur");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("set_day_note", {
      rootPath: "/test",
      date: "2026-06-14",
      note: "Updated note",
    });
  });

  it("day note placeholder is rendered when empty", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });
    store.today = { note: null, entries: [] };

    const wrapper = mountMonthView(store);
    const noteDiv = wrapper.find('[contenteditable="true"]');
    expect(noteDiv.attributes("data-placeholder")).toBe("Add a note…");
  });

  // ============================================================
  // File path
  // ============================================================

  it("renders file path button", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    const wrapper = mountMonthView(store);
    // File path button shows "…/2026/06/2026-06-14.md"
    expect(wrapper.text()).toContain("…/2026/06/2026-06-14.md");
  });

  it("file path button click calls open_in_editor", async () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    const wrapper = mountMonthView(store);
    await flushPromises();
    mockInvoke.mockClear();

    // Find the file path button (contains "…/")
    const fileButton = wrapper.find("button[title]");
    await fileButton.trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("open_in_editor", {
      rootPath: "/test",
      date: "2026-06-14",
    });
  });

  it("file path button hidden when rootPath is empty", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    const wrapper = mountMonthView(store);
    const fileButton = wrapper.find("button[title]");
    expect(fileButton.exists()).toBe(false);
  });

  // ============================================================
  // Day selection edge cases
  // ============================================================

  it("day click loads note via get_entries when day has data", async () => {
    const entries = [makeEntry({ item: "Test" })];
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
      monthEntries: {
        "2026-06-14": [],
        "2026-06-15": entries,
      },
    });
    store.today = { note: null, entries: [] };

    const wrapper = mountMonthView(store);
    await flushPromises();
    mockInvoke.mockClear();

    // Mock get_entries to return a note for the target day
    mockInvoke.mockImplementation(async (cmd: string, args: unknown) => {
      if (cmd === "get_entries") {
        const { date } = args as { date: string };
        if (date === "2026-06-15") return { note: "day note", entries };
        return { note: null, entries: [] };
      }
      if (cmd === "get_commitment_progress") return [];
      return {};
    });

    const strip = wrapper.findComponent(DayStrip);
    await strip.vm.$emit("selectDay", "2026-06-15");
    await flushPromises();

    expect(store.currentDate).toBe("2026-06-15");
    expect(store.today?.note).toBe("day note");
  });

  // ============================================================
  // Commitments loading
  // ============================================================

  it("loadCommitmentProgress: updates store.commitmentProgress on success", async () => {
    const progress = [makeCommitmentProgress({ spent_minutes: 120 })];
    mockInvoke.mockImplementation(async (cmd: string, _args: unknown) => {
      if (cmd === "get_entries") return { note: null, entries: [] };
      if (cmd === "get_commitment_progress") return progress;
      return {};
    });

    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    mountMonthView(store);
    await flushPromises();

    expect(store.commitmentProgress).toEqual(progress);
  });

  it("loadCommitmentProgress: sets empty array on error", async () => {
    mockInvoke.mockImplementation(async (cmd: string, _args: unknown) => {
      if (cmd === "get_entries") return { note: null, entries: [] };
      if (cmd === "get_commitment_progress") throw new Error("fail");
      return {};
    });

    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-14",
    });

    mountMonthView(store);
    await flushPromises();

    expect(store.commitmentProgress).toEqual([]);
  });
});
