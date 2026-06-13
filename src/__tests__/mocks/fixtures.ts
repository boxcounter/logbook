import type {
  Entry,
  Config,
  Dimension,
  Commitment,
  DayFile,
  ConfigErrorDetail,
} from "../../types";

// ============================================================
// Test data factories. Each returns a valid default object
// merged with the caller's overrides.
//
// Usage:
//   import { makeEntry, makeConfig, makeCommitment } from "../mocks/fixtures";
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
    ...overrides,
  };
}

export function makeDimension(overrides?: Partial<Dimension>): Dimension {
  return {
    name: "Goal",
    key: "goal",
    source: "monthly",
    required: false,
    ...overrides,
  };
}

export function makeConfig(overrides?: Partial<Config>): Config {
  return {
    dimensions: [
      makeDimension({ name: "Goal", key: "goal", source: "monthly", required: true }),
      makeDimension({ name: "Business Line", key: "business-line", source: "static", values: ["Platform", "Growth"], required: true }),
      makeDimension({ name: "Category", key: "category", source: "static", values: ["Coding", "Meeting"], required: false }),
    ],
    ...overrides,
  };
}

export function makeCommitment(overrides?: Partial<Commitment>): Commitment {
  return {
    role: "Developer",
    allocation: 40,
    goals: ["Ship feature X", "Code review"],
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
