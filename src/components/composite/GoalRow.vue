<script setup lang="ts">
import { formatDuration } from "../../utils/format";

interface GoalRowModel { name: string; origName: string | null; key: number }

defineProps<{
  goal: GoalRowModel;
  logged: number;
}>();

defineEmits<{ remove: [] }>();
</script>

<template>
  <div class="flex items-center gap-[8px]">
    <span data-test="drag-grip-goal" class="drag-grip-goal cursor-grab text-[var(--color-text-disabled)] select-none px-[2px]">⠿</span>
    <input
      v-model="goal.name" data-test="goal-name" placeholder="Goal name"
      class="flex-1 px-[10px] py-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-form)]
             text-[length:var(--app-text-sm)] text-[var(--color-text-secondary)]
             bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
    />
    <span
      data-test="goal-logged"
      class="text-[length:var(--app-text-xs-alt)] mono whitespace-nowrap min-w-[46px] text-right"
      :class="logged > 0 ? 'text-[var(--color-text-muted)]' : 'text-[var(--color-text-disabled)]'"
    >{{ logged > 0 ? formatDuration(logged) : "0" }}</span>
    <button
      data-test="goal-remove"
      class="text-[length:var(--app-text-base)] text-[var(--color-text-disabled)] hover:text-[var(--color-danger)] cursor-pointer px-[4px]"
      @click="$emit('remove')"
    >&times;</button>
  </div>
</template>
