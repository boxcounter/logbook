<script setup lang="ts">
import { onMounted, onUnmounted, ref, provide } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useStore } from "./stores/useStore";
import { yearMonthFromDate, rolloverDecision, formatDate } from "./utils/dates";
import { SAVED_TOAST_DURATION, UNDO_DELETE_DELAY } from "./utils/constants";
import SetupScreen from "./components/SetupScreen.vue";
import RecoveryScreen from "./components/RecoveryScreen.vue";
import DataVersionScreen from "./components/DataVersionScreen.vue";
import MonthView from "./components/MonthView.vue";
import Toast from "./components/base/Toast.vue";
import type { InitResult, ConfigErrorDetail, ScanWarning, Commitment, CommitmentProgress, MonthDimensions } from "./types";
import { UNDO_TOAST_KEY, SAVED_TOAST_KEY, FOCUS_REQUEST_KEY } from "./types";
import { logError, logInfo } from "./utils/errorLog";
import { applyInitResult } from "./utils/applyInitResult";

const store = useStore();

// Periodic midnight check. Default on in production; tests pass 0 to opt out so
// a recurring timer never traps their vi.runAllTimersAsync() flush loops.
const props = withDefaults(defineProps<{ rolloverIntervalMs?: number }>(), {
  rolloverIntervalMs: 60000,
});

const showUndoToast = ref(false);
const undoAction = ref<(() => void) | null>(null);
let undoTimer: ReturnType<typeof setTimeout> | null = null;

const showScanWarning = ref(false);
const scanWarnings = ref<ScanWarning[]>([]);

// #1: window focus → auto-focus input
const focusRequestId = ref(0);

let lastKnownToday = formatDate(new Date());
let rolloverTimer: ReturnType<typeof setInterval> | null = null;
let watcherHealthTimer: ReturnType<typeof setInterval> | null = null;

// Advance the view if the calendar day changed while we were following "today".
// Shared by the focus handler and the periodic timer so both behave identically.
function maybeRollover() {
  const { rollover, date } = rolloverDecision(
    store.currentDate,
    lastKnownToday,
    formatDate(new Date()),
    store.status === "ready",
  );
  lastKnownToday = formatDate(new Date());
  if (rollover) {
    store.currentDate = date;
    initApp();
  }
}

// Store listener handles for cleanup (prevents HMR duplication)
let unlistenDimensions: (() => void) | null = null;
let unlistenCommitments: (() => void) | null = null;
let unlistenFocus: (() => void) | null = null;

onMounted(async () => {
  try {
    unlistenDimensions = await listen<ConfigErrorDetail[]>("dimensions-changed", async (event) => {
      if (event.payload.length > 0) {
        store.configErrors = event.payload;
        store.configCategory = "in_place";
        // Keep status 'ready' so MonthView stays alive with ConfigErrorBanner.
        // Only root_missing or ConfigError from init should trigger RecoveryScreen.
        if (store.status === "ready") return;
        // If we were still loading or setting up, show the error in RecoveryScreen
        // so the user isn't stuck on a blank/loading screen.
        store.status = "error";
        return;
      }
      // No errors — if previously in error state, re-init; otherwise reload dims.
      if (store.status === "error") {
        await initApp();
        return;
      }
      if (store.status !== "ready") return;
      // Reload dimensions for the currently viewed month only — not initApp()
      const { year, month } = yearMonthFromDate(store.currentDate);
      try {
        const result = await invoke("get_month_dimensions", {
          rootPath: store.rootPath,
          year,
          month,
        }) as MonthDimensions;
        if (yearMonthFromDate(store.currentDate).month !== month) return;
        store.dimensions = result.dimensions;
        store.usingDefaultDimensions = result.usingDefaultDimensions;
      } catch (e) {
        logError("App.dimensionsChanged", e);
      }
    });

    // Re-read the SELECTED month (not the launch month): initApp would reload the
    // real current month and clobber whatever month the user is viewing.
    unlistenCommitments = await listen<ConfigErrorDetail[]>("commitments-changed", async () => {
      if (store.status !== "ready") return;
      const { year, month } = yearMonthFromDate(store.currentDate);
      try {
        // Reload both commitments AND dimensions (monthly file contains both)
        const dimsResult = await invoke("get_month_dimensions", {
          rootPath: store.rootPath, year, month,
        }) as MonthDimensions;
        const commitments = await invoke("get_commitments", { rootPath: store.rootPath, year, month }) as Commitment[];
        store.commitmentProgress = await invoke<CommitmentProgress[]>("get_commitment_progress", { rootPath: store.rootPath, year, month });
        // Guard against stale writes: if the user navigated away while loading, discard.
        const cur = yearMonthFromDate(store.currentDate);
        if (cur.year !== year || cur.month !== month) return;
        store.dimensions = dimsResult.dimensions;
        store.usingDefaultDimensions = dimsResult.usingDefaultDimensions;
        store.commitments = commitments;
      } catch (e) {
        logError("App.commitmentsChanged", e);
      }
    });

    unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused) return;
      focusRequestId.value++;
      maybeRollover();
    });
  } catch (e) {
    logError("App.onMounted", e);
  }

  if (props.rolloverIntervalMs > 0) {
    rolloverTimer = setInterval(maybeRollover, props.rolloverIntervalMs);
  }

  // Watcher health check — every 60 s confirm the file-watcher receiver thread
  // is still running. Gated on rolloverIntervalMs > 0 so tests (which pass 0)
  // can opt out of the recurring timer.
  if (props.rolloverIntervalMs > 0) {
    let watcherWasAlive = true;
    watcherHealthTimer = setInterval(async () => {
      try {
        const alive = await invoke<boolean>("check_watcher_health");
        if (!alive && watcherWasAlive) {
          triggerSavedToast("File watcher stopped — restart the app to resume live updates");
        }
        watcherWasAlive = alive;
      } catch (e) { logError("App.healthCheck", e); }
    }, 60000);
  }

  initApp();
});

onUnmounted(() => {
  unlistenDimensions?.();
  unlistenCommitments?.();
  unlistenFocus?.();
  if (undoTimer) clearTimeout(undoTimer);
  if (savedToastTimer) clearTimeout(savedToastTimer);
  if (rolloverTimer) clearInterval(rolloverTimer);
  if (watcherHealthTimer) clearInterval(watcherHealthTimer);
});

async function initApp() {
  logInfo("App.initApp", "start");
  try {
    const result = (await invoke("init")) as InitResult;
    const warnings = applyInitResult(store, result);
    if (warnings.length > 0) {
      scanWarnings.value = warnings;
      showScanWarning.value = true;
    }
    logInfo("App.initApp", result.status);
  } catch (e) {
    logError("App.initApp", e);
    store.configErrors = [{ kind: "InitError", message: `Failed: ${e}` }];
    store.configCategory = "root_missing";
    store.status = "error";
  }
}

// Saved toast (brief confirmation for entry update, no undo)
const showSavedToast = ref(false);
const savedToastMessage = ref("");
let savedToastTimer: ReturnType<typeof setTimeout> | null = null;

function triggerSavedToast(message: string) {
  if (savedToastTimer) clearTimeout(savedToastTimer);
  savedToastMessage.value = message;
  showSavedToast.value = true;
  savedToastTimer = setTimeout(() => {
    showSavedToast.value = false;
  }, SAVED_TOAST_DURATION);
}

function dismissSavedToast() {
  if (savedToastTimer) clearTimeout(savedToastTimer);
  showSavedToast.value = false;
}

// Undo toast for delete
function triggerUndoToast(undoFn: () => void) {
  if (undoTimer) clearTimeout(undoTimer);
  undoAction.value = undoFn;
  showUndoToast.value = true;
  undoTimer = setTimeout(() => {
    showUndoToast.value = false;
    undoAction.value = null;
  }, UNDO_DELETE_DELAY);
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
provide(UNDO_TOAST_KEY, triggerUndoToast);
provide(SAVED_TOAST_KEY, triggerSavedToast);

provide(FOCUS_REQUEST_KEY, focusRequestId);
</script>

<template>
  <div class="min-h-screen">
    <div v-if="store.status === 'loading'" class="flex items-center justify-center min-h-screen text-gray-500">
      Loading…
    </div>
    <SetupScreen v-else-if="store.status === 'setup'" />
    <DataVersionScreen
      v-else-if="store.status === 'migration_needed'"
      :message="store.dataVersionMessage ?? 'Data format version error.'"
      :root-path="store.rootPath"
    />
    <RecoveryScreen v-else-if="store.status === 'error'" :reload="initApp" />
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

    <!-- Saved Toast -->
    <Toast
      :show="showSavedToast"
      :message="savedToastMessage"
      @dismiss="dismissSavedToast"
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
