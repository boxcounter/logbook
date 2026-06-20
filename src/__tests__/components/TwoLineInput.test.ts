// src/__tests__/components/TwoLineInput.test.ts
import { describe, it, expect, afterEach } from "vitest";
import { ref } from "vue";
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

  it("shows only @ and # hints, not month-navigation hints", () => {
    const wrapper = mountInput();
    expect(wrapper.text()).toContain("dim");
    expect(wrapper.text()).toContain("time");
    expect(wrapper.text()).not.toContain("prev month");
    expect(wrapper.text()).not.toContain("next month");
  });

  it("emits submit with item, minutes, and dimensions on Enter (all required filled)", async () => {
    const wrapper = mountInput({ category: "Engineering", goal: "Bug fixes" });
    await wrapper.find("input").setValue("Code review 1h");
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")?.[0]).toEqual(["Code review", 60, { category: "Engineering", goal: "Bug fixes" }]);
  });

  it("does NOT emit submit when there is no parseable duration", async () => {
    const wrapper = mountInput({ category: "Engineering" });
    await wrapper.find("input").setValue("Code review");
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")).toBeFalsy();
    expect(wrapper.text()).toContain("Need a duration");
  });

  it("does NOT submit when a required dimension is unfilled, and flags the missing chips", async () => {
    const wrapper = mountInput(); // category & goal required & unfilled
    await wrapper.find("input").setValue("Quick note 30m");
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")).toBeFalsy();
    // after a blocked attempt the missing chips are emphasized in the warning color
    expect(wrapper.find("[data-test='missing']").classes()).toContain("text-[var(--color-warning)]");
  });

  it("opens DimensionPopover on @ keydown", async () => {
    const wrapper = mountInput();
    await wrapper.find("input").trigger("keydown", { key: "@" });
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);
  });

  it("closes the dimension popover when clicking outside the composer", async () => {
    const wrapper = mountInput();
    await wrapper.find("input").trigger("keydown", { key: "@" });
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);
    document.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(false);
  });

  it("Enter submits even while the popover is open (does not swallow Enter)", async () => {
    const wrapper = mountInput({ category: "Engineering", goal: "Bug fixes" });
    await wrapper.find("input").setValue("Code review 1h");
    await wrapper.find("input").trigger("keydown", { key: "@" }); // open popover
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")?.[0]).toEqual(["Code review", 60, { category: "Engineering", goal: "Bug fixes" }]);
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

  it("focuses the input on a focus request even when a non-editable element holds focus", async () => {
    const fid = ref(0);
    const btn = document.createElement("button");
    document.body.appendChild(btn);
    const wrapper = mount(TwoLineInput, {
      props: { dimensions, commitments, initialValues: {} },
      attachTo: document.body,
      global: { provide: { focusRequestId: fid } },
    });
    btn.focus();
    expect(document.activeElement).toBe(btn);
    fid.value++;
    await wrapper.vm.$nextTick();
    expect(document.activeElement).toBe(wrapper.find("input").element);
    wrapper.unmount();
    btn.remove();
  });

  it("does not steal focus from an active editable element on a focus request", async () => {
    const fid = ref(0);
    const other = document.createElement("input");
    document.body.appendChild(other);
    const wrapper = mount(TwoLineInput, {
      props: { dimensions, commitments, initialValues: {} },
      attachTo: document.body,
      global: { provide: { focusRequestId: fid } },
    });
    other.focus();
    expect(document.activeElement).toBe(other);
    fid.value++;
    await wrapper.vm.$nextTick();
    expect(document.activeElement).toBe(other); // not stolen
    wrapper.unmount();
    other.remove();
  });

  it("exposes focusInput() to focus the entry input", async () => {
    const wrapper = mount(TwoLineInput, {
      props: { dimensions, commitments, initialValues: {} },
      attachTo: document.body,
    });
    (wrapper.vm as unknown as { focusInput: () => void }).focusInput();
    expect(document.activeElement).toBe(wrapper.find("input").element);
    wrapper.unmount();
  });

  it("Esc clears typed text without emitting submit", async () => {
    const wrapper = mount(TwoLineInput, {
      props: { dimensions: [], commitments: [], initialValues: {} },
    });
    const input = wrapper.find("input");
    await input.setValue("draft work 1h");
    await input.trigger("keydown", { key: "Escape" });
    expect((input.element as HTMLInputElement).value).toBe("");
    expect(wrapper.emitted("submit")).toBeFalsy();
  });

  it("Esc clears a selected dimension token even with no text", async () => {
    // Covers the JSON.stringify(dimValues) !== initialValues half of hasContent:
    // dims are dirty but the text is empty, so esc must reset dims (not submit).
    const wrapper = mountInput({}); // initial dims empty
    // Drive a dimension selection through the popover so dimValues differs from {}.
    await wrapper.find("input").trigger("keydown", { key: "@" }); // open popover
    await wrapper.findComponent({ name: "DimensionPopover" }).vm.$emit("select", "category", "Engineering");
    await wrapper.vm.$nextTick();
    expect(wrapper.find("[data-test='dim-token']").exists()).toBe(true); // token present
    // Close the popover so esc is owned by the input, then esc with no text.
    await wrapper.findComponent({ name: "DimensionPopover" }).vm.$emit("close");
    await wrapper.vm.$nextTick();
    await wrapper.find("input").trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("submit")).toBeFalsy();
    expect(wrapper.find("[data-test='dim-token']").exists()).toBe(false); // dims reset to {}
  });

  it("Esc on an empty input does nothing", async () => {
    const wrapper = mount(TwoLineInput, {
      props: { dimensions: [], commitments: [], initialValues: {} },
    });
    const input = wrapper.find("input");
    await input.trigger("keydown", { key: "Escape" });
    expect((input.element as HTMLInputElement).value).toBe("");
    expect(wrapper.emitted("submit")).toBeFalsy();
  });
});
