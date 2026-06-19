// src/__tests__/components/CommitmentsPanel.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import CommitmentsPanel from "../../components/CommitmentsPanel.vue";
import { makeCommitmentProgress, makeCommitment } from "../mocks/fixtures";

function mountPanel() {
  return mount(CommitmentsPanel, {
    props: {
      progress: [makeCommitmentProgress({ role: "Developer", spent_minutes: 1230, allocation_minutes: 2400 })],
      commitments: [makeCommitment()],
      rootPath: "/x",
      selectedYear: 2026,
      selectedMonth: 6,
    },
  });
}

describe("CommitmentsPanel", () => {
  it("renders role name and mono spent/allocation", () => {
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("Developer");
    expect(wrapper.text()).toContain("20h 30m"); // 1230m
    expect(wrapper.text()).toContain("40"); // allocation hours
  });

  it("progress fill uses the brand gradient (single style, no status colors)", () => {
    const wrapper = mountPanel();
    const fill = wrapper.find("[data-test='progress-fill']");
    expect(fill.attributes("class") || "").not.toMatch(/bg-(orange|yellow|green|red)-/);
  });

  it("toggles the goal list for a role", async () => {
    const wrapper = mountPanel();
    const goalRows = () => wrapper.findAll("[data-test='goal-row']");
    const initial = goalRows().length;
    await wrapper.find("[data-test='role-toggle']").trigger("click");
    expect(goalRows().length).not.toBe(initial);
  });
});
