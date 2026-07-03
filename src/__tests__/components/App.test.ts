import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick } from "vue";
import { STORE_KEY } from "../../stores/useStore";
import { UNDO_TOAST_KEY, SAVED_TOAST_KEY } from "../../types";
import { createTestStore } from "../mocks/store";
import { makeDimensions, makeDayFile, makeCommitment, makeDimension } from "../mocks/fixtures";
import App from "../../App.vue";
import Toast from "../../components/base/Toast.vue";

// Hoisted mocks for Tauri APIs
const { mockInvoke, mockListen, mockGetCurrentWindow, mockLogError, mockLogInfo } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockListen: vi.fn(),
  mockGetCurrentWindow: vi.fn(),
  mockLogError: vi.fn(),
  mockLogInfo: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mockListen }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: mockGetCurrentWindow }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("../../utils/errorLog", () => ({ logError: mockLogError, logInfo: mockLogInfo }));

// Track registered event callbacks so tests can fire them
let dimensionsChangedCallback: ((event: { payload: unknown }) => void) | null = null;
let commitmentsChangedCallback: (() => void) | null = null;
let focusChangedCallback: (({ payload }: { payload: boolean }) => void) | null = null;

function mountApp(extraStubs: Record<string, unknown> = {}) {
  const store = createTestStore();
  const wrapper = mount(App, {
    // rolloverIntervalMs: 0 → no recurring timer, so vi.runAllTimersAsync()
    // flush loops in these tests never trap on it.
    props: { rolloverIntervalMs: 0 },
    global: {
      provide: { [STORE_KEY as symbol]: store },
      stubs: { transition: true, Teleport: true, ...extraStubs },
    },
  });
  return { wrapper, store };
}

const InjectProbe = {
  name: "InjectProbe",
  inject: {
    triggerUndoToast: { from: UNDO_TOAST_KEY },
    triggerSavedToast: { from: SAVED_TOAST_KEY },
  },
  template: "<div />",
};

// ============================================================

describe("App", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    dimensionsChangedCallback = null;
    commitmentsChangedCallback = null;
    focusChangedCallback = null;

    // Default mocks
    mockInvoke.mockResolvedValue({ status: "NeedsSetup" });
    // listen returns unlisten function and stores the callback
    mockListen.mockImplementation(async (event: string, cb: unknown) => {
      if (event === "dimensions-changed") dimensionsChangedCallback = cb as typeof dimensionsChangedCallback;
      if (event === "commitments-changed") commitmentsChangedCallback = cb as typeof commitmentsChangedCallback;
      return () => {
        if (event === "dimensions-changed") dimensionsChangedCallback = null;
        if (event === "commitments-changed") commitmentsChangedCallback = null;
      };
    });
    // getCurrentWindow returns onFocusChanged that stores callback
    mockGetCurrentWindow.mockReturnValue({
      onFocusChanged: vi.fn().mockImplementation(async (cb: unknown) => {
        focusChangedCallback = cb as typeof focusChangedCallback;
        return () => { focusChangedCallback = null; };
      }),
      setTitle: vi.fn(),
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // ---- Initial screen state ----

  it('shows "Loading…" on initial mount', () => {
    const { wrapper } = mountApp();
    expect(wrapper.text()).toContain("Loading");
  });

  it("calls invoke init on mount", async () => {
    mountApp();
    await vi.runAllTimersAsync();
    expect(mockInvoke).toHaveBeenCalledWith("init");
  });

  it("NeedsSetup: shows SetupScreen", async () => {
    mockInvoke.mockResolvedValue({ status: "NeedsSetup" });
    const { wrapper, store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    expect(store.status).toBe("setup");
    expect(wrapper.findComponent({ name: "SetupScreen" }).exists()).toBe(true);
  });

  it("ConfigError (in_place): shows RecoveryScreen with error message and no Retry", async () => {
    mockInvoke.mockResolvedValue({
      status: "ConfigError",
      data: {
        category: "in_place",
        root_path: "/test",
        errors: [{ kind: "MissingName", message: "Dimension 0 has an empty name" }],
        scan_warnings: [],
      },
    });
    const { wrapper, store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    expect(store.status).toBe("error");
    expect(store.configCategory).toBe("in_place");
    expect(wrapper.findComponent({ name: "RecoveryScreen" }).exists()).toBe(true);
    expect(wrapper.text()).toContain("Dimension 0 has an empty name");
    expect(wrapper.text()).not.toContain("Retry");
  });

  it("Ready: shows MonthView and populates store", async () => {
    const dimensions = makeDimensions();
    const today = makeDayFile();
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", dimensions, usingDefaultDimensions: false, today, commitments: [], scan_warnings: [] },
    });
    const { wrapper, store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    expect(store.status).toBe("ready");
    expect(store.rootPath).toBe("/test");
    expect(store.dimensions).toEqual(dimensions);
    expect(wrapper.findComponent({ name: "MonthView" }).exists()).toBe(true);
  });

  it("Init failure: shows error screen with InitError", async () => {
    mockInvoke.mockRejectedValue("Network error");
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    expect(store.status).toBe("error");
    expect(store.configErrors[0].kind).toBe("InitError");
  });

  it("Retry button re-calls initApp", async () => {
    mockInvoke.mockResolvedValue({
      status: "ConfigError",
      data: {
        category: "root_missing",
        root_path: "/test",
        errors: [{ kind: "MissingName", message: "err" }],
        scan_warnings: [],
      },
    });
    const { wrapper } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    vi.clearAllMocks();
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", dimensions: makeDimensions(), usingDefaultDimensions: false, today: makeDayFile(), commitments: [], scan_warnings: [] },
    });

    await wrapper.get('[data-testid="retry"]').trigger("click");
    await vi.runAllTimersAsync();

    expect(mockInvoke).toHaveBeenCalledWith("init");
  });

  // ---- Event listeners ----

  it("registers event listeners on mount", async () => {
    mountApp();
    // onMounted runs async, wait for it
    await vi.runAllTimersAsync();
    await nextTick();

    expect(mockListen).toHaveBeenCalledWith("dimensions-changed", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("commitments-changed", expect.any(Function));
  });

  it("calls unlisten on unmount", async () => {
    // Distinct spies for each listener so we can assert each is cleaned up.
    const unlistenDimensions = vi.fn();
    const unlistenCommitments = vi.fn();
    const unlistenFocus = vi.fn();
    mockListen.mockImplementation(async (event: string) => {
      if (event === "dimensions-changed") return unlistenDimensions;
      if (event === "commitments-changed") return unlistenCommitments;
      return vi.fn();
    });
    mockGetCurrentWindow.mockReturnValue({
      onFocusChanged: vi.fn().mockResolvedValue(unlistenFocus),
      setTitle: vi.fn(),
    });

    const { wrapper } = mountApp();
    // Let onMounted finish so the unlisten handles are actually assigned
    // (they're awaited; before resolution they're still null).
    await vi.runAllTimersAsync();
    await nextTick();

    wrapper.unmount();

    expect(unlistenDimensions).toHaveBeenCalledTimes(1);
    expect(unlistenCommitments).toHaveBeenCalledTimes(1);
    expect(unlistenFocus).toHaveBeenCalledTimes(1);
  });

  it("dimensions-changed event with no errors reloads dimensions for current month", async () => {
    vi.setSystemTime(new Date(2026, 5, 20, 10, 0, 0));
    const dims = makeDimensions();
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", dimensions: dims, usingDefaultDimensions: false, today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    vi.clearAllMocks();
    const newDims = [makeDimension({ name: "Updated", key: "updated", source: "static", required: false })];
    mockInvoke.mockResolvedValue({ dimensions: newDims, usingDefaultDimensions: true });

    // Simulate dimensions-changed event with empty error list
    if (dimensionsChangedCallback) {
      dimensionsChangedCallback({ payload: [] });
      await vi.runAllTimersAsync();
    }

    expect(mockInvoke).toHaveBeenCalledWith("get_month_dimensions", expect.objectContaining({ year: 2026, month: 6 }));
    expect(store.dimensions).toEqual(newDims);
    expect(store.usingDefaultDimensions).toBe(true);
  });

  it("dimensions-changed event with errors shows error screen", async () => {
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", dimensions: makeDimensions(), usingDefaultDimensions: false, today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    if (dimensionsChangedCallback) {
      const errors = [{ kind: "MissingName", message: "Bad config" }];
      dimensionsChangedCallback({ payload: errors });
    }

    expect(store.status).toBe("error");
    expect(store.configCategory).toBe("in_place");
    expect(store.configErrors).toEqual([{ kind: "MissingName", message: "Bad config" }]);
  });

  it("commitments-changed reloads the SELECTED month's commitments and does NOT call init", async () => {
    vi.setSystemTime(new Date(2026, 5, 20, 10, 0, 0)); // 当前月 = 2026-06
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", dimensions: makeDimensions(), usingDefaultDimensions: false, today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    store.currentDate = "2026-07-15"; // 用户切到 7 月（无 commitments）
    store.status = "ready";
    store.commitments = [makeCommitment()]; // 模拟此刻残留的「当前月」数据
    vi.clearAllMocks();

    // 按月路由：7 月空，其它月有数据
    mockInvoke.mockImplementation(async (cmd: string, args: { month: number }) => {
      if (cmd === "get_month_dimensions") return { dimensions: makeDimensions(), usingDefaultDimensions: false };
      if (cmd === "get_commitments") return args.month === 7 ? [] : [makeCommitment()];
      if (cmd === "get_commitment_progress") return [];
      return undefined;
    });

    commitmentsChangedCallback?.();
    await vi.runAllTimersAsync();

    expect(mockInvoke).not.toHaveBeenCalledWith("init");
    expect(mockInvoke).toHaveBeenCalledWith("get_commitments", expect.objectContaining({ year: 2026, month: 7 }));
    expect(store.commitments).toEqual([]); // 跟随 7 月，未被冲回当前月数据
  });

  it("commitments-changed is a no-op when status is not ready", async () => {
    mockInvoke.mockResolvedValue({ status: "NeedsSetup" });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    store.status = "setup";
    vi.clearAllMocks();

    commitmentsChangedCallback?.();
    await vi.runAllTimersAsync();

    expect(mockInvoke).not.toHaveBeenCalledWith("get_commitments");
    expect(mockInvoke).not.toHaveBeenCalledWith("get_commitment_progress");
  });

  it("dimensions-changed reloads dimensions without overwriting commitments", async () => {
    vi.setSystemTime(new Date(2026, 5, 20, 10, 0, 0));
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", dimensions: makeDimensions(), usingDefaultDimensions: false, today: makeDayFile(), commitments: [makeCommitment()], scan_warnings: [] },
    });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    store.currentDate = "2026-07-15";
    const sentinel = [makeCommitment({ role: "JulyOnly" })];
    store.commitments = sentinel; // 选中月（7 月）当前持有的数据
    vi.clearAllMocks();
    const newDims = [makeDimension({ name: "NewDim", key: "new", source: "static", required: false })];
    mockInvoke.mockResolvedValue({ dimensions: newDims, usingDefaultDimensions: true });

    dimensionsChangedCallback?.({ payload: [] }); // 触发 get_month_dimensions
    await vi.runAllTimersAsync();

    expect(store.dimensions).toEqual(newDims);
    expect(store.usingDefaultDimensions).toBe(true);
    expect(store.commitments).toStrictEqual(sentinel); // commitments untouched
  });

  // ---- Focus / midnight crossing ----

  function ymd(d: Date): string {
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }

  it("refocus on the same day does NOT reset the selected date", async () => {
    vi.setSystemTime(new Date(2026, 5, 20, 10, 0, 0));
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", dimensions: makeDimensions(), usingDefaultDimensions: false, today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    store.currentDate = "2026-06-12"; // user navigated to a past day
    vi.clearAllMocks();

    focusChangedCallback?.({ payload: true });
    await vi.runAllTimersAsync();

    expect(store.currentDate).toBe("2026-06-12");
    expect(mockInvoke).not.toHaveBeenCalledWith("init");
  });

  it("midnight crossing while viewing today FOLLOWS to the new today", async () => {
    vi.setSystemTime(new Date(2026, 5, 20, 23, 59, 0));
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", dimensions: makeDimensions(), usingDefaultDimensions: false, today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    expect(store.currentDate).toBe(ymd(new Date(2026, 5, 20)));
    store.status = "ready";
    vi.setSystemTime(new Date(2026, 5, 21, 0, 1, 0)); // crossed midnight
    focusChangedCallback?.({ payload: true });
    await vi.runAllTimersAsync();

    expect(store.currentDate).toBe(ymd(new Date(2026, 5, 21)));
    expect(mockInvoke).toHaveBeenCalledWith("init");
  });

  it("midnight crossing while viewing another day STAYS put", async () => {
    vi.setSystemTime(new Date(2026, 5, 20, 23, 59, 0));
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", dimensions: makeDimensions(), usingDefaultDimensions: false, today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    store.currentDate = "2026-06-12"; // viewing a different day
    store.status = "ready";
    vi.clearAllMocks();
    vi.setSystemTime(new Date(2026, 5, 21, 0, 1, 0));
    focusChangedCallback?.({ payload: true });
    await vi.runAllTimersAsync();

    expect(store.currentDate).toBe("2026-06-12");
    expect(mockInvoke).not.toHaveBeenCalledWith("init");
  });

  it("F6: periodic timer rolls the view over at midnight WITHOUT a focus event", async () => {
    // The bug: a window focused across midnight never fires onFocusChanged, so
    // the view stayed on yesterday. The interval must advance it on its own.
    vi.setSystemTime(new Date(2026, 5, 20, 23, 59, 0));
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", dimensions: makeDimensions(), usingDefaultDimensions: false, today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    const store = createTestStore();
    const wrapper = mount(App, {
      props: { rolloverIntervalMs: 1000 }, // live timer for this test only
      global: {
        provide: { [STORE_KEY as symbol]: store },
        stubs: { transition: true, Teleport: true },
      },
    });
    await flushPromises();
    await nextTick();

    expect(store.currentDate).toBe(ymd(new Date(2026, 5, 20)));
    store.status = "ready";
    vi.clearAllMocks();

    // Cross midnight with NO focus event — only the interval drives the change.
    vi.setSystemTime(new Date(2026, 5, 21, 0, 0, 30));
    await vi.advanceTimersByTimeAsync(1000);
    await flushPromises();

    expect(store.currentDate).toBe(ymd(new Date(2026, 5, 21)));
    expect(mockInvoke).toHaveBeenCalledWith("init");

    wrapper.unmount();
  });

  it("triggerUndoToast: shows undo toast with Undo and Dismiss buttons", async () => {
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", dimensions: makeDimensions(), usingDefaultDimensions: false, today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    // Replace MonthView with a probe that captures App's provided triggerUndoToast.
    const { wrapper } = mountApp({ MonthView: InjectProbe });
    await vi.runAllTimersAsync();
    await nextTick();

    const probe = wrapper.findComponent(InjectProbe);
    const trigger = probe.vm.triggerUndoToast as (fn: () => void) => void;
    expect(typeof trigger).toBe("function");

    // Initially hidden
    expect(wrapper.text()).not.toContain("Entry deleted");

    trigger(() => {});
    await nextTick();

    // Toast now visible with both the message and Undo/Dismiss buttons.
    const undoToast = wrapper
      .findAllComponents(Toast)
      .find((t) => t.props("message") === "Entry deleted");
    expect(undoToast?.props("show")).toBe(true);
    expect(undoToast?.props("undoLabel")).toBe("Undo");
    expect(wrapper.text()).toContain("Entry deleted");
    expect(wrapper.text()).toContain("Undo");
  });

  it("undo toast: auto-dismisses after 5 seconds", async () => {
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", dimensions: makeDimensions(), usingDefaultDimensions: false, today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    const { wrapper } = mountApp({ MonthView: InjectProbe });
    await vi.runAllTimersAsync();
    await nextTick();

    const probe = wrapper.findComponent(InjectProbe);
    (probe.vm.triggerUndoToast as (fn: () => void) => void)(() => {});
    await nextTick();

    const undoToast = () =>
      wrapper.findAllComponents(Toast).find((t) => t.props("message") === "Entry deleted");
    expect(undoToast()?.props("show")).toBe(true);

    // Advance past the 5s undo window.
    vi.advanceTimersByTime(5000);
    await nextTick();

    expect(undoToast()?.props("show")).toBe(false);
  });

  // ---- Scan warning toast ----

  it("shows scan warning toast when Ready has scan_warnings", async () => {
    const scanWarnings = [
      { message: "Data file '2025-01-01.json' has invalid JSON, skipping" },
      { message: "Data file 'corrupt.json' could not be read" },
    ];
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: {
        root_path: "/test",
        dimensions: makeDimensions(),
        usingDefaultDimensions: false,
        today: makeDayFile(),
        commitments: [],
        scan_warnings: scanWarnings,
      },
    });
    const { wrapper } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    // Toast should show the count of data issues
    expect(wrapper.text()).toContain("data issue");
    // MonthView should still be shown (non-blocking toast)
    expect(wrapper.findComponent({ name: "MonthView" }).exists()).toBe(true);
  });

  it("shows scan warning toast when ConfigError has scan_warnings", async () => {
    const scanWarnings = [
      { message: "Data file '2025-01-01.json' has invalid JSON, skipping" },
    ];
    mockInvoke.mockResolvedValue({
      status: "ConfigError",
      data: {
        category: "in_place",
        root_path: "/test",
        errors: [{ kind: "MissingName", message: "Bad config" }],
        scan_warnings: scanWarnings,
      },
    });
    const { wrapper } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    // Toast should appear even in error state
    expect(wrapper.text()).toContain("data issue");
    // Recovery screen should still show
    expect(wrapper.findComponent({ name: "RecoveryScreen" }).exists()).toBe(true);
  });

  it("does not show scan warning toast when scan_warnings is empty", async () => {
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: {
        root_path: "/test",
        dimensions: makeDimensions(),
        usingDefaultDimensions: false,
        today: makeDayFile(),
        commitments: [],
        scan_warnings: [],
      },
    });
    const { wrapper } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    expect(wrapper.text()).not.toContain("data issue");
  });

  it("dismiss button hides scan warning toast", async () => {
    const scanWarnings = [
      { message: "Data file '2025-01-01.json' has invalid JSON, skipping" },
    ];
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: {
        root_path: "/test",
        dimensions: makeDimensions(),
        usingDefaultDimensions: false,
        today: makeDayFile(),
        commitments: [],
        scan_warnings: scanWarnings,
      },
    });
    const { wrapper } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    // Toast should be visible
    expect(wrapper.text()).toContain("data issue");

    // Find the scan-warning Toast by its message (the only one mentioning
    // "data issue") and emit its dismiss event.
    const scanToast = wrapper
      .findAllComponents(Toast)
      .find((t) => String(t.props("message")).includes("data issue"));
    expect(scanToast?.props("show")).toBe(true);

    scanToast!.vm.$emit("dismiss");
    await nextTick();

    expect(scanToast!.props("show")).toBe(false);
    expect(wrapper.text()).not.toContain("data issue");
  });
});
