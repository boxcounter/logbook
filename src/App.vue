<script setup lang="ts">
import { onMounted, onUnmounted, ref, provide } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useStore } from "./stores/useStore";
import SetupScreen from "./components/SetupScreen.vue";
import ConfigErrorBanner from "./components/ConfigErrorBanner.vue";
import MonthView from "./components/MonthView.vue";
import Toast from "./components/base/Toast.vue";
import type { InitResult, ConfigErrorDetail, ScanWarning } from "./types";
import { logError, logInfo } from "./utils/errorLog";

const store = useStore();
const showUndoToast = ref(false);
const undoAction = ref<(() => void) | null>(null);
let undoTimer: ReturnType<typeof setTimeout> | null = null;

const showScanWarning = ref(false);
const scanWarnings = ref<ScanWarning[]>([]);

// #1: window focus → auto-focus input
const focusRequestId = ref(0);

function todayStr(): string {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
}
let lastKnownToday = todayStr();

// Store listener handles for cleanup (prevents HMR duplication)
let unlistenConfig: (() => void) | null = null;
let unlistenCommitments: (() => void) | null = null;
let unlistenFocus: (() => void) | null = null;

onMounted(async () => {
  try {
    unlistenConfig = await listen<ConfigErrorDetail[]>("config-changed", (event) => {
      if (event.payload.length === 0) {
        initApp();
      } else {
        store.configErrors = event.payload;
        store.status = "error";
      }
    });

    unlistenCommitments = await listen<ConfigErrorDetail[]>("commitments-changed", () => {
      initApp();
    });

    unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused) return;
      focusRequestId.value++;
      const newToday = todayStr();
      if (newToday === lastKnownToday) return; // same calendar day: leave the view alone
      // Midnight crossed since we were last focused.
      if (store.currentDate === lastKnownToday && store.status === "ready") {
        store.currentDate = newToday; // we were following "today" → follow to the new today
        initApp();
      }
      lastKnownToday = newToday;
    });
  } catch (e) {
    logError("App.onMounted", e);
  }

  initApp();
});

onUnmounted(() => {
  unlistenConfig?.();
  unlistenCommitments?.();
  unlistenFocus?.();
  if (undoTimer) clearTimeout(undoTimer);
});

async function initApp() {
  logInfo("App.initApp", "start");
  try {
    const result = (await invoke("init")) as InitResult;
    switch (result.status) {
      case "NeedsSetup":
        store.status = "setup";
        break;
      case "ConfigError":
        store.configErrors = result.data.errors;
        store.status = "error";
        if (result.data.scan_warnings.length > 0) {
          scanWarnings.value = result.data.scan_warnings;
          showScanWarning.value = true;
        }
        break;
      case "Ready":
        store.rootPath = result.data.root_path;
        store.config = result.data.config;
        store.today = result.data.today;
        store.commitments = result.data.commitments;
        store.status = "ready";
        if (result.data.scan_warnings.length > 0) {
          scanWarnings.value = result.data.scan_warnings;
          showScanWarning.value = true;
        }
        break;
    }
    logInfo("App.initApp", result.status);
  } catch (e) {
    logError("App.initApp", e);
    store.configErrors = [{ kind: "InitError", message: `Failed: ${e}` }];
    store.status = "error";
  }
}

// Undo toast for delete
function triggerUndoToast(undoFn: () => void) {
  if (undoTimer) clearTimeout(undoTimer);
  undoAction.value = undoFn;
  showUndoToast.value = true;
  undoTimer = setTimeout(() => {
    showUndoToast.value = false;
    undoAction.value = null;
  }, 5000);
}

function dismissUndo() {
  if (undoTimer) clearTimeout(undoTimer);
  showUndoToast.value = false;
  undoAction.value = null;
}

function handleUndo() {
  if (undoAction.value) undoAction.value();
  dismissUndo();
}

function dismissScanWarning() {
  showScanWarning.value = false;
  scanWarnings.value = [];
}

// Provide undo trigger to descendants
provide("triggerUndoToast", triggerUndoToast);
// #1: window focus → auto-focus input
provide("focusRequestId", focusRequestId);
</script>

<template>
  <div class="min-h-screen">
    <div v-if="store.status === 'loading'" class="flex items-center justify-center min-h-screen text-gray-500">
      Loading…
    </div>
    <SetupScreen v-else-if="store.status === 'setup'" />
    <template v-else-if="store.status === 'error'">
      <ConfigErrorBanner />
      <button
        class="mx-lg mt-lg px-lg py-sm bg-blue-600 text-white rounded-[var(--radius-form-lg)] hover:bg-blue-700 text-secondary"
        @click="initApp"
      >
        Retry
      </button>
    </template>
    <div v-else-if="store.status === 'ready'" class="p-2xl">
      <MonthView />
    </div>

    <!-- Undo Toast -->
    <Toast
      :show="showUndoToast"
      message="Entry deleted"
      undo-label="Undo"
      @undo="handleUndo"
      @dismiss="dismissUndo"
    />

    <!-- Scan Warning Toast -->
    <Toast
      :show="showScanWarning"
      :message="`${scanWarnings.length} data issue${scanWarnings.length > 1 ? 's' : ''} found during scan`"
      @dismiss="dismissScanWarning"
    />
  </div>
</template>

<style>
</style>
