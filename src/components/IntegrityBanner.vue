<script setup lang="ts">
import { useStore } from "../stores/useStore";
const store = useStore();
</script>

<template>
  <div
    v-if="store.integrityIssues.length > 0"
    class="bg-[var(--color-danger)]/5 border border-[var(--color-danger)]/20 rounded-[var(--radius-form-lg)] p-lg mx-lg mt-lg text-left"
  >
    <h2 class="text-[var(--color-danger)] font-semibold mb-sm">
      Data Protection Mode Active — entry and editing suspended
    </h2>
    <p class="text-[var(--color-danger)] text-secondary mb-md">
      File issues detected. Fix the files below with a text editor, then press ⌘R to reload.
      Or restore from backup and restart the app.
    </p>
    <div class="flex flex-col gap-lg">
      <div v-for="(issue, i) in store.integrityIssues" :key="i">
        <div class="text-[var(--color-danger)] font-semibold text-secondary mb-xs">
          {{ issue.path }}
        </div>
        <div class="text-[var(--color-danger)] text-secondary whitespace-pre-wrap">
          {{ issue.message }}
        </div>
      </div>
    </div>
  </div>
</template>
