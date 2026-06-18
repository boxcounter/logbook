<script setup lang="ts">
import { ListboxContent, ListboxItem, ListboxItemIndicator, ListboxRoot } from 'reka-ui';

defineProps<{
  options: { value: string; label: string }[];
  modelValue?: string;
  placeholder?: string;
}>();

defineEmits<{
  'update:modelValue': [value: string];
}>();
</script>

<template>
  <ListboxRoot
    :model-value="modelValue"
    @update:model-value="(val: any) => $emit('update:modelValue', val as string)"
  >
    <ListboxContent
      class="min-w-[140px] bg-[var(--color-surface)] border border-[var(--color-border-decorative)]
             rounded-[var(--radius-popover)] shadow-[var(--shadow-popover)]
             overflow-hidden text-[var(--text-base)]
             animate-[popoverIn_0.2s_cubic-bezier(0.16,1,0.3,1)]"
    >
      <ListboxItem
        v-for="opt in options"
        :key="opt.value"
        :value="opt.value"
        class="px-[14px] py-[10px] cursor-pointer transition-colors duration-100
               hover:bg-[var(--color-divider)]
               data-[state=checked]:bg-[var(--color-brand-soft-bg)]
               data-[state=checked]:text-[var(--color-brand-link)]
               data-[state=checked]:font-medium"
      >
        {{ opt.label }}
        <ListboxItemIndicator class="ml-auto text-[var(--color-success)] text-xs">
          &#10003;
        </ListboxItemIndicator>
      </ListboxItem>
    </ListboxContent>
  </ListboxRoot>
</template>
