<script setup lang="ts">
import { formatDurationCompact } from "../../utils/format";
import type { GoalRowModel } from "../../types";

defineProps<{
  goal: GoalRowModel;
  logged: number;
  invalid: boolean;
}>();

defineEmits<{ remove: []; enter: [] }>();
</script>

<template>
  <div class="flex items-center gap-sm">
    <span data-test="drag-grip-goal" class="drag-grip-goal cursor-grab text-[var(--color-text-disabled)] select-none px-2xs">⠿</span>
    <input
      v-model="goal.name" data-test="goal-name" placeholder="Goal name"
      @keydown.enter.exact.prevent="$emit('enter')"
      class="flex-1 px-sm py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)]
             text-secondary text-[var(--color-text-secondary)]
             bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
      :class="invalid ? 'border-[var(--color-danger)]' : ''"
    />
    <span
      data-test="goal-logged"
      class="text-secondary mono whitespace-nowrap min-w-[46px] text-right"
      :class="logged > 0 ? 'text-[var(--color-text-muted)]' : 'text-[var(--color-text-disabled)]'"
    >{{ logged > 0 ? formatDurationCompact(logged) : "0" }}</span>
    <button
      data-test="goal-remove" :disabled="logged > 0"
      :title="logged > 0 ? `${formatDurationCompact(logged)} logged — rename instead` : 'Remove goal'"
      class="text-body cursor-pointer px-xs transition-[color] duration-[var(--motion-fast)]
             text-[var(--color-text-disabled)] hover:text-[var(--color-danger)]
             disabled:text-[var(--color-divider)] disabled:cursor-not-allowed disabled:hover:text-[var(--color-divider)]"
      @click="$emit('remove')"
    >&times;</button>
  </div>
</template>
