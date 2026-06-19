// src/__tests__/components/TwoLineInput.test.ts
import { describe, it, expect, afterEach } from "vitest";
import { mount, enableAutoUnmount } from "@vue/test-utils";
import TwoLineInput from "../../components/TwoLineInput.vue";
import { makeDimension, makeCommitment } from "../mocks/fixtures";

// The popover registers a window keydown listener; unmount after each test.
enableAutoUnmount(afterEach);

const dimensions = [
  makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering"], required: true }),
  makeDimension({ name: "Goal", key: "goal", source: "monthly", required: true }),
];
const commitments = [makeCommitment({ goals: ["Bug fixes"] })];

function mountInput(initialValues: Record<string, string> = {}) {
  return mount(TwoLineInput, { props: { dimensions, commitments, initialValues } });
}

describe("TwoLineInput", () => {
  it("renders the item input and the Enter hint", () => {
    const wrapper = mountInput();
    expect(wrapper.find("input").exists()).toBe(true);
    expect(wrapper.text()).toContain("⏎");
  });

  it("shows a duration token parsed from the item text", async () => {
    const wrapper = mountInput();
    await wrapper.find("input").setValue("Code review 1.5h");
    expect(wrapper.find("[data-test='dur-token']").text()).toContain("1h 30m");
  });

  it("shows a missing indicator per unfilled required dimension", () => {
    const wrapper = mountInput();
    const missing = wrapper.findAll("[data-test='missing']");
    expect(missing.length).toBe(2);
    expect(wrapper.text()).toContain("Category");
    expect(wrapper.text()).toContain("Goal");
  });

  it("emits submit with item, minutes, and dimensions on Enter", async () => {
    const wrapper = mountInput({ category: "Engineering" });
    await wrapper.find("input").setValue("Code review 1h");
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")?.[0]).toEqual(["Code review", 60, { category: "Engineering" }]);
  });

  it("does NOT emit submit when there is no parseable duration", async () => {
    const wrapper = mountInput({ category: "Engineering" });
    await wrapper.find("input").setValue("Code review");
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")).toBeFalsy();
    expect(wrapper.text()).toContain("Need a duration");
  });

  it("submits even when required dimensions are missing (soft hint)", async () => {
    const wrapper = mountInput(); // nothing filled
    await wrapper.find("input").setValue("Quick note 30m");
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")?.[0]).toEqual(["Quick note", 30, {}]);
  });

  it("opens DimensionPopover on @ keydown", async () => {
    const wrapper = mountInput();
    await wrapper.find("input").trigger("keydown", { key: "@" });
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);
  });

  it("Enter submits even while the popover is open (does not swallow Enter)", async () => {
    const wrapper = mountInput({ category: "Engineering" });
    await wrapper.find("input").setValue("Code review 1h");
    await wrapper.find("input").trigger("keydown", { key: "@" }); // open popover
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")?.[0]).toEqual(["Code review", 60, { category: "Engineering" }]);
  });

  it("opens the popover upward (bottom-full) since the input is bottom-anchored", async () => {
    // The entry list (flex-1) pushes the input to the bottom of the card, whose
    // overflow-hidden would clip a downward popover. It must open upward.
    const wrapper = mountInput();
    await wrapper.find("input").trigger("keydown", { key: "@" });
    const popover = wrapper.findComponent({ name: "DimensionPopover" });
    expect(popover.classes()).toContain("bottom-full");
    expect(popover.classes()).not.toContain("top-full");
  });

  it("removes a dimension token when its × is clicked", async () => {
    const wrapper = mountInput({ category: "Engineering" });
    expect(wrapper.find("[data-test='dim-token']").exists()).toBe(true);
    await wrapper.find("[data-test='dim-token-remove']").trigger("click");
    expect(wrapper.find("[data-test='dim-token']").exists()).toBe(false);
  });

  it("clearInput() empties the field", async () => {
    const wrapper = mountInput();
    await wrapper.find("input").setValue("Something 1h");
    (wrapper.vm as unknown as { clearInput: () => void }).clearInput();
    await wrapper.vm.$nextTick();
    expect((wrapper.find("input").element as HTMLInputElement).value).toBe("");
  });
});
