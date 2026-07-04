import { computed, ref, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppStore } from "../stores/useStore";
import { logError } from "../utils/errorLog";
import { HIGHLIGHT_DURATION } from "../utils/constants";

export function useFileActions(store: AppStore) {
  const dayFilePath = computed(() => {
    if (!store.rootPath) return "";
    const d = store.currentDate;
    return `${d.slice(0, 4)}/${d.slice(5, 7)}/${d}.md`;
  });

  const displayPath = computed(() => (store.rootPath ? `…/${dayFilePath.value}` : ""));

  async function revealDayFile() {
    if (!store.rootPath) return;
    try {
      await invoke("reveal_day_file", { rootPath: store.rootPath, date: store.currentDate });
    } catch (e) {
      logError("useFileActions.revealDayFile", e);
    }
  }

  const copiedFeedback = ref(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;

  async function copyFilePath(e: MouseEvent) {
    e.preventDefault();
    if (!store.rootPath) return;
    await navigator.clipboard.writeText(store.rootPath + "/" + dayFilePath.value);
    copiedFeedback.value = true;
    if (copyTimer) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => { copiedFeedback.value = false; }, HIGHLIGHT_DURATION);
  }

  onUnmounted(() => {
    if (copyTimer) clearTimeout(copyTimer);
  });

  return { dayFilePath, displayPath, revealDayFile, copyFilePath, copiedFeedback };
}
