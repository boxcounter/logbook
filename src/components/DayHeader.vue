<!-- src/components/DayHeader.vue -->
<script setup lang="ts">
import { computed } from "vue";
import { formatDuration } from "../utils/format";

const props = defineProps<{
  title: string;
  isToday: boolean;
  entryCount: number;
  totalMinutes: number;
  canGoNext: boolean;
}>();

const emit = defineEmits<{
  "prev-day": [];
  "next-day": [];
}>();

const countLabel = computed(() => (props.entryCount === 1 ? "entry" : "entries"));
const total = computed(() => formatDuration(props.totalMinutes));

function onNext() {
  if (props.canGoNext) emit("next-day");
}
</script>

<template>
  <div class="flex justify-between items-baseline mb-[20px] pb-[14px] border-b border-[var(--color-divider)]">
    <div class="flex items-center gap-[8px]">
      <button
        data-test="prev-day"
        class="inline-flex items-center justify-center w-[22px] h-[22px] rounded-[var(--radius-form-lg)]
               border border-[var(--color-border-form)] text-[var(--color-text-secondary)]
               hover:text-[var(--color-text-primary)] hover:bg-[var(--color-surface-muted)] cursor-pointer transition-colors"
        title="Previous day (⌘[)"
        @click="emit('prev-day')"
      >‹</button>
      <span class="text-[length:var(--app-text-xl)] font-bold text-[var(--color-text-primary)] tracking-[-0.3px]">{{ title }}</span>
      <button
        data-test="next-day"
        class="inline-flex items-center justify-center w-[22px] h-[22px] rounded-[var(--radius-form-lg)]
               border border-[var(--color-border-form)] transition-colors"
        :class="canGoNext
          ? 'text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-surface-muted)] cursor-pointer'
          : 'text-[var(--color-text-disabled)] opacity-40 cursor-default'"
        title="Next day (⌘])"
        @click="onNext"
      >›</button>
      <span
        v-if="isToday"
        data-test="today-badge"
        class="ml-[6px] align-middle text-[length:var(--app-text-micro)] font-semibold
               text-[var(--color-brand-link)] bg-[var(--color-brand-soft-bg)] px-[8px] py-[2px] rounded-[var(--radius-md)]"
      >Today</span>
    </div>
    <span class="text-[length:var(--app-text-xs)] text-[var(--color-text-secondary)]">
      <span class="mono">{{ entryCount }}</span> {{ countLabel }} · <span class="mono">{{ total }}</span>
    </span>
  </div>
</template>
