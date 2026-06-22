<script setup lang="ts">
import { useStore } from "../stores/useStore";
const store = useStore();
</script>

<template>
  <div class="bg-[var(--color-danger)]/5 border border-[var(--color-danger)]/20 rounded-[var(--radius-form-lg)] p-lg mx-lg mt-lg">
    <h2 class="text-[var(--color-danger)] font-semibold mb-sm">
      Configuration Errors ({{ store.configErrors.length }})
    </h2>
    <p class="text-[var(--color-danger)] text-secondary mb-md">
      Fix these errors in your template.yaml or _monthly.md file.
      Changes are detected automatically.
    </p>
    <ul class="list-disc list-inside space-y-xs">
      <li v-for="(err, i) in store.configErrors" :key="i" class="text-[var(--color-danger)] text-secondary">
        <span class="font-mono text-secondary bg-[var(--color-danger)]/10 px-xs rounded">{{ err.kind }}</span>
        {{ err.message }}
      </li>
    </ul>
  </div>
</template>
