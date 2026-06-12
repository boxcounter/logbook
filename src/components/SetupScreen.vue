<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useStore } from "../stores/useStore";
import type { InitResult } from "../types";
import { logError } from "../utils/errorLog";

const store = useStore();

async function selectFolder() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select Logbook data folder",
  });
  if (!selected) return;

  const path = selected as string;
  await trySetRootPath(path);
}

async function trySetRootPath(path: string) {
  try {
    const result = (await invoke("set_root_path", { path })) as InitResult;
    if (result.status === "Ready") {
      store.rootPath = path;
      store.config = result.data.config;
      store.today = result.data.today;
      store.commitments = result.data.commitments;
      store.screen = "ready";
    } else if (result.status === "ConfigError") {
      store.rootPath = path;
      store.configErrors = result.data;
      store.screen = "error";
    }
  } catch (e) {
    const msg = String(e);
    if (msg.includes("Failed to read") || msg.includes("No such file")) {
      const shouldCreate = confirm("No config.yaml found. Create one with default settings?");
      if (shouldCreate) {
        try {
          await invoke("create_starter_files", { path });
          await trySetRootPath(path);
        } catch (e2) {
          logError("SetupScreen.selectFolder", e2);
          store.configErrors = [{ kind: "SetupError", message: `Failed: ${e2}` }];
          store.screen = "error";
        }
      }
    } else {
      logError("SetupScreen.selectFolder", e);
      store.configErrors = [{ kind: "SetupError", message: `Failed: ${e}` }];
      store.screen = "error";
    }
  }
}
</script>

<template>
  <div class="flex flex-col items-center justify-center min-h-screen p-8">
    <h1 class="text-2xl font-bold mb-4">Welcome to Logbook</h1>
    <p class="text-gray-600 mb-6 text-center max-w-md">
      Logbook stores work records as Markdown files with YAML frontmatter.
      Choose a folder to store your data.
    </p>
    <button
      class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
      @click="selectFolder"
    >
      Choose Data Folder
    </button>
  </div>
</template>
