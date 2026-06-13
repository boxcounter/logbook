import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import CommitmentsPanel from "../../components/CommitmentsPanel.vue";
import { makeCommitment, makeEntry } from "../mocks/fixtures";

const devCommitment = makeCommitment({
  role: "Developer",
  allocation: 40,
  goals: ["Ship feature X", "Code review"],
});

const dirCommitment = makeCommitment({
  role: "Director",
  allocation: 20,
  goals: ["Hiring"],
});

function mountPanel(commitments: typeof devCommitment[], entries: ReturnType<typeof makeEntry>[]) {
  return mount(CommitmentsPanel, {
    props: { commitments, entries },
  });
}

// Helper: create an entry with a goal dimension set
function goalEntry(goalName: string, durationMinutes: number) {
  return makeEntry({ dimensions: { goal: goalName }, duration: durationMinutes });
}

// ============================================================

describe("CommitmentsPanel", () => {
  it("renders nothing when commitments empty", () => {
    const wrapper = mountPanel([], []);
    expect(wrapper.find(".bg-white").exists()).toBe(false);
  });

  it("renders each commitment role", () => {
    const wrapper = mountPanel([devCommitment, dirCommitment], []);
    const text = wrapper.text();
    expect(text).toContain("Developer");
    expect(text).toContain("Director");
  });

  it("computes daily allocation correctly (40h/month → 120 min/day)", () => {
    const wrapper = mountPanel([devCommitment], []);
    // 40 * 60 / 20 = 120 minutes = 2h/day
    expect(wrapper.text()).toContain("2.0h");
  });

  it("shows spent / allocation ratio text", () => {
    const wrapper = mountPanel([devCommitment], []);
    // formatDuration(0) = "0m"
    expect(wrapper.text()).toContain("0m");
    expect(wrapper.text()).toContain("2.0h");
  });

  it("progress bar width reflects percentage", () => {
    // 60 minutes spent out of 120 allocated = 50%
    const entry = goalEntry("Code review", 60);
    const wrapper = mountPanel([devCommitment], [entry]);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.attributes("style")).toContain("width: 50%");
  });

  it("green bar when spent < 80% of allocation", () => {
    // 60/120 = 50% < 80% → green
    const entry = goalEntry("Code review", 60);
    const wrapper = mountPanel([devCommitment], [entry]);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.classes()).toContain("bg-green-500");
  });

  it("yellow bar when spent between 80-100%", () => {
    // 100/120 ≈ 83% → yellow
    const entry = goalEntry("Code review", 100);
    const wrapper = mountPanel([devCommitment], [entry]);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.classes()).toContain("bg-yellow-500");
  });

  it("red bar when spent > 100% of allocation", () => {
    const entry = goalEntry("Code review", 150);
    const wrapper = mountPanel([devCommitment], [entry]);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.classes()).toContain("bg-red-500");
  });

  it("renders goal breakdown with names and times", () => {
    const e1 = goalEntry("Code review", 30);
    const e2 = goalEntry("Code review", 15);
    const wrapper = mountPanel([devCommitment], [e1, e2]);
    const text = wrapper.text();
    expect(text).toContain("Code review");
    expect(text).toContain("Ship feature X");
    // Code review total = 45m
    expect(text).toContain("45m");
  });

  it("zero allocation shows 0% width and gray bar", () => {
    const zeroAlloc = makeCommitment({ role: "Advisor", allocation: 0, goals: [] });
    const entry = goalEntry("Code review", 60);
    const wrapper = mountPanel([zeroAlloc], [entry]);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.attributes("style")).toContain("width: 0%");
    expect(bar.classes()).toContain("bg-gray-300");
  });

  it("clamps progress bar width to 100%", () => {
    // 200/120 > 100% → clamped to 100%
    const entry = goalEntry("Code review", 200);
    const wrapper = mountPanel([devCommitment], [entry]);
    const bar = wrapper.find(".h-1\\.5 > div");
    expect(bar.attributes("style")).toContain("width: 100%");
  });
});
