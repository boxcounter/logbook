<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../stores/useStore";
const store = useStore();

const referencedFiles = computed(() => {
  const files = new Set<string>();
  for (const err of store.configErrors) {
    const m = err.message.match(/^([\w/.-]+\.yaml):/);
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
    <div class="flex flex-col gap-lg">
      <div v-for="(err, i) in store.configErrors" :key="i">
        <div class="text-[var(--color-danger)] font-semibold text-secondary mb-xs">{{ err.kind }}</div>
        <div class="text-[var(--color-danger)] text-secondary whitespace-pre-wrap">{{ err.message }}</div>
      </div>
    </div>
  </div>
</template>
