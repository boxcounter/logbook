<script setup lang="ts">
defineProps<{
  variant?: 'primary' | 'outline' | 'secondary' | 'danger';
  size?: 'sm' | 'md';
  disabled?: boolean;
}>();

defineEmits<{
  click: [e: MouseEvent];
}>();
</script>

<template>
  <button
    :disabled="disabled"
    class="inline-flex items-center justify-center font-semibold cursor-pointer
           transition-all duration-[var(--motion-base)] disabled:opacity-50 disabled:cursor-not-allowed"
    :class="[
      size === 'sm'
        ? 'text-secondary py-sm px-lg'
        : 'text-body py-sm px-xl',
      variant === 'primary' || variant === undefined
        ? 'rounded-full border-none text-white'
          + ' bg-gradient-to-br from-[var(--color-brand-gradient-from)] to-[var(--color-brand-gradient-to)]'
          + ' shadow-[var(--shadow-button)]'
          + ' hover:-translate-y-px hover:shadow-[var(--shadow-button-hover)]'
          + ' active:scale-[0.97] active:translate-y-0'
        : variant === 'outline'
        ? 'rounded-full border-2 border-[var(--color-brand-solid)] bg-transparent text-[var(--color-brand-solid)]'
          + ' hover:-translate-y-px hover:shadow-[var(--shadow-card)]'
          + ' active:scale-[0.97] active:translate-y-0'
        : variant === 'secondary'
        ? 'rounded-full border-none bg-[var(--color-divider)] text-[var(--color-text-secondary)]'
          + ' hover:-translate-y-px hover:shadow-[var(--shadow-card)]'
          + ' active:scale-[0.97] active:translate-y-0'
        : /* danger */
          'rounded-full border-none bg-red-50 text-[var(--color-danger)]'
          + ' hover:bg-red-100',
    ]"
    @click="$emit('click', $event)"
  >
    <slot />
  </button>
</template>
