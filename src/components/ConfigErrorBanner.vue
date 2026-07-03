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
      Fix these errors in your config files. Changes are detected automatically.
    </p>
    <div class="flex flex-col gap-lg">
      <div v-for="(err, i) in store.configErrors" :key="i">
        <div class="text-[var(--color-danger)] font-semibold text-secondary mb-xs">{{ err.kind }}</div>
        <div class="text-[var(--color-danger)] text-secondary whitespace-pre-wrap">{{ err.message }}</div>
      </div>
    </div>
  </div>
</template>
