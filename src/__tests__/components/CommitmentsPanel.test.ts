import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { setupTauriMocks } from "../mocks/tauri";
import CommitmentsPanel from "../../components/CommitmentsPanel.vue";
import { makeCommitment, makeCommitmentProgress } from "../mocks/fixtures";
import type { Commitment, CommitmentProgress } from "../../types";

function mountPanel(progress: CommitmentProgress[], selectedYear = 2026, selectedMonth = 6) {
  return mount(CommitmentsPanel, {
    props: { progress, selectedYear, selectedMonth },
  });
}

// Helper to create a progress entry with specific spent values
function goalProgress(name: string, spentMinutes: number) {
  return { name, spent_minutes: spentMinutes };
}

function makeCommitmentObj(overrides?: Partial<Commitment>): Commitment {
  return makeCommitment(overrides);
}

function mountPanelWithEdit(
  commitments: Commitment[],
  progress = commitments.map(c => ({
    role: c.role,
    allocation_minutes: c.allocation * 60,
    spent_minutes: 0,
    goals: c.goals.map(g => ({ name: g, spent_minutes: 0 })),
  })),
  rootPath = "/test/root",
) {
  return mount(CommitmentsPanel, {
    props: {
      progress,
      commitments,
      rootPath,
      selectedYear: 2026,
      selectedMonth: 6,
    },
  });
}

// ============================================================

describe("CommitmentsPanel", () => {
  it("renders nothing when progress empty", () => {
    const wrapper = mountPanel([]);
    expect(wrapper.find('[data-test="commitments-panel"]').exists()).toBe(false);
  });

  it("renders each commitment role", () => {
    const progress = [
      makeCommitmentProgress({ role: "Developer" }),
      makeCommitmentProgress({ role: "Director" }),
    ];
    const wrapper = mountPanel(progress);
    const text = wrapper.text();
    expect(text).toContain("Developer");
    expect(text).toContain("Director");
  });

  it("shows monthly allocation in hours", () => {
    // 2400 minutes = 40.0h
    const progress = [makeCommitmentProgress({ allocation_minutes: 2400 })];
    const wrapper = mountPanel(progress);
    expect(wrapper.text()).toContain("40.0h");
  });

  it("shows spent / allocation ratio text", () => {
    const progress = [makeCommitmentProgress({ spent_minutes: 150, allocation_minutes: 2400 })];
    const wrapper = mountPanel(progress);
    // formatDuration(150) = "2h 30m"
    expect(wrapper.text()).toContain("2h 30m");
  });

  it("progress bar width reflects percentage", () => {
    // 1200 spent out of 2400 allocated = 50%
    const progress = [makeCommitmentProgress({ spent_minutes: 1200, allocation_minutes: 2400 })];
    const wrapper = mountPanel(progress);
    const bar = wrapper.find("[data-test='progress-fill']");
    expect(bar.attributes("style")).toContain("width: 50%");
  });

  it("clamps progress bar width to 100%", () => {
    // 3000 spent > 2400 allocated → clamped to 100%
    const progress = [makeCommitmentProgress({ spent_minutes: 3000, allocation_minutes: 2400 })];
    const wrapper = mountPanel(progress);
    const bar = wrapper.find("[data-test='progress-fill']");
    expect(bar.attributes("style")).toContain("width: 100%");
  });

  it("red bar when spent > allocation", () => {
    const progress = [makeCommitmentProgress({ spent_minutes: 3000, allocation_minutes: 2400 })];
    const wrapper = mountPanel(progress);
    const bar = wrapper.find("[data-test='progress-fill']");
    expect(bar.classes()).toContain("bg-red-500");
  });

  it("renders goal breakdown with names and times", () => {
    const progress = [
      makeCommitmentProgress({
        goals: [
          goalProgress("Code review", 75),
          goalProgress("Ship feature X", 120),
        ],
      }),
    ];
    const wrapper = mountPanel(progress);
    const text = wrapper.text();
    expect(text).toContain("Code review");
    expect(text).toContain("1h 15m");
    expect(text).toContain("Ship feature X");
    expect(text).toContain("2h");
  });

  it("shows zero goal as '0m' with gray text", () => {
    const progress = [makeCommitmentProgress()];
    const wrapper = mountPanel(progress);
    const text = wrapper.text();
    expect(text).toContain("0m");

    // Find the goal with 0 spent — should have text-gray-300 class
    const goalRow = wrapper.find("[data-test='goal-row']");
    expect(goalRow.exists()).toBe(true);
    expect(goalRow.text()).toContain("0m");
  });

  it("zero allocation shows 0% width and gray bar", () => {
    const progress = [makeCommitmentProgress({ allocation_minutes: 0, spent_minutes: 60 })];
    const wrapper = mountPanel(progress);
    const bar = wrapper.find("[data-test='progress-fill']");
    expect(bar.attributes("style")).toContain("width: 0%");
    expect(bar.classes()).toContain("bg-gray-300");
  });

  it("orange bar when spent significantly behind elapsed time (current month)", () => {
    // Mock date to June 15 (50% elapsed), spent is 0 → way behind → orange
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 5, 15)); // month is 0-indexed: 5 = June

    const progress = [makeCommitmentProgress({
      spent_minutes: 0,
      allocation_minutes: 2400, // 40h
    })];
    const wrapper = mountPanel(progress, 2026, 6);
    const bar = wrapper.find("[data-test='progress-fill']");
    expect(bar.classes()).toContain("bg-orange-500");

    vi.useRealTimers();
  });

  it("green bar when spent is in sync with elapsed time (current month)", () => {
    vi.useFakeTimers();
    // June 15: ~50% elapsed. 1200/2400 = 50% → within [50%*0.6, 50%*1.4] = [30%, 70%] → green
    vi.setSystemTime(new Date(2026, 5, 15));

    const progress = [makeCommitmentProgress({
      spent_minutes: 1200,
      allocation_minutes: 2400,
    })];
    const wrapper = mountPanel(progress, 2026, 6);
    const bar = wrapper.find("[data-test='progress-fill']");
    expect(bar.classes()).toContain("bg-green-500");

    vi.useRealTimers();
  });

  it("yellow bar when spent is ahead of elapsed time (current month)", () => {
    vi.useFakeTimers();
    // June 5: ~17% elapsed. 40% spent → > 17% * 1.4 = 23.3% → yellow
    vi.setSystemTime(new Date(2026, 5, 5));

    const progress = [makeCommitmentProgress({
      spent_minutes: 960, // 40% of 2400
      allocation_minutes: 2400,
    })];
    const wrapper = mountPanel(progress, 2026, 6);
    const bar = wrapper.find("[data-test='progress-fill']");
    expect(bar.classes()).toContain("bg-yellow-500");

    vi.useRealTimers();
  });

  it("historical month uses 100% elapsed (color based on total completion)", () => {
    // May 2026 — historical month. elapsed = 100%.
    // 50% spent < 60% elapsed → orange
    const progress = [makeCommitmentProgress({
      spent_minutes: 1200, // 50% of 2400
      allocation_minutes: 2400,
    })];
    const wrapper = mountPanel(progress, 2026, 5);
    const bar = wrapper.find("[data-test='progress-fill']");
    expect(bar.classes()).toContain("bg-orange-500");

    // 95% spent → within [60%, 140%] → green
    const progress2 = [makeCommitmentProgress({
      spent_minutes: 2280, // 95%
      allocation_minutes: 2400,
    })];
    const wrapper2 = mountPanel(progress2, 2026, 5);
    const bar2 = wrapper2.find("[data-test='progress-fill']");
    expect(bar2.classes()).toContain("bg-green-500");
  });
});

describe("CommitmentsPanel edit mode", () => {
  it("shows edit button when commitments provided", () => {
    const commitments = [makeCommitmentObj()];
    const wrapper = mountPanelWithEdit(commitments);
    const editBtn = wrapper.find('[data-test="edit-btn"]');
    expect(editBtn.exists()).toBe(true);
  });

  it("clicking edit button enters edit mode", async () => {
    const commitments = [makeCommitmentObj()];
    const wrapper = mountPanelWithEdit(commitments);
    const editBtn = wrapper.find('[data-test="edit-btn"]');
    await editBtn.trigger("click");
    expect(wrapper.text()).toContain("Save");
    expect(wrapper.text()).toContain("Cancel");
  });

  it("edit mode shows role and allocation inputs", async () => {
    const commitments = [makeCommitmentObj({ role: "Developer", allocation: 40 })];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    const roleInput = wrapper.find('input[type="text"]');
    expect((roleInput.element as HTMLInputElement).value).toBe("Developer");

    const allocInput = wrapper.find("input[type='number']");
    expect((allocInput.element as HTMLInputElement).value).toBe("40");
  });

  it("edit mode shows goal names as inputs with delete buttons", async () => {
    const commitments = [makeCommitmentObj({ goals: ["Goal A"] })];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    // Goal names are bound as input values, not text content
    const goalInputs = wrapper.findAll('input[type="text"]');
    const goalInput = goalInputs[1]; // first is role input, second is goal input
    expect((goalInput.element as HTMLInputElement).value).toBe("Goal A");
    expect(wrapper.find('[data-test="delete-goal-btn"]').exists()).toBe(true);
  });

  it("can add a new goal to a role", async () => {
    const commitments = [makeCommitmentObj({ goals: ["Goal A"] })];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    const addGoalBtns = wrapper.findAll('[data-test="add-goal-btn"]');
    expect(addGoalBtns.length).toBe(1);
    await addGoalBtns[0].trigger("click");

    expect((wrapper.vm as any).editingCommitments[0].goals.length).toBe(2);
  });

  it("can delete a goal from a role", async () => {
    const commitments = [makeCommitmentObj({ goals: ["A", "B"] })];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    const deleteBtns = wrapper.findAll('[data-test="delete-goal-btn"]');
    await deleteBtns[0].trigger("click");

    expect((wrapper.vm as any).editingCommitments[0].goals.length).toBe(1);
    expect((wrapper.vm as any).editingCommitments[0].goals[0]).toBe("B");
  });

  it("can add a new role", async () => {
    const commitments = [makeCommitmentObj()];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    const addRoleBtn = wrapper.find('[data-test="add-role-btn"]');
    expect(addRoleBtn.exists()).toBe(true);
    await addRoleBtn.trigger("click");

    expect((wrapper.vm as any).editingCommitments.length).toBe(2);
  });

  it("can remove a role if more than one", async () => {
    const commitments = [
      makeCommitmentObj({ role: "Dev" }),
      makeCommitmentObj({ role: "PM" }),
    ];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    const roleDeleteBtns = wrapper.findAll('[data-test="delete-role-btn"]');
    expect(roleDeleteBtns.length).toBe(2);

    await roleDeleteBtns[0].trigger("click");
    expect((wrapper.vm as any).editingCommitments.length).toBe(1);
  });

  it("last role has no delete button", async () => {
    const commitments = [makeCommitmentObj()];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    const roleDeleteBtns = wrapper.findAll('[data-test="delete-role-btn"]');
    expect(roleDeleteBtns.length).toBe(0);
  });

  it("cancel restores snapshot and returns to display mode", async () => {
    const commitments = [makeCommitmentObj({ allocation: 40 })];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    const allocInput = wrapper.find("input[type='number']");
    await allocInput.setValue(99);

    const cancelBtn = wrapper.find('[data-test="cancel-btn"]');
    await cancelBtn.trigger("click");

    expect(wrapper.text()).toContain("40.0h");
  });

  it("frontend pre-validation: empty role name blocked", async () => {
    const commitments = [makeCommitmentObj({ role: "Dev" })];
    const wrapper = mountPanelWithEdit(commitments);
    const mocks = setupTauriMocks();

    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    const roleInput = wrapper.find('input[type="text"]');
    await roleInput.setValue("");

    const saveBtn = wrapper.find('[data-test="save-btn"]');
    await saveBtn.trigger("click");

    expect(wrapper.text()).toContain("Role name cannot be empty");
    expect(mocks.invoke).not.toHaveBeenCalledWith("set_commitments", expect.anything());
  });

  it("frontend pre-validation: zero allocation blocked", async () => {
    const commitments = [makeCommitmentObj({ allocation: 40 })];
    const wrapper = mountPanelWithEdit(commitments);
    const mocks = setupTauriMocks();

    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    const allocInput = wrapper.find("input[type='number']");
    await allocInput.setValue(0);

    const saveBtn = wrapper.find('[data-test="save-btn"]');
    await saveBtn.trigger("click");

    expect(wrapper.text()).toContain("must be greater than 0");
    expect(mocks.invoke).not.toHaveBeenCalledWith("set_commitments", expect.anything());
  });

  it("frontend pre-validation: empty goal name blocked", async () => {
    const commitments = [makeCommitmentObj({ goals: ["A"] })];
    const wrapper = mountPanelWithEdit(commitments);
    const mocks = setupTauriMocks();

    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    const addGoalBtn = wrapper.find('[data-test="add-goal-btn"]');
    await addGoalBtn.trigger("click");

    const saveBtn = wrapper.find('[data-test="save-btn"]');
    await saveBtn.trigger("click");

    expect(wrapper.text()).toContain("Goal name cannot be empty");
    expect(mocks.invoke).not.toHaveBeenCalledWith("set_commitments", expect.anything());
  });

  it("save calls invoke and emits saved event on success", async () => {
    const commitments = [makeCommitmentObj({ allocation: 80 })];
    setupTauriMocks();
    const wrapper = mountPanelWithEdit(commitments);

    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    const saveBtn = wrapper.find('[data-test="save-btn"]');
    await saveBtn.trigger("click");

    expect(wrapper.emitted("saved")).toBeTruthy();
  });

  it("displays backend error", async () => {
    const commitments = [makeCommitmentObj()];
    const mocks = setupTauriMocks();
    const wrapper = mountPanelWithEdit(commitments);
    mocks.invoke.mockRejectedValueOnce("Cannot delete goal 'X': used by 3 entries this month");

    await wrapper.find('[data-test="edit-btn"]').trigger("click");

    const saveBtn = wrapper.find('[data-test="save-btn"]');
    await saveBtn.trigger("click");
    await wrapper.vm.$nextTick();
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Cannot delete goal");
  });

  it("exits edit mode when commitments prop changes externally", async () => {
    const commitments = [makeCommitmentObj()];
    const wrapper = mountPanelWithEdit(commitments);

    // Enter edit mode
    await wrapper.find('[data-test="edit-btn"]').trigger("click");
    expect((wrapper.vm as any).isEditing).toBe(true);

    // Simulate external change: file watcher pushes new data for both props
    const changedCommitments = [makeCommitmentObj({ allocation: 99 })];
    const changedProgress = [
      makeCommitmentProgress({ allocation_minutes: 99 * 60 }),
    ];
    await wrapper.setProps({
      commitments: changedCommitments,
      progress: changedProgress,
    });
    await wrapper.vm.$nextTick();

    // Should exit edit mode
    expect((wrapper.vm as any).isEditing).toBe(false);
    // Should be back in display mode showing new values
    expect(wrapper.text()).toContain("99.0h");
  });
});
