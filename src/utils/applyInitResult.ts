import type { AppStore } from "../stores/useStore";
import type { InitResult, ScanWarning } from "../types";

/**
 * Map an InitResult onto the store and return scan_warnings for the caller to
 * surface (toast). Shared by App.initApp and useRootFolderPicker so the two
 * entry points stay in sync.
 */
export function applyInitResult(store: AppStore, result: InitResult): ScanWarning[] {
  switch (result.status) {
    case "NeedsSetup":
      store.status = "setup";
      return [];
    case "DataVersionNotFound":
      store.rootPath = result.data.root_path;
      store.dataVersionMessage = "Data version file not found. Please run the Logbook migration tool to initialize your data directory.";
      store.status = "migration_needed";
      return [];
    case "DataVersionMismatch":
      store.rootPath = result.data.root_path;
      store.dataVersionMessage = `Data format version mismatch. Expected version ${result.data.expected}, found version ${result.data.found}. Please run the Logbook migration tool to update your data directory.`;
      store.status = "migration_needed";
      return [];
    case "ConfigError":
      store.configErrors = result.data.errors;
      store.configCategory = result.data.category;
      store.rootPath = result.data.root_path;
      store.status = "error";
      return result.data.scan_warnings;
    case "Ready":
      store.rootPath = result.data.root_path;
      store.dimensions = result.data.dimensions;
      store.usingDefaultDimensions = result.data.usingDefaultDimensions;
      store.today = result.data.today;
      store.commitments = result.data.commitments;
      store.configCategory = null;
      store.status = "ready";
      return result.data.scan_warnings;
  }
}
