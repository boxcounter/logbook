<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../stores/useStore";
const store = useStore();

const referencedFiles = computed(() => {
  const files = new Set<string>();
  for (const err of store.configErrors) {
    const m = err.message.match(/^(\S+)\s/);
    if (m) files.add(m[1]);
  }
  return [...files];
});
</script>

<template>
  <div class="bg-[var(--color-danger)]/5 border border-[var(--color-danger)]/20 rounded-[var(--radius-form-lg)] p-lg mx-lg mt-lg text-left">
    <h2 class="text-[var(--color-danger)] font-semibold mb-sm">
      Configuration Errors ({{ store.configErrors.length }})
    </h2>
    <p class="text-[var(--color-danger)] text-secondary mb-md">
      Fix these errors in
      <template v-for="(f, i) in referencedFiles" :key="f">
        <code v-if="i > 0 && i === referencedFiles.length - 1"> or </code>
        <code v-else-if="i > 0">, </code>
        <code class="bg-[var(--color-danger)]/10 px-xs rounded">{{ f }}</code>
      </template>.
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
