// src/__tests__/components/CommitmentsPanel.test.ts
import { describe, it, expect, vi } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import CommitmentsPanel from "../../components/CommitmentsPanel.vue";
import { makeCommitmentProgress, makeCommitment } from "../mocks/fixtures";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("vue-draggable-plus", () => ({
  VueDraggable: {
    name: "VueDraggable",
    props: ["modelValue", "handle", "animation", "group", "tag"],
    emits: ["update:modelValue"],
    render() { return (this as any).$slots.default?.(); },
  },
}));

function mountPanel(overrides = {}) {
  return mount(CommitmentsPanel, {
    props: {
      progress: [makeCommitmentProgress({ role: "Developer", goal_spent_minutes: 800, general_spent_minutes: 430, allocation_minutes: 2400 })],
      commitments: [makeCommitment()],
      rootPath: "/x", selectedYear: 2026, selectedMonth: 6,
      ...overrides,
    },
    global: { stubs: { teleport: true } },
  });
}

describe("CommitmentsPanel", () => {
  it("renders role name and mono spent/allocation", () => {
    const w = mountPanel();
    expect(w.text()).toContain("Developer");
    expect(w.text()).toContain("20.5h");
    expect(w.text()).toContain("40");
  });
  it("progress segments use gradient backgrounds (no status colors)", () => {
    const w = mountPanel();
    const goal = w.find("[data-test='progress-goal']");
    const general = w.find("[data-test='progress-general']");
    expect(goal.exists()).toBe(true);
    expect(general.exists()).toBe(true);
    expect(goal.attributes("class") || "").not.toMatch(/bg-(orange|yellow|green|red)-/);
    expect(general.attributes("class") || "").not.toMatch(/bg-(orange|yellow|green|red)-/);
  });
  it("opens the modal on Edit click", async () => {
    const w = mountPanel();
    expect(w.find("[role='dialog']").exists()).toBe(false);
    await w.find("[data-test='edit-btn']").trigger("click");
    expect(w.find("[role='dialog']").exists()).toBe(true);
  });
  it("shows 'Set up commitments' and opens modal when there are no commitments", async () => {
    const w = mountPanel({ progress: [], commitments: [] });
    const setup = w.find("[data-test='setup-btn']");
    expect(setup.exists()).toBe(true);
    await setup.trigger("click");
    expect(w.find("[role='dialog']").exists()).toBe(true);
  });
  it("toggles the goal list for a role", async () => {
    const w = mountPanel();
    const goalRows = () => w.findAll("[data-test='goal-row']");
    const initial = goalRows().length;
    await w.find("[data-test='role-toggle']").trigger("click");
    expect(goalRows().length).not.toBe(initial);
  });
  it("re-emits 'saved' to the parent when the modal saves", async () => {
    const w = mountPanel(); // default commitments are valid → save passes validation
    await w.find("[data-test='edit-btn']").trigger("click");
    await w.find("[data-test='save']").trigger("click");
    await flushPromises();
    expect(w.emitted("saved")).toBeTruthy();
    expect(w.find("[role='dialog']").exists()).toBe(false); // modal closed after save
  });
  it("closing the modal hides the dialog", async () => {
    const w = mountPanel();
    await w.find("[data-test='edit-btn']").trigger("click");
    expect(w.find("[role='dialog']").exists()).toBe(true);
    await w.find("[data-test='cancel']").trigger("click"); // no changes → immediate close
    expect(w.find("[role='dialog']").exists()).toBe(false);
  });
});
