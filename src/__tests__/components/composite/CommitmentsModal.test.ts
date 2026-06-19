import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { invoke } from "@tauri-apps/api/core";
import CommitmentsModal from "../../../components/composite/CommitmentsModal.vue";
import { makeCommitment, makeCommitmentProgress } from "../../mocks/fixtures";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// vuedraggable stub: render the #item slot for each model element (no real DnD in jsdom)
vi.mock("vuedraggable", () => ({
  default: {
    name: "draggable",
    props: ["modelValue", "itemKey", "handle", "group", "tag", "animation"],
    emits: ["update:modelValue"],
    render() {
      const items = (this as any).modelValue || [];
      const slots = (this as any).$slots;
      return items.map((element: any, index: number) =>
        slots.item ? slots.item({ element, index }) : null
      );
    },
  },
}));

const baseProps = () => ({
  open: true,
  commitments: [
    makeCommitment({ role: "Developer", allocation: 40, goals: ["Ship onboarding v2", "Review auth PR"] }),
  ],
  progress: [
    makeCommitmentProgress({
      role: "Developer", allocation_minutes: 2400, spent_minutes: 870,
      goals: [
        { name: "Ship onboarding v2", spent_minutes: 865 },
        { name: "Review auth PR", spent_minutes: 5 },
      ],
    }),
  ],
  rootPath: "/tmp", selectedYear: 2026, selectedMonth: 6,
});

function mountModal(overrides = {}) {
  return mount(CommitmentsModal, {
    props: { ...baseProps(), ...overrides },
    global: { stubs: { teleport: true } },
  });
}

beforeEach(() => { (invoke as any).mockReset?.(); (invoke as any).mockResolvedValue?.([]); });

describe("CommitmentsModal — base", () => {
  it("renders role and goal values from props", () => {
    const w = mountModal();
    expect((w.find("[data-test='role-name']").element as HTMLInputElement).value).toBe("Developer");
    const goals = w.findAll("[data-test='goal-name']").map(g => (g.element as HTMLInputElement).value);
    expect(goals).toContain("Ship onboarding v2");
    expect(goals).toContain("Review auth PR");
  });

  it("renders a drag handle per role and per goal", () => {
    const w = mountModal();
    expect(w.findAll("[data-test='drag-grip-role']").length).toBe(1);
    expect(w.findAll("[data-test='drag-grip-goal']").length).toBe(2);
  });

  it("adds a goal row on + Add Goal", async () => {
    const w = mountModal();
    const before = w.findAll("[data-test='goal-name']").length;
    await w.find("[data-test='add-goal']").trigger("click");
    expect(w.findAll("[data-test='goal-name']").length).toBe(before + 1);
  });

  it("adds a role on + Add Role", async () => {
    const w = mountModal();
    await w.find("[data-test='add-role']").trigger("click");
    expect(w.findAll("[data-test='role-name']").length).toBe(2);
  });

  it("Save calls set_commitments with trimmed commitments and emits saved+close", async () => {
    const w = mountModal();
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).toHaveBeenCalledWith("set_commitments", expect.objectContaining({
      rootPath: "/tmp", year: 2026, month: 6,
      commitments: [{ role: "Developer", allocation: 40, goals: ["Ship onboarding v2", "Review auth PR"] }],
    }));
    expect(w.emitted("saved")).toBeTruthy();
    expect(w.emitted("close")).toBeTruthy();
  });

  it("Cancel emits close without invoking backend", async () => {
    const w = mountModal();
    await w.find("[data-test='cancel']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.emitted("close")).toBeTruthy();
  });
});
