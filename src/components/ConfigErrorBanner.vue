<script setup lang="ts">
import { useStore } from "../stores/useStore";
const store = useStore();
</script>

<template>
  <div class="bg-[var(--color-danger)]/5 border border-[var(--color-danger)]/20 rounded-lg p-4 mx-4 mt-4">
    <h2 class="text-[var(--color-danger)] font-semibold mb-2">
      Configuration Errors ({{ store.configErrors.length }})
    </h2>
    <p class="text-[var(--color-danger)] text-sm mb-3">
      Fix these errors in your config.yaml or _monthly.md file.
      Changes are detected automatically.
    </p>
    <ul class="list-disc list-inside space-y-1">
      <li v-for="(err, i) in store.configErrors" :key="i" class="text-[var(--color-danger)] text-sm">
        <span class="font-mono text-xs bg-[var(--color-danger)]/10 px-1 rounded">{{ err.kind }}</span>
        {{ err.message }}
      </li>
    </ul>
  </div>
</template>
