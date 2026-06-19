<script setup lang="ts">
defineProps<{
  color?: 'category' | 'biz' | 'importance' | 'goal' | 'missing';
  label?: string;
  value: string;
  closable?: boolean;
}>();

defineEmits<{
  close: [];
}>();
</script>

<template>
  <span
    class="inline-flex items-center gap-[4px] px-[10px] py-[3px]
           rounded-full text-[var(--app-text-xs)] font-medium cursor-pointer
           transition-opacity duration-200 hover:opacity-80"
    :class="
      color === 'category'
        ? 'bg-[var(--color-chip-category-bg)] border border-[var(--color-chip-category-border)] text-[var(--color-chip-category-text)]'
        : color === 'biz'
        ? 'bg-[var(--color-chip-biz-bg)] border border-[var(--color-chip-biz-border)] text-[var(--color-chip-biz-text)]'
        : color === 'importance'
        ? 'bg-[var(--color-chip-importance-bg)] border border-[var(--color-chip-importance-border)] text-[var(--color-chip-importance-text)]'
        : color === 'goal'
        ? 'bg-[var(--color-chip-goal-bg)] border border-[var(--color-chip-goal-border)] text-[var(--color-chip-goal-text)]'
        : color === 'missing'
        ? 'bg-[var(--color-chip-missing-bg)] border-2 border-dashed border-[var(--color-chip-missing-border)] text-[var(--color-chip-missing-text)]'
        : 'bg-[var(--color-brand-soft-bg)] border border-[var(--color-chip-category-border)] text-[var(--color-brand-solid)]'
    "
    @click="closable && $emit('close')"
  >
    {{ label ? label + ': ' : '' }}{{ value }}
    <span v-if="closable" class="opacity-40 hover:opacity-80 text-[14px] leading-none">&times;</span>
  </span>
</template>
