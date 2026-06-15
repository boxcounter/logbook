import { vi, type Mock } from "vitest";

// ============================================================
// Shared Tauri IPC mock hub for component tests.
//
// Usage (at top of each component test file):
//   import { setupTauriMocks } from "../mocks/tauri";
//   const mocks = setupTauriMocks();
//
//   // Override per-test:
//   mocks.invoke.mockResolvedValueOnce(...);
//
// Each mock module is hoist-safe because setupTauriMocks()
// calls vi.mock() internally before returning the mocks.
// ============================================================

export interface TauriMocks {
  invoke: Mock;
  listen: Mock;
  unlisten: () => void;
  onFocusChanged: Mock;
  getCurrentWindow: Mock;
  openDialog: Mock;
}

// Default responses for every Tauri command
const defaultInvoke = async (cmd: string, _args?: unknown) => {
  switch (cmd) {
    case "init":
      return { status: "NeedsSetup" };
    case "set_root_path":
      return { status: "NeedsSetup" };
    case "get_entries":
      return { note: null, entries: [] } as unknown;
    case "append_entry":
      return { id: "00000000-0000-0000-0000-000000000000", item: "", duration: 0, dimensions: {} } as unknown;
    case "update_entry":
      return { note: null, entries: [] } as unknown;
    case "delete_entry":
      return { note: null, entries: [] } as unknown;
    case "set_day_note":
      return { note: "", entries: [] } as unknown;
    case "open_in_editor":
      return;
    case "create_starter_files":
      return;
    case "log_error":
    case "log_info":
      return;
    case "set_commitments":
      return (_args as any)?.commitments ?? [];
    default:
      throw new Error(`Unmocked invoke command: ${cmd}`);
  }
};

export function setupTauriMocks(): TauriMocks {
  const invoke = vi.fn().mockImplementation(defaultInvoke);
  const listen = vi.fn().mockResolvedValue(() => {});
  const onFocusChanged = vi.fn().mockResolvedValue(() => {});
  const getCurrentWindow = vi.fn().mockReturnValue({ onFocusChanged });
  const openDialog = vi.fn().mockResolvedValue(null); // user cancels by default

  vi.mock("@tauri-apps/api/core", () => ({ invoke }));
  vi.mock("@tauri-apps/api/event", () => ({ listen }));
  vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow }));
  vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openDialog }));

  return {
    invoke,
    listen,
    unlisten: () => {},
    onFocusChanged,
    getCurrentWindow,
    openDialog,
  };
}
