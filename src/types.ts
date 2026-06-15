export interface Dimension {
  name: string;
  key: string;
  source: "static" | "monthly";
  values?: string[];
  required: boolean;
}

export interface Config {
  dimensions: Dimension[];
}

export interface Commitment {
  role: string;
  allocation: number; // hours per month
  goals: string[];
}

export interface CommitmentProgress {
  role: string;
  allocation_minutes: number;
  spent_minutes: number;
  goals: GoalProgress[];
}

export interface GoalProgress {
  name: string;
  spent_minutes: number;
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

export interface ScanWarning {
  kind: string;   // "SkippedFile" | "CorruptedFile" | "OrphanedTemp"
  path: string;   // relative to root_path
  message: string;
}

export type InitResult =
  | { status: "NeedsSetup" }
  | { status: "ConfigError"; data: { errors: ConfigErrorDetail[]; scan_warnings: ScanWarning[] } }
  | {
      status: "Ready";
      data: {
        root_path: string;
        config: Config;
        today: DayFile;
        commitments: Commitment[];
        scan_warnings: ScanWarning[];
      };
    };

export interface ConfigErrorDetail {
  kind: string;
  message: string;
}

export type Screen = "loading" | "setup" | "error" | "ready";

