<script setup lang="ts">
import { ref } from 'vue';

defineProps<{
  modelValue?: string;
  placeholder?: string;
}>();

defineEmits<{
  'update:modelValue': [value: string];
}>();

const inputEl = ref<HTMLInputElement | null>(null);
defineExpose({ inputEl });
</script>

<template>
  <input
    ref="inputEl"
    type="text"
    :value="modelValue"
    :placeholder="placeholder"
    class="w-full px-[16px] py-[10px] text-[var(--text-base)] leading-relaxed
           bg-[var(--color-surface)] text-[var(--color-text-primary)]
           border-2 border-[var(--color-border-form)] rounded-full
           outline-none
           transition-all duration-200
           placeholder:text-[var(--color-placeholder)]
           focus:border-[var(--color-brand-solid)]
           focus:bg-[#fafaff]
           focus:shadow-[var(--shadow-focus-ring)]"
    @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>
