export interface Dimension {
  name: string;
  key: string;
  source: "static" | "monthly";
  values?: string[];
}

export interface Config {
  dimensions: Dimension[];
}

export interface Commitment {
  role: string;
  allocation: number; // hours per week
  goals: string[];
}

export interface Entry {
  id: string; // UUID v4
  item: string;
  duration: number; // minutes
  dimensions: Record<string, string>;
}

export interface DayFile {
  note: string | null;
  entries: Entry[];
}

export interface NewEntry {
  item: string;
  duration: string;
  dimensions: Record<string, string>;
}

export interface UpdateEntry {
  item?: string;
  duration?: string;
  dimensions?: Record<string, string>;
}

export type InitResult =
  | { status: "NeedsSetup" }
  | { status: "ConfigError"; data: ConfigErrorDetail[] }
  | {
      status: "Ready";
      data: { config: Config; today: DayFile; commitments: Commitment[] };
    };

export interface ConfigErrorDetail {
  kind: string;
  message: string;
}

export type Screen = "loading" | "setup" | "error" | "ready";
