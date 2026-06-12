<script setup lang="ts">
import { onMounted, onUnmounted, ref, provide } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useStore } from "./stores/useStore";
import SetupScreen from "./components/SetupScreen.vue";
import ConfigErrorBanner from "./components/ConfigErrorBanner.vue";
import TodayView from "./components/TodayView.vue";
import type { InitResult, ConfigErrorDetail } from "./types";
import { logError, logInfo } from "./utils/errorLog";

const store = useStore();
const showUndoToast = ref(false);
const undoAction = ref<(() => void) | null>(null);
let undoTimer: ReturnType<typeof setTimeout> | null = null;

// #1: window focus → auto-focus input
const focusRequestId = ref(0);

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
        store.screen = "error";
      }
    });

    unlistenCommitments = await listen<ConfigErrorDetail[]>("commitments-changed", () => {
      initApp();
    });

    unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) focusRequestId.value++;
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
        store.screen = "setup";
        break;
      case "ConfigError":
        store.configErrors = result.data;
        store.screen = "error";
        break;
      case "Ready":
        store.rootPath = result.data.root_path;
        store.config = result.data.config;
        store.today = result.data.today;
        store.commitments = result.data.commitments;
        store.screen = "ready";
        break;
    }
    logInfo("App.initApp", result.status);
  } catch (e) {
    logError("App.initApp", e);
    store.configErrors = [{ kind: "InitError", message: `Failed: ${e}` }];
    store.screen = "error";
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

// Provide undo trigger to descendants
provide("triggerUndoToast", triggerUndoToast);
// #1: window focus → auto-focus input
provide("focusRequestId", focusRequestId);
</script>

<template>
  <div class="min-h-screen">
    <div v-if="store.screen === 'loading'" class="flex items-center justify-center min-h-screen text-gray-500">
      Loading…
    </div>
    <SetupScreen v-else-if="store.screen === 'setup'" />
    <template v-else-if="store.screen === 'error'">
      <ConfigErrorBanner />
      <button
        class="mx-4 mt-4 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm"
        @click="initApp"
      >
        Retry
      </button>
    </template>
    <TodayView v-else-if="store.screen === 'ready'" />

    <!-- Undo Toast -->
    <Teleport to="body">
      <transition name="toast">
        <div
          v-if="showUndoToast"
          class="fixed bottom-6 left-1/2 -translate-x-1/2 flex items-center gap-4 bg-gray-900 text-white px-5 py-3 rounded-lg shadow-lg z-50 text-sm"
        >
          <span>Entry deleted</span>
          <button class="text-blue-400 font-medium hover:text-blue-300" @click="handleUndo">Undo</button>
          <button class="text-gray-500 hover:text-gray-300 text-base leading-none" @click="dismissUndo">×</button>
        </div>
      </transition>
    </Teleport>
  </div>
</template>

<style>
.toast-enter-active { transition: all 0.2s ease-out; }
.toast-leave-active { transition: all 0.2s ease-in; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translate(-50%, 1rem); }
</style>
