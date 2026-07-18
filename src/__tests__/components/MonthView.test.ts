// src/__tests__/components/MonthView.test.ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import MonthView from "../../components/MonthView.vue";
import { STORE_KEY } from "../../stores/useStore";
import { UNDO_TOAST_KEY } from "../../types";
import { createTestStore } from "../mocks/store";
import { makeDimensions, makeCommitment, makeEntry } from "../mocks/fixtures";
import { addDays } from "../../utils/dates";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn().mockResolvedValue("0.0.0") }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: vi.fn().mockReturnValue({ setTitle: vi.fn() }) }));
import { getCurrentWindow } from "@tauri-apps/api/window";
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

// Compute today's date string at test runtime (not hardcoded) so isSelectedToday works
function todayDateStr(): string {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
}

function makeStore() {
  const today = todayDateStr();
  return createTestStore({
    status: "ready",
    rootPath: "/root",
    dimensions: makeDimensions(),
    commitments: [makeCommitment({ goals: ["Bug fixes"] })],
    currentDate: today,
    monthEntries: { [today]: [makeEntry({ item: "Existing", duration: 60 })] },
  });
}

function mountView(store = makeStore()) {
  return mount(MonthView, {
    global: {
      provide: { [STORE_KEY as symbol]: store, focusRequestId: { value: 0 }, [UNDO_TOAST_KEY as symbol]: () => {} },
    },
  });
}

beforeEach(() => {
  invokeMock.mockReset();
  // Route by command so progress/commitments always return arrays
  invokeMock.mockImplementation(async (cmd: string) => {
    if (cmd === "get_commitment_progress") return [];
    if (cmd === "get_commitments") return [];
    return { note: null, entries: [] };
  });
});

describe("MonthView", () => {
  it("renders the three zones: HeatmapCalendar, DayHeader, EntryList, EntryComposer", () => {
    const wrapper = mountView();
    expect(wrapper.findComponent({ name: "HeatmapCalendar" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "DayHeader" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "EntryList" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "EntryComposer" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "CommitmentsPanel" }).exists()).toBe(true);
  });

  it("calls append_entry when EntryComposer emits submit", async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "append_entry") return makeEntry({ item: "New task", duration: 30 });
      if (cmd === "get_commitment_progress") return [];
      return { note: null, entries: [] };
    });
    const wrapper = mountView();
    wrapper.findComponent({ name: "EntryComposer" }).vm.$emit("submit", "New task", 30, { category: "Coding" });
    await wrapper.vm.$nextTick();
    expect(invokeMock).toHaveBeenCalledWith(
      "append_entry",
      expect.objectContaining({ rootPath: "/root", date: todayDateStr() }),
    );
  });

  it("only renders EntryComposer when the selected day is today", () => {
    const store = makeStore();
    store.currentDate = "2026-06-10"; // not today (past date in-month)
    const wrapper = mountView(store);
    expect(wrapper.findComponent({ name: "EntryComposer" }).exists()).toBe(false);
  });

  it("passes is-today=false to EntryList for a non-today date", () => {
    const store = makeStore();
    store.currentDate = "2026-06-10";
    const wrapper = mountView(store);
    const entryList = wrapper.findComponent({ name: "EntryList" });
    expect(entryList.props("isToday")).toBe(false);
  });

  it("passes is-today=true to EntryList for today", () => {
    const wrapper = mountView();
    const entryList = wrapper.findComponent({ name: "EntryList" });
    expect(entryList.props("isToday")).toBe(true);
  });

  it("shows the default-template indicator only when usingDefaultDimensions is true and month is not in the future", () => {
    const off = mountView();
    expect(off.text()).not.toContain("Using default template");

    const store = makeStore();
    store.usingDefaultDimensions = true;
    const on = mountView(store);
    expect(on.text()).toContain("Using default template");

    // Future month: should NOT show even when usingDefaultDimensions is true
    const futureStore = makeStore();
    futureStore.usingDefaultDimensions = true;
    // Set to next year, January — definitely future
    futureStore.currentDate = `${new Date().getFullYear() + 1}-01-01`;
    const future = mountView(futureStore);
    expect(future.text()).not.toContain("Using default template");
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
    await flushPromises(); // wait for onMounted loadMonth
    wrapper.findComponent({ name: "DayHeader" }).vm.$emit("prev-day");
    await flushPromises(); // wait for shiftDay's loadMonth
    expect(store.currentDate).toBe(addDays(todayDateStr(), -1));
  });

  it("next-day is a no-op when the selected day is today", async () => {
    const store = makeStore(); // currentDate === today
    const wrapper = mountView(store);
    wrapper.findComponent({ name: "DayHeader" }).vm.$emit("next-day");
    await wrapper.vm.$nextTick();
    expect(store.currentDate).toBe(todayDateStr());
  });

  it("confirmed navigation away from today discards the draft; returning to today shows an empty composer", async () => {
    // guardUnsaved asks before leaving today with an unsubmitted draft; on
    // confirm, the draft is discarded. Mechanism: the composer only renders
    // for "today", so leaving unmounts it and returning remounts it fresh.
    const confirmSpy = vi.fn(() => true);
    vi.stubGlobal("confirm", confirmSpy);
    try {
      const store = makeStore();
      const wrapper = mountView(store);
      await flushPromises(); // onMounted loadMonth
      const composer = () => wrapper.findComponent({ name: "EntryComposer" });
      expect(composer().exists()).toBe(true);

      await composer().find("input").setValue("draft 30m");
      wrapper.findComponent({ name: "DayHeader" }).vm.$emit("prev-day");
      await flushPromises();

      expect(confirmSpy).toHaveBeenCalled(); // guardUnsaved ran
      expect(store.currentDate).toBe(addDays(todayDateStr(), -1));
      expect(composer().exists()).toBe(false); // unmounted on a non-today date

      wrapper.findComponent({ name: "DayHeader" }).vm.$emit("next-day");
      await flushPromises();

      expect(store.currentDate).toBe(todayDateStr());
      expect(composer().exists()).toBe(true);
      expect((composer().find("input").element as HTMLInputElement).value).toBe("");
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it("passes can-go-next=false to DayHeader when today is selected", () => {
    const store = makeStore();
    const wrapper = mountView(store);
    expect(wrapper.findComponent({ name: "DayHeader" }).props("canGoNext")).toBe(false);
  });

  it("⌘[ moves back one day", async () => {
    const store = makeStore();
    mountView(store);
    await flushPromises(); // wait for onMounted loadMonth
    window.dispatchEvent(new KeyboardEvent("keydown", { code: "BracketLeft", metaKey: true }));
    await flushPromises(); // wait for shiftDay's loadMonth
    expect(store.currentDate).toBe(addDays(todayDateStr(), -1));
  });

  it("⌘⇧[ moves back one month", async () => {
    const store = makeStore();
    const wrapper = mountView(store);
    const expectedMonth = todayDateStr().slice(0, 7); // current YYYY-MM
    window.dispatchEvent(new KeyboardEvent("keydown", { code: "BracketLeft", metaKey: true, shiftKey: true }));
    await wrapper.vm.$nextTick();
    expect(store.currentDate.slice(0, 7)).not.toBe(expectedMonth);
  });

  it("⌘⇧[ moves back one month (real browser key = '{')", async () => {
    const store = makeStore();
    const wrapper = mountView(store);
    const expectedMonth = todayDateStr().slice(0, 7);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "{", code: "BracketLeft", metaKey: true, shiftKey: true }));
    await wrapper.vm.$nextTick();
    expect(store.currentDate.slice(0, 7)).not.toBe(expectedMonth);
  });

  it("⌘⇧] moves forward one month", async () => {
    const store = makeStore();
    const wrapper = mountView(store);
    const expectedMonth = todayDateStr().slice(0, 7);
    window.dispatchEvent(new KeyboardEvent("keydown", { code: "BracketRight", metaKey: true, shiftKey: true }));
    await wrapper.vm.$nextTick();
    expect(store.currentDate.slice(0, 7)).not.toBe(expectedMonth);
  });

  it("⌘⇧] moves forward one month (real browser key = '}')", async () => {
    const store = makeStore();
    const wrapper = mountView(store);
    const expectedMonth = todayDateStr().slice(0, 7);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "}", code: "BracketRight", metaKey: true, shiftKey: true }));
    await wrapper.vm.$nextTick();
    expect(store.currentDate.slice(0, 7)).not.toBe(expectedMonth);
  });

  it("⌘T jumps back to today from another date", async () => {
    const store = makeStore();
    const wrapper = mountView(store);
    await wrapper.vm.$nextTick();
    store.currentDate = "2020-01-15"; // navigate far away
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "t", metaKey: true }));
    await wrapper.vm.$nextTick();
    expect(store.currentDate).toBe(todayDateStr());
  });

  it("⌘T clears entry list when today has no entries (regression: stale data after navigation)", async () => {
    const store = makeStore();
    const today = todayDateStr();

    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "get_month_entries") {
          return { "2026-06-15": { note: null, entries: [makeEntry({ item: "Old task", duration: 30 })] } };
        }
      if (cmd === "get_commitment_progress") return [];
      if (cmd === "get_commitments") return [];
      if (cmd === "get_month_dimensions") return { dimensions: [], usingDefaultDimensions: true };
      if (cmd === "get_entries") return { note: null, entries: [] };
      return {};
    });

    mountView(store);
    await flushPromises();

    // Navigate to a past date that has entries in monthEntries
    store.currentDate = "2026-06-15";
    expect(store.today!.entries).toHaveLength(1);

    // ⌘T → goToToday() → today not in monthEntries → loadMonth → today.entries should be []
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "t", metaKey: true }));
    await flushPromises();
    await flushPromises();

    expect(store.currentDate).toBe(today);
    expect(store.today!.entries).toHaveLength(0);
  });

  it("commitments follow the selected month when navigating to a prior month", async () => {
    const store = makeStore(); // currentDate === today, commitments = [一个 commitment]
    const curYM = todayDateStr().slice(0, 7); // 当前 YYYY-MM

    // 当前月有 commitments；其它任何月为空（模拟「目标月还没建 commitments」）
    invokeMock.mockImplementation(async (cmd: string, args: { year: number; month: number }) => {
      if (cmd === "get_commitment_progress") return [];
      if (cmd === "get_commitments") {
        const ym = `${args.year}-${String(args.month).padStart(2, "0")}`;
        return ym === curYM ? [makeCommitment({ role: "Dev", goals: ["G"] })] : [];
      }
      return { note: null, entries: [] };
    });

    mountView(store);
    await flushPromises(); // 等 onMounted 的 loadMonth(当前月) 跑完
    expect(store.commitments).toHaveLength(1); // 当前月：有数据

    // 切到上一个月（⌘⇧[）—— 该月没有 commitments
    window.dispatchEvent(new KeyboardEvent("keydown", { code: "BracketLeft", metaKey: true, shiftKey: true }));
    await flushPromises();
    await flushPromises(); // belt-and-suspenders: chained async loads must settle before asserting

    expect(store.commitments).toEqual([]); // 跟随目标月：空，而非停留在当前月数据
  });

  it("does not render version string in the DOM (version is now in OS window title)", () => {
    const wrapper = mountView();
    expect(wrapper.text()).not.toMatch(/v\d+\.\d+\.\d+/);
  });

  it("sets the OS window title to Logbook v{version} on mount", () => {
    mountView();
    expect(getCurrentWindow().setTitle).toHaveBeenCalledWith("Logbook v0.0.0");
  });
});

// ---- Delete entry: optimistic delete + 5s timer + undo + failure rollback ----
// (F2: this whole chain was previously untested; F5: the timer read currentDate
//  at fire time, so navigating within the 5s window deleted the wrong day.)
describe("MonthView delete entry", () => {
  const DEL_ID = "del-1";

  // Seed get_month_entries so loadMonth rebuilds store.today with a known entry.
  async function mountWithUndo() {
    const seedDate = todayDateStr();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "delete_entry") return null;
      if (cmd === "get_commitment_progress") return [];
      if (cmd === "get_commitments") return [];
      if (cmd === "get_month_entries") return { [seedDate]: { note: null, entries: [makeEntry({ id: DEL_ID, item: "Doomed", duration: 60 })] } };
      return { note: null, entries: [] };
    });
    const store = makeStore();
    let undoFn: (() => void) | null = null;
    const wrapper = mount(MonthView, {
      global: {
        provide: {
          [STORE_KEY as symbol]: store,
          focusRequestId: { value: 0 },
          [UNDO_TOAST_KEY as symbol]: (fn: () => void) => { undoFn = fn; },
        },
      },
    });
    await flushPromises(); // loadMonth done → store.today holds DEL_ID, stable
    return { wrapper, store, getUndo: () => undoFn };
  }

  function emitDelete(wrapper: ReturnType<typeof mount>, id: string) {
    wrapper.findComponent({ name: "EntryList" }).vm.$emit("delete", id);
  }

  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("optimistically removes the entry, then calls delete_entry after 5s", async () => {
    const { wrapper, store } = await mountWithUndo();
    const date = store.currentDate;

    emitDelete(wrapper, DEL_ID);
    await wrapper.vm.$nextTick();
    // Removed from the list immediately (optimistic), backend not called yet.
    expect(store.today!.entries.find(e => e.id === DEL_ID)).toBeUndefined();
    expect(invokeMock).not.toHaveBeenCalledWith("delete_entry", expect.anything());

    await vi.advanceTimersByTimeAsync(5000);
    expect(invokeMock).toHaveBeenCalledWith(
      "delete_entry",
      expect.objectContaining({ rootPath: "/root", date, entryId: DEL_ID }),
    );
  });

  it("F5: deletes the day the entry was on, not the day navigated to within the 5s window", async () => {
    const { wrapper, store } = await mountWithUndo();
    const originalDate = store.currentDate;

    emitDelete(wrapper, DEL_ID);
    await wrapper.vm.$nextTick();

    // User navigates to a different day before the timer fires.
    store.currentDate = "2026-06-10";

    await vi.advanceTimersByTimeAsync(5000);

    // The delete must target the day the entry actually lived on.
    expect(invokeMock).toHaveBeenCalledWith(
      "delete_entry",
      expect.objectContaining({ date: originalDate, entryId: DEL_ID }),
    );
    expect(invokeMock).not.toHaveBeenCalledWith(
      "delete_entry",
      expect.objectContaining({ date: "2026-06-10" }),
    );
  });

  it("undo before the timer restores the entry and never calls delete_entry", async () => {
    const { wrapper, store, getUndo } = await mountWithUndo();

    emitDelete(wrapper, DEL_ID);
    await wrapper.vm.$nextTick();
    expect(store.today!.entries.find(e => e.id === DEL_ID)).toBeUndefined();

    // User clicks Undo.
    getUndo()!();
    await wrapper.vm.$nextTick();
    expect(store.today!.entries.find(e => e.id === DEL_ID)).toBeDefined();

    await vi.advanceTimersByTimeAsync(5000);
    expect(invokeMock).not.toHaveBeenCalledWith("delete_entry", expect.anything());
  });

  it("re-inserts the entry when the backend delete fails", async () => {
    const { wrapper, store } = await mountWithUndo();
    // Now make the backend reject the delete (mount/load already settled).
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "delete_entry") throw new Error("backend rejected");
      if (cmd === "get_commitment_progress") return [];
      return { note: null, entries: [] };
    });

    emitDelete(wrapper, DEL_ID);
    await wrapper.vm.$nextTick();
    expect(store.today!.entries.find(e => e.id === DEL_ID)).toBeUndefined();

    await vi.advanceTimersByTimeAsync(5000);
    await flushPromises();

    // Failed delete must restore the entry rather than silently drop it.
    expect(store.today!.entries.find(e => e.id === DEL_ID)).toBeDefined();
  });
});


// ---- Stale-date cache writes: await resolves after the user navigated ----
// handleSubmit / handleUpdateEntry / handleUpdateDimensions used to read
// store.currentDate AFTER the IPC await. If the user switches day while the
// call is in flight, the result is written into the newly-selected day's
// cache (phantom entry / old day's DayFile clobbering the new day's view).
// handleDeleteEntry already captures the date on entry (see F5 above); these
// three must behave the same way.
describe("MonthView stale-date writes after in-flight IPC", () => {
  // A day in the same month as dateStr, guaranteed different from it (an
  // in-month navigation, so no month reload is involved).
  function sameMonthOtherDay(dateStr: string): string {
    const day = parseInt(dateStr.slice(8, 10), 10);
    const other = day === 15 ? 16 : 15;
    return `${dateStr.slice(0, 8)}${String(other).padStart(2, "0")}`;
  }

  const existing = () => makeEntry({ item: "Existing", duration: 60 });

  function mockMonthWithEntry(origDate: string, entry = existing()) {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "get_commitment_progress") return [];
      if (cmd === "get_commitments") return [];
      if (cmd === "get_month_entries") return { [origDate]: { note: null, entries: [entry] } };
      return { note: null, entries: [] };
    });
    return entry;
  }

  it("submit lands on the ORIGINAL day when the user navigates while append_entry is in flight", async () => {
    const origDate = todayDateStr();
    const otherDate = sameMonthOtherDay(origDate);
    mockMonthWithEntry(origDate);
    const store = makeStore();
    const wrapper = mountView(store);
    await flushPromises(); // loadMonth done

    let resolveAppend!: (v: unknown) => void;
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "append_entry") return new Promise((r) => { resolveAppend = r; });
      if (cmd === "get_commitment_progress") return [];
      return { note: null, entries: [] };
    });
    const added = makeEntry({ item: "New task", duration: 30 });
    wrapper.findComponent({ name: "EntryComposer" }).vm.$emit("submit", "New task", 30, {});
    await wrapper.vm.$nextTick();
    expect(invokeMock).toHaveBeenCalledWith("append_entry", expect.objectContaining({ date: origDate }));

    // User switches day BEFORE the IPC resolves.
    store.currentDate = otherDate;
    resolveAppend(added);
    await flushPromises();

    // The entry must land on the day it was submitted from ...
    expect(store.monthEntries[origDate]?.find((e) => e.id === added.id)).toBeDefined();
    // ... and must NOT appear on the day the user navigated to (phantom entry).
    expect(store.monthEntries[otherDate]?.find((e) => e.id === added.id)).toBeUndefined();
    expect(store.today!.entries.find((e) => e.id === added.id)).toBeUndefined();
  });

  it("update lands on the ORIGINAL day when the user navigates while update_entry is in flight", async () => {
    const origDate = todayDateStr();
    const otherDate = sameMonthOtherDay(origDate);
    const entry = mockMonthWithEntry(origDate);
    const store = makeStore();
    const wrapper = mountView(store);
    await flushPromises();

    let resolveUpdate!: (v: unknown) => void;
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "update_entry") return new Promise((r) => { resolveUpdate = r; });
      if (cmd === "get_commitment_progress") return [];
      return { note: null, entries: [] };
    });
    wrapper.findComponent({ name: "EntryList" }).vm.$emit("update", entry.id, "Edited", 45);
    await wrapper.vm.$nextTick();
    expect(invokeMock).toHaveBeenCalledWith("update_entry", expect.objectContaining({ date: origDate, entryId: entry.id }));

    store.currentDate = otherDate;
    const updatedEntries = [makeEntry({ id: entry.id, item: "Edited", duration: 45 })];
    resolveUpdate({ note: null, entries: updatedEntries });
    await flushPromises();

    expect(store.monthEntries[origDate]).toEqual(updatedEntries);
    expect(store.monthEntries[otherDate]).toBeUndefined();
  });

  it("dimension update lands on the ORIGINAL day when the user navigates while update_entry is in flight", async () => {
    const origDate = todayDateStr();
    const otherDate = sameMonthOtherDay(origDate);
    const entry = mockMonthWithEntry(origDate);
    const store = makeStore();
    const wrapper = mountView(store);
    await flushPromises();

    let resolveUpdate!: (v: unknown) => void;
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "update_entry") return new Promise((r) => { resolveUpdate = r; });
      if (cmd === "get_commitment_progress") return [];
      return { note: null, entries: [] };
    });
    wrapper.findComponent({ name: "EntryList" }).vm.$emit("updateDimensions", entry.id, { goal: "Ship feature X" });
    await wrapper.vm.$nextTick();
    expect(invokeMock).toHaveBeenCalledWith("update_entry", expect.objectContaining({ date: origDate, entryId: entry.id }));

    store.currentDate = otherDate;
    const updatedEntries = [makeEntry({ id: entry.id, item: "Existing", duration: 60 })];
    resolveUpdate({ note: null, entries: updatedEntries });
    await flushPromises();

    expect(store.monthEntries[origDate]).toEqual(updatedEntries);
    expect(store.monthEntries[otherDate]).toBeUndefined();
  });
});
