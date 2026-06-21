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
      store.status = "ready";
    } else if (result.status === "ConfigError") {
      store.rootPath = path;
      store.configErrors = result.data.errors;
      store.status = "error";
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
          store.status = "error";
        }
      }
    } else {
      logError("SetupScreen.selectFolder", e);
      store.configErrors = [{ kind: "SetupError", message: `Failed: ${e}` }];
      store.status = "error";
    }
  }
}
</script>

<template>
  <div class="flex flex-col items-center justify-center min-h-screen p-8">
    <h1 class="text-2xl font-bold mb-4 text-[var(--color-text-primary)]">Welcome to Logbook</h1>
    <p class="text-[var(--color-text-secondary)] mb-6 text-center max-w-md">
      Logbook stores work records as Markdown files with YAML frontmatter.
      Choose a folder to store your data.
    </p>
    <button
      class="px-[24px] py-[12px] bg-gradient-to-br from-[var(--color-brand-gradient-from)] to-[var(--color-brand-gradient-to)] text-white rounded-full hover:-translate-y-px hover:shadow-[var(--shadow-button-hover)] transition-all duration-200 text-[16px] font-semibold cursor-pointer shadow-[var(--shadow-button)]"
      @click="selectFolder"
    >
      Choose Data Folder
    </button>
  </div>
</template>
