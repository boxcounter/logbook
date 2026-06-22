<script setup lang="ts">
import { useStore } from "../stores/useStore";
import { useRootFolderPicker } from "../composables/useRootFolderPicker";

const store = useStore();
const { pick } = useRootFolderPicker(store);
</script>

<template>
  <div class="flex flex-col items-center justify-center min-h-screen p-2xl">
    <h1 class="text-title font-bold mb-lg text-[var(--color-text-primary)]">Welcome to Logbook</h1>
    <p class="text-[var(--color-text-secondary)] mb-xl text-center max-w-md">
      Logbook stores work records as Markdown files with YAML frontmatter.
      Choose a folder to store your data.
    </p>
    <button
      class="px-xl py-md bg-gradient-to-br from-[var(--color-brand-gradient-from)] to-[var(--color-brand-gradient-to)] text-white rounded-full hover:-translate-y-px hover:shadow-[var(--shadow-button-hover)] transition-all duration-[var(--motion-base)] text-body font-semibold cursor-pointer shadow-[var(--shadow-button)]"
      @click="pick"
    >
      Choose Data Folder
    </button>
  </div>
</template>
