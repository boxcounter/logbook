import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { ref } from "vue";
import EntryInput from "../../components/EntryInput.vue";
import MentionMenu from "../../components/composite/MentionMenu.vue";
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

// Helper: find the MentionMenu component in the wrapper
function findMentionMenu(wrapper: ReturnType<typeof mountInput>["wrapper"]) {
  return wrapper.findComponent(MentionMenu);
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
    const btn = wrapper.find("button");
    expect(btn.text()).toBe("Log");
  });

  it("Log button disabled when input empty", () => {
    const { wrapper } = mountInput();
    const btn = wrapper.find("button");
    expect((btn.element as HTMLButtonElement).disabled).toBe(true);
  });

  it("Log button enabled when input has text", async () => {
    const { wrapper } = mountInput();
    await typeIn(wrapper, "Meeting 30m");
    const btn = wrapper.find("button");
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

    const btn = wrapper.find("button");
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

    const btn = wrapper.find("button");
    await btn.trigger("click");

    expect(wrapper.text()).toContain("Missing required");
    expect(wrapper.emitted("submit")).toBeFalsy();
  });

  it("no duration in input: shows error", async () => {
    const { wrapper } = mountInput({
      initialValues: { goal: "Code review", "business-line": "Platform" },
    });
    await typeIn(wrapper, "Just text");

    const btn = wrapper.find("button");
    await btn.trigger("click");

    expect(wrapper.text()).toContain("Could not parse duration");
    expect(wrapper.emitted("submit")).toBeFalsy();
  });

  it("empty input: no submit", async () => {
    const { wrapper } = mountInput();
    const btn = wrapper.find("button");
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
    // Find the Goal chip by its text content
    const allSpans = wrapper.findAll("span");
    const chip = allSpans.find(s => s.text().includes("Goal: Ship feature X"));
    expect(chip).toBeTruthy();
    await chip!.trigger("click");

    // The hint text should reappear (goal cleared, business-line still required)
    expect(wrapper.text()).toContain("+ Business Line");
  });

  it("shows missing required chips with red dashed border", () => {
    const { wrapper } = mountInput({ initialValues: {} });
    expect(wrapper.text()).toContain("+ Goal");
    expect(wrapper.text()).toContain("+ Business Line");
  });

  it("clicking missing required chip opens the dim menu", async () => {
    const { wrapper } = mountInput({ initialValues: {} });
    // Find the red missing chip and click
    const missingChips = wrapper.findAll(".border-dashed");
    expect(missingChips.length).toBeGreaterThan(0);
    await missingChips[0].trigger("click");

    // Menu should be visible (dim phase)
    expect(wrapper.text()).toContain("Pick a dimension");
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
    expect(wrapper.text()).toContain("Pick a dimension");

    // Press Escape
    await keydownOnInput(wrapper, "Escape");
    expect(wrapper.text()).not.toContain("Pick a dimension");
  });

  // ---- MentionMenu integration: select event ----

  it("selecting a dim+value sets dimension and closes when all required filled", async () => {
    const { wrapper } = mountInput({
      initialValues: { "business-line": "Platform" },
    });
    // Open menu
    await keydownOnInput(wrapper, "@");
    expect(wrapper.text()).toContain("Pick a dimension");

    // Simulate MentionMenu select event
    const mentionMenu = findMentionMenu(wrapper);
    expect(mentionMenu.exists()).toBe(true);
    await mentionMenu.vm.$emit("select", "goal", "Ship feature X");
    await wrapper.vm.$nextTick();

    // All required filled → menu should close
    expect(wrapper.text()).not.toContain("Pick a dimension");
    // Chips should show the new value
    expect(wrapper.text()).toContain("Goal: Ship feature X");
  });

  it("loops back to dim phase when more required remain after select", async () => {
    const { wrapper } = mountInput({ initialValues: {} });
    await keydownOnInput(wrapper, "@");

    const mentionMenu = findMentionMenu(wrapper);
    expect(mentionMenu.exists()).toBe(true);
    await mentionMenu.vm.$emit("select", "goal", "Ship feature X");
    await wrapper.vm.$nextTick();

    // Menu should stay open (not all required filled — Business Line still needed)
    expect(wrapper.text()).toContain("Pick a dimension");
    // Goal chip should appear
    expect(wrapper.text()).toContain("Goal: Ship feature X");
  });

  it("MentionMenu close event removes mention and closes menu", async () => {
    const { wrapper } = mountInput();
    await keydownOnInput(wrapper, "@");
    expect(wrapper.text()).toContain("Pick a dimension");

    const mentionMenu = findMentionMenu(wrapper);
    expect(mentionMenu.exists()).toBe(true);
    await mentionMenu.vm.$emit("close");
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).not.toContain("Pick a dimension");
  });

  // ---- Dot progress footer ----

  it("shows dot progress footer in dim phase when there are required dims", async () => {
    const { wrapper } = mountInput({ initialValues: {} });
    await keydownOnInput(wrapper, "@");

    expect(wrapper.text()).toContain("to go");
  });

  it('shows "0 to go" when all required filled', async () => {
    const { wrapper } = mountInput({
      initialValues: { goal: "Code review", "business-line": "Platform" },
    });
    await keydownOnInput(wrapper, "@");
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("0 to go");
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

  // ---- focus injection ----

  it("focusRequestId change does not error when body is active", () => {
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
