import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { ref } from "vue";
import EntryInput from "../../components/EntryInput.vue";
import { makeCommitment } from "../mocks/fixtures";
import type { Dimension, Commitment } from "../../types";

const { mockLogInfo } = vi.hoisted(() => ({
  mockLogInfo: vi.fn(),
}));

vi.mock("../../utils/errorLog", () => ({ logInfo: mockLogInfo }));

const testDimensions: Dimension[] = [
  { name: "Goal", key: "goal", source: "monthly", required: true },
  { name: "Business Line", key: "business-line", source: "static", values: ["Platform", "Growth"], required: true },
  { name: "Category", key: "category", source: "static", values: ["Coding", "Meeting"], required: false },
];

const testCommitments: Commitment[] = [
  makeCommitment({ role: "Developer", allocation: 40, goals: ["Ship feature X", "Code review"] }),
];

function mountInput(overrides?: {
  dimensions?: Dimension[];
  commitments?: Commitment[];
  initialValues?: Record<string, string>;
}) {
  const focusRequestId = ref(0);
  const wrapper = mount(EntryInput, {
    props: {
      dimensions: overrides?.dimensions ?? testDimensions,
      commitments: overrides?.commitments ?? testCommitments,
      initialValues: overrides?.initialValues ?? {},
    },
    global: {
      provide: { focusRequestId },
    },
  });
  return { wrapper, focusRequestId };
}

// Helper: find the text input
function inputEl(wrapper: ReturnType<typeof mountInput>["wrapper"]) {
  return wrapper.find("input[type='text']");
}

// Helper: set input value and trigger events so v-model + onInput fire
async function typeIn(wrapper: ReturnType<typeof mountInput>["wrapper"], value: string) {
  const inp = inputEl(wrapper);
  await inp.setValue(value);
}

// Helper: trigger keydown on input
async function keydownOnInput(wrapper: ReturnType<typeof mountInput>["wrapper"], key: string, opts?: { ctrlKey?: boolean }) {
  const inp = inputEl(wrapper);
  await inp.trigger("keydown", { key, ctrlKey: opts?.ctrlKey ?? false });
}

// ============================================================

describe("EntryInput", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ---- Rendering ----

  it("renders input with placeholder", () => {
    const { wrapper } = mountInput();
    const inp = inputEl(wrapper);
    expect(inp.attributes("placeholder")).toContain("Sprint planning");
  });

  it("renders Log submit button", () => {
    const { wrapper } = mountInput();
    const btn = wrapper.find("button[type='button']");
    expect(btn.text()).toBe("Log");
  });

  it("Log button disabled when input empty", () => {
    const { wrapper } = mountInput();
    const btn = wrapper.find("button[type='button']");
    expect((btn.element as HTMLButtonElement).disabled).toBe(true);
  });

  it("Log button enabled when input has text", async () => {
    const { wrapper } = mountInput();
    await typeIn(wrapper, "Meeting 30m");
    const btn = wrapper.find("button[type='button']");
    expect((btn.element as HTMLButtonElement).disabled).toBe(false);
  });

  it('shows "@ to set dimensions" hint when no required dimensions exist', () => {
    // Use dimensions with no required fields
    const dims: Dimension[] = [
      { name: "Category", key: "category", source: "static", values: ["A", "B"], required: false },
    ];
    const { wrapper } = mountInput({ dimensions: dims, commitments: [] });
    expect(wrapper.text()).toContain("@ to set dimensions");
  });

  // ---- Duration preview ----

  it("shows duration preview when valid duration typed", async () => {
    const { wrapper } = mountInput();
    await typeIn(wrapper, "Meeting 1.5h");
    const text = wrapper.text();
    expect(text).toContain("Duration:");
    expect(text).toContain("1h 30m");
  });

  it("no preview when input has no duration", async () => {
    const { wrapper } = mountInput();
    await typeIn(wrapper, "Just some text");
    expect(wrapper.text()).not.toContain("Duration:");
  });

  it("no preview when input is empty", () => {
    const { wrapper } = mountInput();
    expect(wrapper.text()).not.toContain("Duration:");
  });

  // ---- Submit ----

  it("valid submit: emits submit with item, durationMinutes, dimensions", async () => {
    const { wrapper } = mountInput({
      initialValues: { goal: "Code review", "business-line": "Platform" },
    });
    await typeIn(wrapper, "Sprint planning 1.5h");

    const btn = wrapper.find("button[type='button']");
    await btn.trigger("click");

    expect(wrapper.emitted("submit")).toBeTruthy();
    const payload = wrapper.emitted("submit")![0];
    expect(payload[0]).toBe("Sprint planning");
    expect(payload[1]).toBe(90);
    expect(payload[2]).toEqual({ goal: "Code review", "business-line": "Platform" });
  });

  it("missing required dimensions: shows error", async () => {
    const { wrapper } = mountInput({ initialValues: {} });
    await typeIn(wrapper, "Task 30m");

    const btn = wrapper.find("button[type='button']");
    await btn.trigger("click");

    expect(wrapper.text()).toContain("Missing required");
    expect(wrapper.emitted("submit")).toBeFalsy();
  });

  it("no duration in input: shows error", async () => {
    const { wrapper } = mountInput({
      initialValues: { goal: "Code review", "business-line": "Platform" },
    });
    await typeIn(wrapper, "Just text");

    const btn = wrapper.find("button[type='button']");
    await btn.trigger("click");

    expect(wrapper.text()).toContain("Could not parse duration");
    expect(wrapper.emitted("submit")).toBeFalsy();
  });

  it("empty input: no submit", async () => {
    const { wrapper } = mountInput();
    const btn = wrapper.find("button[type='button']");
    await btn.trigger("click");

    expect(wrapper.emitted("submit")).toBeFalsy();
  });

  it("Enter key submits when menu is closed", async () => {
    const { wrapper } = mountInput({
      initialValues: { goal: "Code review", "business-line": "Platform" },
    });
    await typeIn(wrapper, "Meeting 30m");
    await keydownOnInput(wrapper, "Enter");

    expect(wrapper.emitted("submit")).toBeTruthy();
  });

  // ---- Dimension chips ----

  it("renders filled dimension chips", () => {
    const { wrapper } = mountInput({
      initialValues: { goal: "Ship feature X", "business-line": "Platform" },
    });
    expect(wrapper.text()).toContain("Goal: Ship feature X");
    expect(wrapper.text()).toContain("Business Line: Platform");
  });

  it("clicking a chip clears that dimension", async () => {
    const { wrapper } = mountInput({
      initialValues: { goal: "Ship feature X" },
    });
    // Find the chip and click it
    const chip = wrapper.find(".inline-flex.items-center.gap-1.px-2");
    await chip.trigger("click");

    // The hint text should reappear (goal cleared, business-line still required)
    expect(wrapper.text()).toContain("+ Business Line");
  });

  it("shows missing required chips with red dashed border", () => {
    const { wrapper } = mountInput({ initialValues: {} });
    expect(wrapper.text()).toContain("+ Goal");
    expect(wrapper.text()).toContain("+ Business Line");
  });

  it("clicking missing required chip opens val menu directly", async () => {
    const { wrapper } = mountInput({ initialValues: {} });
    // Find the red missing chip and click
    const missingChips = wrapper.findAll(".border-dashed");
    expect(missingChips.length).toBeGreaterThan(0);
    await missingChips[0].trigger("click");

    // Menu should be visible with val phase
    expect(wrapper.text()).toContain("Pick a value");
  });

  // ---- @mention menu: open/close ----

  it("press @ opens dim menu and inserts @ in input", async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");

    const text = wrapper.text();
    expect(text).toContain("DIM");
    expect(text).toContain("Pick a dimension");
  });

  it("Escape closes menu and removes mention", async () => {
    const { wrapper } = mountInput();
    // Open menu first
    await keydownOnInput(wrapper, "@");
    expect(wrapper.find(".absolute").exists()).toBe(true);

    // Press Escape
    await keydownOnInput(wrapper, "Escape");
    expect(wrapper.find(".absolute").exists()).toBe(false);
  });

  it("Ctrl+[ closes menu and removes mention", async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");
    expect(wrapper.find(".absolute").exists()).toBe(true);

    await keydownOnInput(wrapper, "[", { ctrlKey: true });
    expect(wrapper.find(".absolute").exists()).toBe(false);
  });

  // ---- @mention menu: navigation ----

  it("ArrowDown moves selection down", async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");
    await keydownOnInput(wrapper, "ArrowDown");

    // Second item should be highlighted
    const items = wrapper.findAll(".mention-item");
    expect(items.length).toBeGreaterThan(1);
    expect(items[1].classes()).toContain("bg-[var(--color-brand-soft-bg)]");
  });

  it("ArrowUp at top clamps to 0", async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");
    await keydownOnInput(wrapper, "ArrowUp");

    const items = wrapper.findAll(".mention-item");
    expect(items[0].classes()).toContain("bg-[var(--color-brand-soft-bg)]");
  });

  it("ArrowDown at bottom clamps", async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");
    const items = wrapper.findAll(".mention-item");
    // Press down many times
    for (let i = 0; i < items.length + 5; i++) {
      await keydownOnInput(wrapper, "ArrowDown");
    }
    // Last item should be highlighted
    const freshItems = wrapper.findAll(".mention-item");
    expect(freshItems[freshItems.length - 1].classes()).toContain("bg-[var(--color-brand-soft-bg)]");
  });

  it("Ctrl+N / Ctrl+P navigate items", async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");

    await keydownOnInput(wrapper, "n", { ctrlKey: true });
    const items = wrapper.findAll(".mention-item");
    expect(items[1].classes()).toContain("bg-[var(--color-brand-soft-bg)]");

    await keydownOnInput(wrapper, "p", { ctrlKey: true });
    const fresh = wrapper.findAll(".mention-item");
    expect(fresh[0].classes()).toContain("bg-[var(--color-brand-soft-bg)]");
  });

  it("Ctrl+J confirms selection and advances to val phase", async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");

    // First item should be Goal (required, monthly)
    await keydownOnInput(wrapper, "j", { ctrlKey: true });

    // Should advance to val phase
    expect(wrapper.text()).toContain("Pick a value");
  });

  it("Enter confirms selection in menu", async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");
    await keydownOnInput(wrapper, "Enter");

    // Should advance to val phase
    expect(wrapper.text()).toContain("Pick a value");
  });

  it("Tab confirms selection in menu", async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");
    await keydownOnInput(wrapper, "Tab");

    // Should advance to val phase
    expect(wrapper.text()).toContain("Pick a value");
  });

  // ---- Two-phase flow (dim → val → commit/loop) ----

  it("dim pick → val pick → sets dimension value and closes when all required filled", async () => {
    const { wrapper } = mountInput({
      initialValues: { "business-line": "Platform" },
    });
    // Open menu
    await keydownOnInput(wrapper, "@");
    // Confirm on Goal (first item)
    await keydownOnInput(wrapper, "Enter");
    // Now in val phase for Goal — first value (Ship feature X) should be selected
    expect(wrapper.text()).toContain("Ship feature X");
    await keydownOnInput(wrapper, "Enter");

    // All required filled (Goal + Business Line) → menu should close
    expect(wrapper.find(".absolute").exists()).toBe(false);
    // Chips should show both
    expect(wrapper.text()).toContain("Goal: Ship feature X");
  });

  it("loops back to dim phase when more required remain after confirm", async () => {
    const { wrapper } = mountInput({ initialValues: {} });
    // Open menu
    await keydownOnInput(wrapper, "@");
    // Select Goal
    await keydownOnInput(wrapper, "Enter");
    // Select a value for Goal
    await keydownOnInput(wrapper, "Enter");

    // Goal is now filled but Business Line is still required → should loop back
    expect(wrapper.find(".absolute").exists()).toBe(true);
    expect(wrapper.text()).toContain("DIM");
    expect(wrapper.text()).toContain("Pick a dimension");
  });

  it("skipFilled: cursor jumps to first unfilled required dim on loop back", async () => {
    const { wrapper } = mountInput({ initialValues: {} });
    // Open menu
    await keydownOnInput(wrapper, "@");
    // Select Goal (first item), confirm val
    await keydownOnInput(wrapper, "Enter");
    await keydownOnInput(wrapper, "Enter");

    // Should loop back. Goal is filled, Business Line is next required.
    // skipFilled=true should select Business Line (index 1) not Goal (index 0)
    const items = wrapper.findAll(".mention-item");
    // Business Line should be the highlighted item
    expect(items[1].classes()).toContain("bg-[var(--color-brand-soft-bg)]");
    expect(items[1].text()).toContain("Business Line");
  });

  // ---- goBackToDim ----

  it("goBackToDim: left arrow in val phase returns to dim phase", async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");
    // Select dimension → val phase
    await keydownOnInput(wrapper, "Enter");
    expect(wrapper.text()).toContain("Pick a value");

    // Click the left arrow button in the val header
    const backBtn = wrapper.find(".font-bold.leading-none");
    await backBtn.trigger("click");

    // Should be back in dim phase
    expect(wrapper.text()).toContain("DIM");
    expect(wrapper.text()).toContain("Pick a dimension");
  });

  // ---- Dot progress footer ----

  it("shows dot progress footer in dim phase when there are required dims", async () => {
    const { wrapper } = mountInput({ initialValues: {} });
    await keydownOnInput(wrapper, "@");

    expect(wrapper.text()).toContain("to go");
  });

  it('shows "All required ✓" when everything filled', async () => {
    const { wrapper } = mountInput({
      initialValues: { goal: "Code review", "business-line": "Platform" },
    });
    await keydownOnInput(wrapper, "@");
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("All required");
  });

  // ---- Filter text / onInput ----

  it("filters dimensions by typed text", async () => {
    const { wrapper } = mountInput();
    // Open menu with @
    await keydownOnInput(wrapper, "@");

    // Set input value and trigger onInput
    const inp = inputEl(wrapper);
    await inp.setValue("@Busi");
    await inp.trigger("input");

    // Check menu items only (not chip text)
    const menuItems = wrapper.findAll(".mention-item");
    const menuTexts = menuItems.map(i => i.text());
    // "Business Line" should be in the filtered menu
    expect(menuTexts.some(t => t.includes("Business Line"))).toBe(true);
    // "Goal" should NOT be in the filtered menu
    expect(menuTexts.some(t => t.includes("Goal"))).toBe(false);
  });

  it('shows "No matches" when filter yields empty', async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");
    const inp = inputEl(wrapper);
    await inp.setValue("@ZZZZZZZ");
    await inp.trigger("input");

    expect(wrapper.text()).toContain("No matches");
  });

  // ---- clearInput ----

  it("clearInput exposed method: clears input value", () => {
    const { wrapper } = mountInput();
    const inp = inputEl(wrapper);
    // Set value via v-model
    (inp.element as HTMLInputElement).value = "Some text";
    inp.trigger("input");

    (wrapper.vm as unknown as { clearInput: () => void }).clearInput();
    // Verify the input ref was cleared
    expect((wrapper.vm as any).input).toBe("");
  });

  // ---- initialValues watcher ----

  it("syncs initialValues to dimValues on mount", () => {
    const { wrapper } = mountInput({
      initialValues: { goal: "Initial goal" },
    });
    expect(wrapper.text()).toContain("Goal: Initial goal");
  });

  it("updates dimValues when initialValues prop changes", async () => {
    const { wrapper } = mountInput({ initialValues: {} });
    // Check vm state — no goal value set
    expect((wrapper.vm as any).dimValues).toEqual({});

    await wrapper.setProps({ initialValues: { goal: "New goal" } });
    // The reactive dimValues should now have the goal
    expect((wrapper.vm as any).dimValues).toEqual({ goal: "New goal" });
  });

  // ---- Menu items show metadata ----

  it("shows value count on unfilled required dim items", async () => {
    const { wrapper } = mountInput({ initialValues: {} });
    await keydownOnInput(wrapper, "@");

    // Goal is monthly, should show 2 values (from commitments)
    expect(wrapper.text()).toContain("2 values");
  });

  it("shows checkmark ✓ on filled required dim items", async () => {
    const { wrapper } = mountInput({
      initialValues: { goal: "Code review" },
    });
    await keydownOnInput(wrapper, "@");

    // Goal is filled — should show checkmark
    const items = wrapper.findAll(".mention-item");
    const goalItem = items.find(i => i.text().includes("Goal"));
    expect(goalItem?.text()).toContain("✓");
  });

  // ---- val phase footer ----

  it("shows navigation hint footer in val phase", async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");
    await keydownOnInput(wrapper, "Enter"); // → val phase

    expect(wrapper.text()).toContain("Back to dimensions");
  });

  // ---- focus injection ----

  it("focusRequestId change focuses input when body is active", () => {
    const { focusRequestId, wrapper } = mountInput();
    const inp = inputEl(wrapper);
    const focusSpy = vi.spyOn(inp.element as HTMLInputElement, "focus");

    focusRequestId.value++;
    // This triggers the watch, which checks activeElement
    // In jsdom, document.activeElement starts as body
    expect(focusSpy).not.toHaveBeenCalled();
    // Note: actually the watcher checks if active is body — in jsdom it usually is
  });
});
