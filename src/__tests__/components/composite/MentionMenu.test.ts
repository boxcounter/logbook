import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import MentionMenu from "../../../components/composite/MentionMenu.vue";

const mockDimensions = [
  { name: "Goal", key: "goal", source: "monthly" as const, required: true },
  { name: "Category", key: "category", source: "static" as const, values: ["Coding", "Meeting"], required: false },
];

const mockCommitments = [
  { role: "Dev", allocation: 40, goals: ["Ship it", "Review"] },
];

describe("MentionMenu", () => {
  it("renders dimension names in dim phase", () => {
    const wrapper = mount(MentionMenu, {
      props: { dimensions: mockDimensions, commitments: mockCommitments, dimValues: {} },
    });
    expect(wrapper.text()).toContain("Goal");
    expect(wrapper.text()).toContain("Category");
    expect(wrapper.text()).toContain("Pick a dimension");
  });

  it("shows 'Required' for unfilled required dims", () => {
    const wrapper = mount(MentionMenu, {
      props: { dimensions: mockDimensions, commitments: mockCommitments, dimValues: {} },
    });
    expect(wrapper.text()).toContain("Required");
  });

  it("enters val phase on dimension click", async () => {
    const wrapper = mount(MentionMenu, {
      props: { dimensions: mockDimensions, commitments: mockCommitments, dimValues: {} },
    });
    // Click the first dim item (Goal)
    const dimItems = wrapper.findAll(".cursor-pointer");
    await dimItems[0].trigger("click");
    expect(wrapper.text()).toContain("Pick a value");
  });

  it("emits select when a value is chosen", async () => {
    const wrapper = mount(MentionMenu, {
      props: { dimensions: mockDimensions, commitments: mockCommitments, dimValues: { goal: "" } },
    });
    // Click Goal dimension
    const dimItems = wrapper.findAll(".cursor-pointer");
    await dimItems[0].trigger("click");
    await wrapper.vm.$nextTick();
    // Click first val option (index 1 — index 0 is the back button)
    const valItems = wrapper.findAll(".cursor-pointer");
    await valItems[1].trigger("click");
    expect(wrapper.emitted("select")).toBeTruthy();
  });

  it("emits close when all required are filled", async () => {
    const wrapper = mount(MentionMenu, {
      props: { dimensions: mockDimensions, commitments: mockCommitments, dimValues: { goal: "Ship it" } },
    });
    // Click Goal dimension
    const dimItems = wrapper.findAll(".cursor-pointer");
    await dimItems[0].trigger("click");
    await wrapper.vm.$nextTick();
    // Click a value (index 1 — index 0 is the back button)
    const valItems = wrapper.findAll(".cursor-pointer");
    await valItems[1].trigger("click");
    expect(wrapper.emitted("close")).toBeTruthy();
  });
});
