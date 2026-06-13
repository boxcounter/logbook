import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import { makeConfig, makeDayFile } from "../mocks/fixtures";
import DateNavigator from "../../components/DateNavigator.vue";

const { mockInvoke, mockLogError } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockLogError: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("../../utils/errorLog", () => ({ logError: mockLogError }));

const TODAY = new Date();
const todayStr = `${TODAY.getFullYear()}-${String(TODAY.getMonth() + 1).padStart(2, "0")}-${String(TODAY.getDate()).padStart(2, "0")}`;

function mountNav(overrides?: Parameters<typeof createTestStore>[0]) {
  const store = createTestStore({
    rootPath: "/test",
    currentDate: todayStr,
    today: makeDayFile({ note: null }),
    config: makeConfig(),
    ...overrides,
  });
  const wrapper = mount(DateNavigator, {
    global: {
      provide: { [STORE_KEY as symbol]: store },
      stubs: { transition: true },
    },
  });
  return { wrapper, store };
}

// ============================================================

describe("DateNavigator", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(makeDayFile());
  });

  it("renders left and right arrow buttons", () => {
    const { wrapper } = mountNav();
    const buttons = wrapper.findAll("button");
    expect(buttons).toHaveLength(2);
    expect(buttons[0].text()).toContain("←");
    expect(buttons[1].text()).toContain("→");
  });

  it("renders granularity select with day/week/month options", () => {
    const { wrapper } = mountNav();
    const select = wrapper.find("select");
    const options = select.findAll("option");
    const texts = options.map(o => o.text());
    expect(texts).toContain("Day");
    expect(texts).toContain("Week");
    expect(texts).toContain("Month");
  });

  it('day mode: shows "Today" label when currentDate is today', () => {
    const { wrapper } = mountNav({ currentDate: todayStr, granularity: "day" });
    expect(wrapper.text()).toContain("Today");
  });

  it("day mode: shows date label for other days", () => {
    // Use a date far from today
    const { wrapper } = mountNav({ currentDate: "2020-01-15", granularity: "day" });
    expect(wrapper.text()).not.toContain("Today");
    expect(wrapper.text()).not.toContain("Yesterday");
    expect(wrapper.text()).not.toContain("Tomorrow");
  });

  it("week mode: shows week label", () => {
    const { wrapper } = mountNav({ currentDate: "2026-06-10", granularity: "week" });
    // Should show something like "Jun 8 – Jun 14"
    const text = wrapper.text();
    expect(text).toContain("Jun");
    expect(text).toContain("–");
  });

  it("month mode: shows month and year", () => {
    const { wrapper } = mountNav({ currentDate: "2026-06-15", granularity: "month" });
    expect(wrapper.text()).toContain("June");
    expect(wrapper.text()).toContain("2026");
  });

  it("click left arrow: shifts date and calls invoke get_entries", async () => {
    const { wrapper, store } = mountNav({ currentDate: "2026-06-15", granularity: "day" });
    const initialDate = store.currentDate;

    const leftBtn = wrapper.findAll("button")[0];
    await leftBtn.trigger("click");

    expect(store.currentDate).not.toBe(initialDate);
    expect(mockInvoke).toHaveBeenCalledWith("get_entries", expect.objectContaining({
      rootPath: "/test",
    }));
  });

  it("click right arrow: shifts date forward", async () => {
    const { wrapper } = mountNav({ currentDate: "2026-06-15", granularity: "day" });
    const rightBtn = wrapper.findAll("button")[1];
    await rightBtn.trigger("click");

    expect(mockInvoke).toHaveBeenCalled();
    expect(wrapper.emitted("navigate")).toBeTruthy();
  });

  it("granularity change: updates store and reloads", async () => {
    const { wrapper, store } = mountNav({ granularity: "day" });
    const select = wrapper.find("select");

    await select.setValue("week");
    expect(store.granularity).toBe("week");
  });

  it("note save on blur: calls invoke set_day_note", async () => {
    const { wrapper } = mountNav({ rootPath: "/data" });
    const noteEl = wrapper.find("[contenteditable]");

    // Set textContent directly (jsdom contenteditable limitation)
    (noteEl.element as HTMLElement).textContent = "My note";
    await noteEl.trigger("blur");

    expect(mockInvoke).toHaveBeenCalledWith("set_day_note", expect.objectContaining({
      rootPath: "/data",
      note: "My note",
    }));
  });

  it("note syncs when store.today.note changes", async () => {
    // Mount with null note, then change it — the watcher should sync to DOM
    const { wrapper, store } = mountNav({ today: makeDayFile({ note: null }) });
    store.today = { note: "Updated note", entries: [] };
    await wrapper.vm.$nextTick();

    const noteEl = wrapper.find("[contenteditable]");
    expect(noteEl.element.textContent).toBe("Updated note");
  });

  it("emits navigate after loadDay succeeds", async () => {
    const { wrapper } = mountNav({ currentDate: "2026-06-15", granularity: "day" });
    const rightBtn = wrapper.findAll("button")[1];
    await rightBtn.trigger("click");

    expect(wrapper.emitted("navigate")).toBeTruthy();
  });

  it("logs error when loadDay fails", async () => {
    mockInvoke.mockRejectedValue("Network error");
    const { wrapper } = mountNav({ currentDate: "2026-06-15", granularity: "day" });
    const rightBtn = wrapper.findAll("button")[1];
    await rightBtn.trigger("click");

    expect(mockLogError).toHaveBeenCalled();
  });
});
