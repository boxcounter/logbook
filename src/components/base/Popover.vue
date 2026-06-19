<script setup lang="ts">
import { PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from 'reka-ui';

defineProps<{
  triggerClass?: string;
}>();
</script>

<template>
  <PopoverRoot>
    <PopoverTrigger
      :class="triggerClass"
      class="inline-flex items-center gap-[4px] px-[10px] py-[5px]
             border-2 border-[var(--color-border-form)] rounded-[var(--radius-form)]
             text-[var(--app-text-base)] leading-relaxed
             bg-[var(--color-surface)] text-[var(--color-text-secondary)]
             cursor-pointer transition-all duration-200
             focus:border-[var(--color-brand-solid)]
             focus:bg-[#fafaff]
             focus:shadow-[var(--shadow-focus-ring)]"
    >
      <slot name="trigger" />
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent
        class="bg-[var(--color-surface)] border border-[var(--color-border-decorative)]
               rounded-[var(--radius-popover)] shadow-[var(--shadow-popover)] overflow-hidden
               animate-[popoverIn_0.2s_cubic-bezier(0.16,1,0.3,1)]"
      >
        <slot />
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>

<style scoped>
@keyframes popoverIn {
  from { opacity: 0; transform: translateY(-4px) scale(0.97); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
</style>
