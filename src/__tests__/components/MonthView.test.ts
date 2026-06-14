import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import { makeConfig, makeEntry, makeCommitmentProgress } from "../mocks/fixtures";
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
      currentDate: "2026-06-14", // same as system time
    });
    const wrapper = mountMonthView(store);
    expect(wrapper.findComponent(QuickEntry).exists()).toBe(true);
  });

  it("QuickEntry hidden when selected date is not today", () => {
    const store = createTestStore({
      screen: "ready",
      rootPath: "/test",
      config: makeConfig(),
      currentDate: "2026-06-10", // not today
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

    // Simulate DayStrip emitting selectDay
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
});
