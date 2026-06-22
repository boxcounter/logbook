import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { AppStore } from "../stores/useStore";
import type { InitResult } from "../types";
import { applyInitResult } from "../utils/applyInitResult";
import { logError } from "../utils/errorLog";

/**
 * Folder selection + set_root_path + store update. Shared by SetupScreen
 * (first run) and RecoveryScreen ("Choose a different folder").
 */
export function useRootFolderPicker(store: AppStore) {
  async function pick(): Promise<void> {
    const selected = await open({ directory: true, multiple: false, title: "Select Logbook data folder" });
    if (!selected) return;
    await applyRootPath(selected as string);
  }

  async function applyRootPath(path: string): Promise<void> {
    try {
      const result = (await invoke("set_root_path", { path })) as InitResult;
      applyInitResult(store, result);
    } catch (e) {
      logError("useRootFolderPicker.applyRootPath", e);
      store.configErrors = [{ kind: "SetupError", message: `Failed: ${e}` }];
      store.configCategory = "root_missing";
      store.status = "error";
    }
  }

  return { pick };
}
