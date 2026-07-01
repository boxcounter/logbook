<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../stores/useStore";
import { useRootFolderPicker } from "../composables/useRootFolderPicker";
import { logError } from "../utils/errorLog";
import ConfigErrorBanner from "./ConfigErrorBanner.vue";

const props = defineProps<{ reload: () => Promise<void> | void }>();

const store = useStore();
const { pick } = useRootFolderPicker(store);
const confirmingFresh = ref(false);
const recreateError = ref<string | null>(null);

async function recreate() {
  recreateError.value = null;
  try {
    await invoke("create_starter_files", { path: store.rootPath });
    await props.reload();
  } catch (e) {
    logError("RecoveryScreen.recreate", e);
    recreateError.value = `Failed to recreate: ${e}`;
  }
}

async function revealTemplate() {
  try {
    await invoke("reveal_template_file", { rootPath: store.rootPath });
  } catch (e) {
    logError("RecoveryScreen.revealTemplate", e);
  }
}
</script>

<template>
  <div class="min-h-screen p-2xl">
    <div class="mx-auto max-w-[28rem] text-center">
      <!-- Tier 1: in_place -->
      <template v-if="store.configCategory === 'in_place'">
        <ConfigErrorBanner />
        <button
          data-testid="reveal-config"
          class="mt-lg px-lg py-sm rounded-[var(--radius-form-lg)] bg-[var(--color-brand-solid)] text-white text-secondary whitespace-nowrap cursor-pointer hover:shadow-[var(--shadow-button-hover)] transition-all duration-[var(--motion-base)]"
          @click="revealTemplate"
        >
          Reveal dimensions.template.yaml in Finder
        </button>
      </template>

      <!-- Tier 2: config_missing -->
      <template v-else-if="store.configCategory === 'config_missing'">
        <h1 class="text-title font-bold mb-md text-[var(--color-text-primary)]">Your dimension template is missing</h1>
        <p class="text-[var(--color-text-secondary)] mb-sm">Your data folder is here, but its dimensions.template.yaml file is gone:</p>
        <code class="inline-block max-w-full break-all text-secondary font-mono bg-[var(--color-danger)]/10 px-sm py-xs rounded-[var(--radius-sm)] mb-md">{{ store.rootPath }}</code>
        <p class="text-[var(--color-text-secondary)] mb-xl">Your records are still in place. Recreate a default template to continue.</p>
        <div class="flex flex-wrap justify-center gap-md">
          <button
            data-testid="recreate-config"
            class="px-lg py-sm rounded-[var(--radius-form-lg)] bg-[var(--color-brand-solid)] text-white text-secondary whitespace-nowrap cursor-pointer hover:shadow-[var(--shadow-button-hover)] transition-all duration-[var(--motion-base)]"
            @click="recreate"
          >
            Recreate default template
          </button>
          <button
            data-testid="choose-folder"
            class="px-lg py-sm rounded-[var(--radius-form-lg)] border border-[var(--color-border-form)] text-secondary text-[var(--color-text-primary)] whitespace-nowrap cursor-pointer"
            @click="pick"
          >
            Choose a different folder…
          </button>
        </div>
        <p v-if="recreateError" data-testid="recreate-error" class="text-secondary text-[var(--color-danger)] mt-sm">{{ recreateError }}</p>
      </template>

      <!-- Tier 3: root_missing (and fallback) -->
      <template v-else>
        <h1 class="text-title font-bold mb-md text-[var(--color-text-primary)]">Can't find your Logbook folder</h1>
        <p class="text-[var(--color-text-secondary)] mb-sm">Logbook expects your data here, but it isn't available:</p>
        <code class="inline-block max-w-full break-all text-secondary font-mono bg-[var(--color-danger)]/10 px-sm py-xs rounded-[var(--radius-sm)] mb-md">{{ store.rootPath }}</code>
        <p class="text-[var(--color-text-secondary)] mb-xl">
          This can happen if iCloud hasn't finished syncing, the drive isn't mounted, or the folder was
          moved or deleted. Logbook won't create files here automatically, to avoid conflicting with data
          that may still be syncing.
        </p>
        <div class="flex flex-wrap justify-center gap-md mb-lg">
          <button
            data-testid="retry"
            class="px-lg py-sm rounded-[var(--radius-form-lg)] bg-[var(--color-brand-solid)] text-white text-secondary whitespace-nowrap cursor-pointer hover:shadow-[var(--shadow-button-hover)] transition-all duration-[var(--motion-base)]"
            @click="props.reload"
          >
            Retry
          </button>
          <button
            data-testid="choose-folder"
            class="px-lg py-sm rounded-[var(--radius-form-lg)] border border-[var(--color-border-form)] text-secondary text-[var(--color-text-primary)] whitespace-nowrap cursor-pointer"
            @click="pick"
          >
            Choose a different folder…
          </button>
        </div>

        <div class="text-secondary text-[var(--color-text-secondary)]">
          <button
            v-if="!confirmingFresh"
            data-testid="start-fresh"
            class="underline cursor-pointer"
            @click="confirmingFresh = true"
          >
            Folder was deleted on purpose? Start fresh here
          </button>
          <div v-else>
            <p class="mb-sm">This creates a brand-new empty Logbook at the path above. If your data is only out of sync, do NOT do this.</p>
            <div class="flex flex-wrap justify-center gap-md">
              <button
                data-testid="start-fresh-confirm"
                class="px-lg py-sm rounded-[var(--radius-form-lg)] bg-[var(--color-danger)] text-white text-secondary whitespace-nowrap cursor-pointer"
                @click="recreate"
              >
                Yes, create a fresh Logbook
              </button>
              <button
                class="px-lg py-sm rounded-[var(--radius-form-lg)] border border-[var(--color-border-form)] text-secondary text-[var(--color-text-primary)] whitespace-nowrap cursor-pointer"
                @click="confirmingFresh = false"
              >
                Cancel
              </button>
            </div>
            <p v-if="recreateError" data-testid="recreate-error" class="text-secondary text-[var(--color-danger)] mt-sm">{{ recreateError }}</p>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>
