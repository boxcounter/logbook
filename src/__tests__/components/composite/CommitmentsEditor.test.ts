import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import CommitmentsEditor from "../../../components/composite/CommitmentsEditor.vue";
import { makeCommitment } from "../../mocks/fixtures";

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const commitments = [makeCommitment({ role: "Dev", allocation: 40, goals: ["Ship"] })];

function mountEditor(props = {}) {
  return mount(CommitmentsEditor, {
    props: {
      commitments,
      rootPath: "/tmp",
      selectedYear: 2026,
      selectedMonth: 7,
      ...props,
    },
  });
}

describe("CommitmentsEditor", () => {
  it("renders role and goal fields", () => {
    const wrapper = mountEditor();
    const roleInput = wrapper.find("input[placeholder='Role']") as any;
    expect(roleInput.element.value).toBe("Dev");
    const goalInput = wrapper.find("input[placeholder='Goal name']") as any;
    expect(goalInput.element.value).toBe("Ship");
  });

  it("adds a goal row on + Add Goal click", async () => {
    const wrapper = mountEditor();
    const addBtns = wrapper.findAll("button");
    const addGoalBtn = [...addBtns].find(b => b.text().includes("Add Goal"));
    expect(addGoalBtn).toBeTruthy();
    if (addGoalBtn) {
      await addGoalBtn.trigger("click");
      const goalInputs = wrapper.findAll("input[placeholder='Goal name']");
      expect(goalInputs.length).toBeGreaterThanOrEqual(2);
    }
  });

  it("adds a role on + Add Role click", async () => {
    const wrapper = mountEditor();
    const btns = wrapper.findAll("button");
    const addRoleBtn = [...btns].find(b => b.text().includes("Add Role"));
    expect(addRoleBtn).toBeTruthy();
    if (addRoleBtn) {
      await addRoleBtn.trigger("click");
      const roleInputs = wrapper.findAll("input[placeholder='Role']");
      expect(roleInputs.length).toBe(2);
    }
  });

  it("shows validation error for empty role name", async () => {
    const wrapper = mountEditor();
    const roleInput = wrapper.find("input[placeholder='Role']");
    await roleInput.setValue("");
    const saveBtn = [...wrapper.findAll("button")].find(b => b.text().includes("Save"));
    expect(saveBtn).toBeTruthy();
    if (saveBtn) {
      await saveBtn.trigger("click");
      expect(wrapper.text()).toContain("Role name cannot be empty");
    }
  });

  it("emits cancel on Cancel click", async () => {
    const wrapper = mountEditor();
    const cancelBtn = [...wrapper.findAll("button")].find(b => b.text().includes("Cancel"));
    expect(cancelBtn).toBeTruthy();
    if (cancelBtn) {
      await cancelBtn.trigger("click");
      expect(wrapper.emitted("cancel")).toHaveLength(1);
    }
  });
});
