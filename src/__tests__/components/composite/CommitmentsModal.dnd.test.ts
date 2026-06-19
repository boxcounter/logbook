import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { invoke } from "@tauri-apps/api/core";
import CommitmentsModal from "../../../components/composite/CommitmentsModal.vue";
import { makeCommitment, makeCommitmentProgress } from "../../mocks/fixtures";

// vuedraggable is intentionally NOT mocked here: this guards that the alloc input
// DOM node survives a re-render (the focus-loss regression that drove the
// RoleCard/GoalRow extraction). With the real component, stepping allocation must
// patch the input in place, not remount it.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => { (invoke as any).mockReset?.(); (invoke as any).mockResolvedValue?.([]); });

// NOTE: the <Teleport> is intentionally NOT stubbed here. The teleport: true stub
// wraps the teleported subtree in an unstable boundary that breaks vuedraggable's
// keyed child reuse, remounting RoleCard on every modal re-render — a test-only
// artifact that does not happen with the real Teleport in the running app. To
// faithfully guard the production focus-stability behavior, this test renders the
// real Teleport into document.body (which jsdom provides via attachTo).
function mountModal() {
  return mount(CommitmentsModal, {
    props: {
      open: true,
      commitments: [makeCommitment({ role: "Developer", allocation: 40, goals: ["A", "B"] })],
      progress: [makeCommitmentProgress({ role: "Developer", allocation_minutes: 2400, spent_minutes: 870, goals: [{ name: "A", spent_minutes: 865 }, { name: "B", spent_minutes: 5 }] })],
      rootPath: "/tmp", selectedYear: 2026, selectedMonth: 6,
    },
    attachTo: document.body,
  });
}

describe("CommitmentsModal — DnD focus stability (real vuedraggable)", () => {
  it("keeps the same allocation input DOM node across a stepper change", async () => {
    const w = mountModal();
    const before = document.querySelector("[data-test='alloc']") as HTMLInputElement;
    before.setAttribute("data-marker", "X");
    await w.findComponent({ name: "RoleCard" }).find("[data-test='alloc-inc']").trigger("click"); // 40 -> 45
    const after = document.querySelector("[data-test='alloc']") as HTMLInputElement;
    expect(after.getAttribute("data-marker")).toBe("X"); // same node reused, not remounted
    expect(after.value).toBe("45");
  });
});
