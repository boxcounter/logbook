import { describe, it, expect, vi, beforeEach } from "vitest";
import { nextTick } from "vue";
import { mount } from "@vue/test-utils";
import { invoke } from "@tauri-apps/api/core";
import CommitmentsModal from "../../../components/composite/CommitmentsModal.vue";
import { makeCommitment, makeCommitmentProgress } from "../../mocks/fixtures";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// vue-draggable-plus stub: render the default slot (the v-for output) — no real DnD in jsdom
vi.mock("vue-draggable-plus", () => ({
  VueDraggable: {
    name: "VueDraggable",
    props: ["modelValue", "handle", "animation", "group", "tag"],
    emits: ["update:modelValue"],
    render() { return (this as any).$slots.default?.(); },
  },
}));

const baseProps = () => ({
  open: true,
  commitments: [
    makeCommitment({ role: "Developer", allocation: 40, goals: ["Ship onboarding v2", "Review auth PR"] }),
  ],
  progress: [
    makeCommitmentProgress({
      role: "Developer", allocation_minutes: 2400, goal_spent_minutes: 870, general_spent_minutes: 0,
      goals: [
        { name: "Ship onboarding v2", spent_minutes: 865 },
        { name: "Review auth PR", spent_minutes: 5},
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
    // In two-column layout, only the selected role's `role-name` renders.
    // Count left-panel rows (role-row + role-row-selected) instead.
    expect(w.findAll("[data-test='role-row'],[data-test='role-row-selected']").length).toBe(2);
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

  it("editing the draft does not mutate the commitments prop (working-copy isolation)", async () => {
    const goals = ["Ship onboarding v2", "Review auth PR"];
    const commitments = [makeCommitment({ role: "Developer", allocation: 40, goals })];
    const w = mountModal({ commitments });

    // Mutate the draft via the editor: role name, a goal name, and add a goal.
    await w.find("[data-test='role-name']").setValue("Architect");
    await w.findAll("[data-test='goal-name']")[0].setValue("Ship onboarding v3");
    await w.find("[data-test='add-goal']").trigger("click");

    // The original prop object (and its nested goals array) must be untouched.
    expect(commitments[0].role).toBe("Developer");
    expect(commitments[0].goals).toBe(goals); // same array reference, not replaced
    expect(goals).toEqual(["Ship onboarding v2", "Review auth PR"]); // length + values intact
  });
});

describe("CommitmentsModal — allocation stepper", () => {
  it("increments by 5 on +", async () => {
    const w = mountModal();
    await w.find("[data-test='alloc-inc']").trigger("click");
    expect((w.find("[data-test='alloc']").element as HTMLInputElement).value).toBe("45");
  });
  it("decrements by 5 on -", async () => {
    const w = mountModal();
    await w.find("[data-test='alloc-dec']").trigger("click");
    expect((w.find("[data-test='alloc']").element as HTMLInputElement).value).toBe("35");
  });
  it("disables - at the 5h floor and never goes below 5", async () => {
    const w = mountModal({
      commitments: [makeCommitment({ role: "Developer", allocation: 5, goals: [] })],
      progress: [makeCommitmentProgress({ role: "Developer", allocation_minutes: 300, goal_spent_minutes: 0, general_spent_minutes: 0, goals: [] })],
    });
    const dec = w.find("[data-test='alloc-dec']");
    expect((dec.element as HTMLButtonElement).disabled).toBe(true);
    await dec.trigger("click");
    expect((w.find("[data-test='alloc']").element as HTMLInputElement).value).toBe("5");
  });
  it("Arrow Up/Down adjusts by 5", async () => {
    const w = mountModal();
    // Re-query the input after each keydown: under the `teleport: true` test stub,
    // the committedHours-driven modal re-render remounts the draggable's keyed child,
    // so a captured wrapper would point at a detached node. (Real Teleport patches
    // the node in place — verified separately — so this is a test-harness concern.)
    await w.find("[data-test='alloc']").trigger("keydown", { key: "ArrowUp" });
    expect((w.find("[data-test='alloc']").element as HTMLInputElement).value).toBe("45");
    await w.find("[data-test='alloc']").trigger("keydown", { key: "ArrowDown" });
    expect((w.find("[data-test='alloc']").element as HTMLInputElement).value).toBe("40");
  });
  it("floors a decimal typed value and re-syncs the input", async () => {
    const w = mountModal();
    const inp = w.find("[data-test='alloc']");
    await inp.setValue("3.7");
    expect((inp.element as HTMLInputElement).value).toBe("3");
  });
  it("clamps a cleared field to 1 and re-syncs the input (no desync)", async () => {
    const w = mountModal({
      commitments: [makeCommitment({ role: "Developer", allocation: 1, goals: [] })],
      progress: [makeCommitmentProgress({ role: "Developer", allocation_minutes: 60, goal_spent_minutes: 0, general_spent_minutes: 0, goals: [] })],
    });
    const inp = w.find("[data-test='alloc']");
    await inp.setValue("");
    expect((inp.element as HTMLInputElement).value).toBe("1");
  });
});

describe("CommitmentsModal — summary, progress & over-commit", () => {
  it("header shows live committed total and logged total", async () => {
    const w = mountModal(); // committed 40h, logged 870m = 14h 30m
    expect(w.find("[data-test='committed']").text()).toContain("40h");
    expect(w.find("[data-test='logged']").text()).toContain("14.5h");
    await w.find("[data-test='alloc-inc']").trigger("click"); // 40→45
    expect(w.find("[data-test='committed']").text()).toContain("45h");
  });
  it("shows role logged and per-goal logged", () => {
    const w = mountModal();
    expect(w.find("[data-test='role-spent']").text()).toContain("14.5h");
    const logged = w.findAll("[data-test='goal-logged']").map(n => n.text());
    expect(logged.some(t => t.includes("14.4h"))).toBe(true);
    expect(logged.some(t => t.includes("0.1h"))).toBe(true);
  });
  it("bar fills proportionally to spent/allocation", () => {
    const w = mountModal(); // 870/2400 ≈ 36%
    expect((w.find("[data-test='bar-fill']").element as HTMLElement).style.width).toBe("36%");
  });
  it("keeps per-goal logged matched by original name after a rename", async () => {
    const w = mountModal();
    // Rename the first goal; its logged time (matched by origName) must persist.
    await w.findAll("[data-test='goal-name']")[0].setValue("Renamed goal");
    const logged = w.findAll("[data-test='goal-logged']").map(n => n.text());
    expect(logged.some(t => t.includes("14.4h"))).toBe(true);
  });
  it("turns amber + 'over by' when allocation drops below logged", async () => {
    const w = mountModal();
    const dec = w.find("[data-test='alloc-dec']");
    for (let i = 0; i < 6; i++) await dec.trigger("click"); // 40→10
    expect((w.find("[data-test='bar-fill']").element as HTMLElement).style.width).toBe("100%");
    expect(w.find("[data-test='role-spent']").text()).toContain("over by");
    expect(w.find("[data-test='bar-fill']").classes().join(" ")).toContain("bg-[var(--color-warning)]");
  });
});

describe("CommitmentsModal — delete constraints", () => {
  it("disables goal remove when the goal has logged time", () => {
    const w = mountModal();
    expect(w.findAll("[data-test='goal-remove']").every(b => (b.element as HTMLButtonElement).disabled)).toBe(true);
  });
  it("enables remove for a freshly added (0-logged) goal", async () => {
    const w = mountModal();
    await w.find("[data-test='add-goal']").trigger("click");
    const removes = w.findAll("[data-test='goal-remove']");
    expect((removes[removes.length - 1].element as HTMLButtonElement).disabled).toBe(false);
  });
  it("removes a 0-logged goal on click", async () => {
    const w = mountModal();
    await w.find("[data-test='add-goal']").trigger("click");
    const before = w.findAll("[data-test='goal-name']").length;
    const removes = w.findAll("[data-test='goal-remove']");
    await removes[removes.length - 1].trigger("click");
    expect(w.findAll("[data-test='goal-name']").length).toBe(before - 1);
  });
  it("disables role Delete when any goal has logged time", () => {
    const w = mountModal();
    expect((w.find("[data-test='role-delete']").element as HTMLButtonElement).disabled).toBe(true);
  });
  it("role Delete on a 0-logged role shows inline confirm then removes", async () => {
    const w = mountModal({
      commitments: [
        makeCommitment({ role: "Developer", allocation: 40, goals: ["Ship onboarding v2"] }),
        makeCommitment({ role: "Advisor", allocation: 5, goals: ["Office hours"] }),
      ],
      progress: [
        makeCommitmentProgress({ role: "Developer", goal_spent_minutes: 870, general_spent_minutes: 0, allocation_minutes: 2400, goals: [{ name: "Ship onboarding v2", spent_minutes: 870}] }),
        makeCommitmentProgress({ role: "Advisor", goal_spent_minutes: 0, general_spent_minutes: 0, allocation_minutes: 300, goals: [{ name: "Office hours", spent_minutes: 0}] }),
      ],
    });
    // Two-column: only the selected role's delete button renders.
    // Developer is selected by default; select Advisor in the left panel first.
    await w.findAll("[data-test='role-row']")[0].trigger("click");
    const advisorDel = w.find("[data-test='role-delete']");
    expect((advisorDel.element as HTMLButtonElement).disabled).toBe(false);
    await advisorDel.trigger("click");
    await w.find("[data-test='role-delete-confirm']").trigger("click");
    // After removing Advisor, count roles in the left panel.
    expect(w.findAll("[data-test='role-row'],[data-test='role-row-selected']").length).toBe(1);
  });

  it("Cancel in the role-delete confirm dismisses without removing", async () => {
    const w = mountModal({
      commitments: [
        makeCommitment({ role: "Developer", allocation: 40, goals: ["Ship onboarding v2"] }),
        makeCommitment({ role: "Advisor", allocation: 5, goals: ["Office hours"] }),
      ],
      progress: [
        makeCommitmentProgress({ role: "Developer", goal_spent_minutes: 870, general_spent_minutes: 0, allocation_minutes: 2400, goals: [{ name: "Ship onboarding v2", spent_minutes: 870}] }),
        makeCommitmentProgress({ role: "Advisor", goal_spent_minutes: 0, general_spent_minutes: 0, allocation_minutes: 300, goals: [{ name: "Office hours", spent_minutes: 0}] }),
      ],
    });
    // Select Advisor in the left panel, then trigger its delete
    await w.findAll("[data-test='role-row']")[0].trigger("click"); // Advisor
    await w.find("[data-test='role-delete']").trigger("click");
    await w.find("[data-test='role-delete-cancel']").trigger("click");
    expect(w.findAll("[data-test='role-row'],[data-test='role-row-selected']").length).toBe(2); // nothing removed
    expect(w.find("[data-test='role-delete-confirm']").exists()).toBe(false); // confirm dismissed
  });

  it("clicking a logged goal's remove does not delete it", async () => {
    const w = mountModal(); // baseProps: Developer with two logged goals
    const before = w.findAll("[data-test='goal-name']").length;
    await w.findAll("[data-test='goal-remove']")[0].trigger("click");
    expect(w.findAll("[data-test='goal-name']").length).toBe(before); // guard blocks removal
  });
});

describe("CommitmentsModal — validation", () => {
  it("blocks save + message on empty role name", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("");
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.text()).toContain("Role name is required");
  });
  it("blocks save on duplicate role names", async () => {
    const w = mountModal({
      commitments: [makeCommitment({ role: "Developer", allocation: 40, goals: [] }), makeCommitment({ role: "Developer", allocation: 20, goals: [] })],
      progress: [],
    });
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.text()).toContain("Duplicate role name");
  });
  it("blocks save on duplicate goal names across roles", async () => {
    const w = mountModal({
      commitments: [makeCommitment({ role: "Developer", allocation: 40, goals: ["Shared"] }), makeCommitment({ role: "Advisor", allocation: 5, goals: ["Shared"] })],
      progress: [],
    });
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.text()).toContain("Duplicate goal name");
  });
  it("blocks emptying a goal that has logged time", async () => {
    const w = mountModal();
    await w.findAll("[data-test='goal-name']")[0].setValue("");
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.text()).toContain("can't be empty");
  });
  it("silently drops a blank 0-logged goal row on save", async () => {
    (invoke as any).mockResolvedValue([]);
    const w = mountModal({
      commitments: [makeCommitment({ role: "Developer", allocation: 40, goals: ["Ship onboarding v2"] })],
      progress: [makeCommitmentProgress({ role: "Developer", goal_spent_minutes: 0, general_spent_minutes: 0, allocation_minutes: 2400, goals: [{ name: "Ship onboarding v2", spent_minutes: 0}] })],
    });
    await w.find("[data-test='add-goal']").trigger("click");
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).toHaveBeenCalledWith("set_commitments", expect.objectContaining({
      commitments: [{ role: "Developer", allocation: 40, goals: ["Ship onboarding v2"] }],
    }));
  });

  it("red-borders the offending role field after a blocked save", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("");
    await w.find("[data-test='save']").trigger("click");
    expect(w.find("[data-test='role-name']").classes()).toContain("border-[var(--color-danger)]");
  });
  it("red-borders a duplicate goal field after a blocked save", async () => {
    const w = mountModal({
      commitments: [
        makeCommitment({ role: "Developer", allocation: 40, goals: ["Shared"] }),
        makeCommitment({ role: "Advisor", allocation: 5, goals: ["Shared"] }),
      ],
      progress: [],
    });
    await w.find("[data-test='save']").trigger("click");
    // Two-column: only the selected role's (Developer's) goals render.
    // The "Shared" goal appears once, and must be red-bordered.
    const dupGoalInputs = w.findAll("[data-test='goal-name']")
      .filter(i => (i.element as HTMLInputElement).value === "Shared");
    expect(dupGoalInputs.length).toBe(1);
    expect(dupGoalInputs[0].classes()).toContain("border-[var(--color-danger)]");
  });

  it("clears the error and saves after the user fixes an invalid field", async () => {
    (invoke as any).mockResolvedValue([]);
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("");
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.text()).toContain("Role name is required");
    await w.find("[data-test='role-name']").setValue("Engineer");
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).toHaveBeenCalledWith("set_commitments", expect.objectContaining({
      commitments: expect.arrayContaining([expect.objectContaining({ role: "Engineer" })]),
    }));
    expect(w.text()).not.toContain("Role name is required");
  });

  it("blocks save with 'At least one role is required' when the draft is empty", async () => {
    const w = mountModal({
      commitments: [makeCommitment({ role: "Solo", allocation: 40, goals: [] })],
      progress: [makeCommitmentProgress({ role: "Solo", allocation_minutes: 2400, goal_spent_minutes: 0, general_spent_minutes: 0, goals: [] })],
    });
    await w.find("[data-test='role-delete']").trigger("click");      // 0-logged role → inline confirm
    await w.find("[data-test='role-delete-confirm']").trigger("click"); // remove it → draft empty
    await w.find("[data-test='save']").trigger("click");
    expect(invoke).not.toHaveBeenCalled();
    expect(w.text()).toContain("At least one role is required");
  });
});

describe("CommitmentsModal — keyboard", () => {
  it("Enter in a goal input adds a new goal row below", async () => {
    const w = mountModal();
    const before = w.findAll("[data-test='goal-name']").length;
    await w.findAll("[data-test='goal-name']")[0].trigger("keydown", { key: "Enter" });
    expect(w.findAll("[data-test='goal-name']").length).toBe(before + 1);
  });
  it("Enter on a trailing blank goal does NOT add another", async () => {
    const w = mountModal();
    await w.find("[data-test='add-goal']").trigger("click");
    const count = w.findAll("[data-test='goal-name']").length;
    await w.findAll("[data-test='goal-name']")[count - 1].trigger("keydown", { key: "Enter" });
    expect(w.findAll("[data-test='goal-name']").length).toBe(count);
  });
  it("Cmd/Ctrl+Enter saves", async () => {
    (invoke as any).mockResolvedValue([]);
    const w = mountModal();
    await w.find("[data-test='overlay']").trigger("keydown", { key: "Enter", metaKey: true });
    expect(invoke).toHaveBeenCalledWith("set_commitments", expect.anything());
  });
  it("Cmd+Enter in a goal input does NOT insert a goal row (only saves)", async () => {
    (invoke as any).mockResolvedValue([]);
    const w = mountModal();
    const before = w.findAll("[data-test='goal-name']").length;
    await w.findAll("[data-test='goal-name']")[0].trigger("keydown", { key: "Enter", metaKey: true });
    expect(w.findAll("[data-test='goal-name']").length).toBe(before); // .enter.exact: no insert
    expect(invoke).toHaveBeenCalledWith("set_commitments", expect.anything()); // bubbled to overlay → save
  });
});

describe("CommitmentsModal — close & discard", () => {
  it("moves focus into the dialog on open so esc reaches it regardless of trigger focus", async () => {
    const w = mount(CommitmentsModal, {
      props: { ...baseProps(), open: true },
      attachTo: document.body,
      global: { stubs: { teleport: true } },
    });
    await nextTick();
    await nextTick();
    const overlay = w.find("[data-test='overlay']").element;
    expect(document.activeElement).toBe(overlay);
    w.unmount();
  });
  it("Esc closes immediately when there are no changes", async () => {
    const w = mountModal();
    await w.find("[data-test='overlay']").trigger("keydown", { key: "Escape" });
    expect(w.emitted("close")).toBeTruthy();
  });
  it("Esc with changes shows discard confirm instead of closing", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("Changed");
    await w.find("[data-test='overlay']").trigger("keydown", { key: "Escape" });
    expect(w.emitted("close")).toBeFalsy();
    expect(w.find("[data-test='discard-confirm']").exists()).toBe(true);
  });
  it("Discard in the confirm emits close", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("Changed");
    await w.find("[data-test='cancel']").trigger("click");
    await w.find("[data-test='discard-yes']").trigger("click");
    expect(w.emitted("close")).toBeTruthy();
  });
  it("clicking the backdrop behaves like cancel (no changes → close)", async () => {
    const w = mountModal();
    await w.find("[data-test='overlay']").trigger("click");
    expect(w.emitted("close")).toBeTruthy();
  });
  it("Keep editing dismisses the discard confirm without closing", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("Changed");
    await w.find("[data-test='cancel']").trigger("click"); // dirty → discard confirm
    expect(w.find("[data-test='discard-confirm']").exists()).toBe(true);
    await w.find("[data-test='discard-confirm'] button").trigger("click"); // first button = "Keep editing"
    expect(w.find("[data-test='discard-confirm']").exists()).toBe(false); // dismissed
    expect(w.emitted("close")).toBeFalsy(); // not closed
  });
  it("backdrop click with changes shows the discard confirm (does not close)", async () => {
    const w = mountModal();
    await w.find("[data-test='role-name']").setValue("Changed");
    await w.find("[data-test='overlay']").trigger("click");
    expect(w.emitted("close")).toBeFalsy();
    expect(w.find("[data-test='discard-confirm']").exists()).toBe(true);
  });
});
