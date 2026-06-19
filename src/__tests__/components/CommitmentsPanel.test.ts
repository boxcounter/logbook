// src/__tests__/components/CommitmentsPanel.test.ts
import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import CommitmentsPanel from "../../components/CommitmentsPanel.vue";
import { makeCommitmentProgress, makeCommitment } from "../mocks/fixtures";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("vuedraggable", () => ({
  default: { name: "draggable", props: ["modelValue","itemKey","handle","group","tag","animation"], emits: ["update:modelValue"],
    render() { const items=(this as any).modelValue||[]; const s=(this as any).$slots; return items.map((element:any,index:number)=> s.item? s.item({element,index}):null); } },
}));

function mountPanel(overrides = {}) {
  return mount(CommitmentsPanel, {
    props: {
      progress: [makeCommitmentProgress({ role: "Developer", spent_minutes: 1230, allocation_minutes: 2400 })],
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
    expect(w.text()).toContain("20h 30m");
    expect(w.text()).toContain("40");
  });
  it("progress fill uses the brand gradient (single style, no status colors)", () => {
    const w = mountPanel();
    const fill = w.find("[data-test='progress-fill']");
    expect(fill.attributes("class") || "").not.toMatch(/bg-(orange|yellow|green|red)-/);
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
});
