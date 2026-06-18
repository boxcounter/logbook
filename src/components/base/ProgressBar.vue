<script setup lang="ts">
defineProps<{
  spent: number;
  allocation: number;
  variant?: 'default' | 'warm';
}>();
</script>

<template>
  <div class="h-[8px] bg-[var(--color-divider)] rounded-[4px] overflow-hidden">
    <div
      class="h-full rounded-[4px] transition-all duration-500"
      :style="{
        width: allocation === 0 ? '0%' : Math.min(100, Math.round((spent / allocation) * 100)) + '%',
        background: variant === 'warm'
          ? 'linear-gradient(90deg, #f59e0b, #ef4444)'
          : 'linear-gradient(90deg, var(--color-brand-gradient-from), var(--color-brand-gradient-to))',
      }"
    />
  </div>
</template>
