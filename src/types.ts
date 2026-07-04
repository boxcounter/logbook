// types.ts
import type { InjectionKey, Ref } from "vue";

export interface Dimension {
  name: string;
  key: string;
  source: "static" | "commitments:goals" | "commitments:role";
  values?: string[];
  required: boolean;
  deleted: boolean;
}

export interface MonthDimensions {
  dimensions: Dimension[];
  usingDefaultDimensions: boolean;
}

export interface Commitment {
  role: string;
  allocation: number; // hours per month
  goals: string[];
}

export interface CommitmentProgress {
  role: string;
  allocation_minutes: number;
  goal_spent_minutes: number;
  general_spent_minutes: number;
  goals: GoalProgress[];
}

export interface GoalProgress {
  name: string;
  spent_minutes: number;
}

export type Attribution = "ok" | "unattributed" | "mismatch";

export interface CommitmentProgressResult {
  roles: CommitmentProgress[];
  unattributed_count: number;
  unattributed_total_minutes: number;
  mismatch_count: number;
}

// Working-copy row models for the commitments editor
// (CommitmentsModal / RoleCard / GoalRow). `orig*` capture the name at load time
// so logged progress stays matched while the user retypes a name; `key` is a
// stable id for vue-draggable-plus reordering. `origName`/`origRole` are null for
// rows added during the current edit session.
export interface GoalRowModel {
  name: string;
  origName: string | null;
  key: number;
}

export interface RoleRowModel {
  role: string;
  allocation: number;
  goals: GoalRowModel[];
  origRole: string | null;
  key: number;
}

export interface Entry {
  id: string; // UUID v4
  item: string;
  duration: number; // minutes
  dimensions: Record<string, string>;
  attribution: Attribution;
}

export interface DayFile {
  note: string | null;
  entries: Entry[];
}

export interface CreateEntryInput {
  item: string;
  duration: string;
  dimensions: Record<string, string>;
}

export interface UpdateEntryInput {
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
  | { status: "DataVersionNotFound"; data: { root_path: string } }
  | { status: "DataVersionMismatch"; data: { root_path: string; expected: number; found: number } }
  | { status: "ConfigError"; data: { category: RecoveryCategory; root_path: string; errors: ConfigErrorDetail[]; scan_warnings: ScanWarning[] } }
  | {
      status: "Ready";
      data: {
        root_path: string;
        dimensions: Dimension[];
        usingDefaultDimensions: boolean;
        today: DayFile;
        commitments: Commitment[];
        scan_warnings: ScanWarning[];
      };
    };

export interface ConfigErrorDetail {
  kind: string;
  message: string;
}

export type RecoveryCategory = "in_place" | "config_missing" | "root_missing";

export type AppStatus = "loading" | "setup" | "migration_needed" | "error" | "ready";

export const UNDO_TOAST_KEY: InjectionKey<(undoFn: () => void) => void> = Symbol("triggerUndoToast");
export const SAVED_TOAST_KEY: InjectionKey<(msg: string) => void> = Symbol("triggerSavedToast");
export const FOCUS_REQUEST_KEY: InjectionKey<Ref<number>> = Symbol("focusRequestId");

