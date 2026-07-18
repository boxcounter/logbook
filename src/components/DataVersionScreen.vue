<script setup lang="ts">
import { useStore } from "../stores/useStore";
import { useRootFolderPicker } from "../composables/useRootFolderPicker";

defineProps<{
  message: string;
  rootPath: string;
}>();

const store = useStore();
// Same escape hatch as RecoveryScreen: pick a different data folder and
// re-init via set_root_path. applyInitResult already handles every result
// variant (Ready / ConfigError / DataVersionNotFound / DataVersionMismatch),
// so picking another outdated folder simply lands back on this screen with a
// refreshed message.
const { pick } = useRootFolderPicker(store);
</script>

<template>
  <div class="flex flex-col items-center justify-center min-h-screen gap-lg p-lg">
    <div class="max-w-[28rem] text-center text-secondary">
      <p class="mb-md">{{ message }}</p>
      <p class="text-micro text-[var(--color-text-muted)]">
        数据目录: {{ rootPath }}
      </p>
    </div>
    <button
      data-testid="choose-folder"
      class="px-lg py-sm rounded-[var(--radius-form-lg)] border border-[var(--color-border-form)] text-secondary text-[var(--color-text-primary)] whitespace-nowrap cursor-pointer"
      @click="pick"
    >
      Choose a different folder…
    </button>
  </div>
</template>
