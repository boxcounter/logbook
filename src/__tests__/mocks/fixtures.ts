import type {
  Entry,
  Dimension,
  Commitment,
  DayFile,
  ConfigErrorDetail,
  CommitmentProgress,
} from "../../types";

// ============================================================
// Test data factories. Each returns a valid default object
// merged with the caller's overrides.
//
// Usage:
//   import { makeEntry, makeDimensions, makeCommitment } from "../mocks/fixtures";
//   const entry = makeEntry({ item: "Meeting", duration: 60 });
// ============================================================

let _uuidCounter = 0;

function fakeUuid(): string {
  _uuidCounter += 1;
  const n = String(_uuidCounter).padStart(12, "0");
  return `00000000-0000-0000-0000-${n}`;
}

export function makeEntry(overrides?: Partial<Entry>): Entry {
  return {
    id: fakeUuid(),
    item: "Test entry",
    duration: 30,
    dimensions: {},
    attribution: "ok",
    ...overrides,
  };
}

export function makeDimension(overrides?: Partial<Dimension>): Dimension {
  return {
    name: "Goal",
    key: "goal",
    source: "monthly",
    required: false,
    deleted: false,
    ...overrides,
  };
}

export function makeDimensions(): Dimension[] {
  return [
    makeDimension({ name: "Goal", key: "goal", source: "monthly", required: true }),
    makeDimension({ name: "Business Line", key: "business-line", source: "static", values: ["Platform", "Growth"], required: true }),
    makeDimension({ name: "Category", key: "category", source: "static", values: ["Coding", "Meeting"], required: false }),
  ];
}

export function makeCommitment(overrides?: Partial<Commitment>): Commitment {
  return {
    role: "Developer",
    allocation: 40,
    goals: ["Ship feature X", "Code review"],
    ...overrides,
  };
}

export function makeCommitmentProgress(overrides?: Partial<CommitmentProgress>): CommitmentProgress {
  return {
    role: "Developer",
    allocation_minutes: 2400,
    goal_spent_minutes: 0,
    general_spent_minutes: 0,
    goals: [
      { name: "Ship feature X", spent_minutes: 0 },
      { name: "Code review", spent_minutes: 0 },
    ],
    ...overrides,
  };
}

export function makeDayFile(overrides?: Partial<DayFile>): DayFile {
  return {
    note: null,
    entries: [],
    ...overrides,
  };
}

export function makeConfigErrors(): ConfigErrorDetail[] {
  return [
    { kind: "MissingName", message: "Dimension 0 has an empty name" },
    { kind: "MissingKey", message: 'Dimension missing key' },
  ];
}
