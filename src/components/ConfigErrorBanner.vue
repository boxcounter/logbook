<script setup lang="ts">
import { useStore } from "../stores/useStore";
const store = useStore();
</script>

<template>
  <div class="bg-[var(--color-danger)]/5 border border-[var(--color-danger)]/20 rounded-[var(--radius-form-lg)] p-lg mx-lg mt-lg text-left">
    <h2 class="text-[var(--color-danger)] font-semibold mb-sm">
      Configuration Errors ({{ store.configErrors.length }})
    </h2>
    <p class="text-[var(--color-danger)] text-secondary mb-md">
      Fix these errors in your dimensions.template.yaml, dimensions.yaml, or commitments.yaml file.
      Changes are detected automatically.
    </p>
    <ul class="list-disc list-inside space-y-xs">
      <li v-for="(err, i) in store.configErrors" :key="i" class="text-[var(--color-danger)] text-secondary">
        <div class="font-mono text-secondary bg-[var(--color-danger)]/10 px-xs rounded inline-block mb-xs">{{ err.kind }}</div>
        <div class="whitespace-pre-wrap">{{ err.message }}</div>
      </li>
    </ul>
  </div>
</template>
