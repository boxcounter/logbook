// src/__tests__/components/MonthView.test.ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { reactive } from "vue";
import MonthView from "../../components/MonthView.vue";
import { STORE_KEY } from "../../stores/useStore";
import { makeConfig, makeCommitment, makeEntry } from "../mocks/fixtures";
import { addDays } from "../../utils/dates";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));

// Compute today's date string at test runtime (not hardcoded) so isSelectedToday works
function todayDateStr(): string {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
}

function makeStore() {
  const today = todayDateStr();
  return reactive({
    screen: "ready",
    rootPath: "/root",
    config: makeConfig(),
    configErrors: [],
    commitments: [makeCommitment({ goals: ["Bug fixes"] })],
    commitmentProgress: [],
    today: { note: null, entries: [makeEntry({ item: "Existing", duration: 60 })] },
    lastDimensions: {},
    currentDate: today,
    monthEntries: { [today]: [makeEntry({ item: "Existing", duration: 60 })] },
    availableMonths: null,
  });
}

function mountView(store = makeStore()) {
  return mount(MonthView, {
    global: {
      provide: { [STORE_KEY as symbol]: store, focusRequestId: { value: 0 }, triggerUndoToast: () => {} },
    },
  });
}

beforeEach(() => {
  invokeMock.mockReset();
  // Route by command so get_commitment_progress always returns an array
  invokeMock.mockImplementation(async (cmd: string) => {
    if (cmd === "get_commitment_progress") return [];
    return { note: null, entries: [] };
  });
});

describe("MonthView", () => {
  it("renders the three zones: HeatmapCalendar, DayHeader, EntryList, TwoLineInput", () => {
    const wrapper = mountView();
    expect(wrapper.findComponent({ name: "HeatmapCalendar" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "DayHeader" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "EntryList" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "TwoLineInput" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "CommitmentsPanel" }).exists()).toBe(true);
  });

  it("calls append_entry when TwoLineInput emits submit", async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "append_entry") return makeEntry({ item: "New task", duration: 30 });
      if (cmd === "get_commitment_progress") return [];
      return { note: null, entries: [] };
    });
    const wrapper = mountView();
    wrapper.findComponent({ name: "TwoLineInput" }).vm.$emit("submit", "New task", 30, { category: "Coding" });
    await wrapper.vm.$nextTick();
    expect(invokeMock).toHaveBeenCalledWith(
      "append_entry",
      expect.objectContaining({ rootPath: "/root", date: todayDateStr() }),
    );
  });

  it("only renders TwoLineInput when the selected day is today", () => {
    const store = makeStore();
    store.currentDate = "2026-06-10"; // not today (past date in-month)
    const wrapper = mountView(store);
    expect(wrapper.findComponent({ name: "TwoLineInput" }).exists()).toBe(false);
  });

  it("renders the day note above the entry list", () => {
    const wrapper = mountView();
    const html = wrapper.html();
    const noteIdx = html.indexOf('contenteditable');
    const listIdx = html.indexOf('No entries'); // empty-state marker, or fall back below
    const listAnchor = listIdx !== -1 ? listIdx : html.indexOf('overflow-y-auto');
    expect(noteIdx).toBeGreaterThan(-1);
    expect(listAnchor).toBeGreaterThan(-1);
    expect(noteIdx).toBeLessThan(listAnchor);
  });

  it("Esc on the day note reverts its content to the pre-edit snapshot", async () => {
    const wrapper = mountView();
    const note = wrapper.find("[contenteditable]");
    note.element.textContent = "original";
    await note.trigger("focus");          // snapshot taken here
    note.element.textContent = "edited away";
    await note.trigger("keydown", { key: "Escape" });
    expect(note.element.textContent).toBe("original");
  });

  it("Enter on the day note commits and blurs instead of inserting a newline", async () => {
    const wrapper = mountView();
    const note = wrapper.find("[contenteditable]");
    note.element.textContent = "my note";
    await note.trigger("focus");
    const blurSpy = vi.spyOn(note.element as HTMLElement, "blur");
    await note.trigger("keydown", { key: "Enter" });
    expect(blurSpy).toHaveBeenCalled();
  });

  it("prev-day from DayHeader moves currentDate back one day", async () => {
    const store = makeStore();
    const wrapper = mountView(store);
    wrapper.findComponent({ name: "DayHeader" }).vm.$emit("prev-day");
    await wrapper.vm.$nextTick();
    expect(store.currentDate).toBe(addDays(todayDateStr(), -1));
  });

  it("next-day is a no-op when the selected day is today", async () => {
    const store = makeStore(); // currentDate === today
    const wrapper = mountView(store);
    wrapper.findComponent({ name: "DayHeader" }).vm.$emit("next-day");
    await wrapper.vm.$nextTick();
    expect(store.currentDate).toBe(todayDateStr());
  });

  it("passes can-go-next=false to DayHeader when today is selected", () => {
    const store = makeStore();
    const wrapper = mountView(store);
    expect(wrapper.findComponent({ name: "DayHeader" }).props("canGoNext")).toBe(false);
  });

  it("⌘[ moves back one day", async () => {
    const store = makeStore();
    const wrapper = mountView(store);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "[", metaKey: true }));
    await wrapper.vm.$nextTick();
    expect(store.currentDate).toBe(addDays(todayDateStr(), -1));
  });

  it("⌘⇧[ moves back one month", async () => {
    const store = makeStore();
    const wrapper = mountView(store);
    const expectedMonth = todayDateStr().slice(0, 7); // current YYYY-MM
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "[", metaKey: true, shiftKey: true }));
    await wrapper.vm.$nextTick();
    expect(store.currentDate.slice(0, 7)).not.toBe(expectedMonth);
  });
});
