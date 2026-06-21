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
  <div class="flex justify-between items-baseline mb-xl pb-md border-b border-[var(--color-divider)]">
    <div class="flex items-center gap-sm">
      <div class="flex items-center gap-2xs">
        <button
          data-test="prev-day"
          class="leading-none text-body text-[var(--color-text-secondary)]
                 hover:text-[var(--color-text-primary)] cursor-pointer px-xs py-2xs transition-colors"
          title="Previous day (⌘[)"
          @click="emit('prev-day')"
        >←</button>
        <button
          data-test="next-day"
          class="leading-none text-body px-xs py-2xs transition-colors"
          :class="canGoNext
            ? 'text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] cursor-pointer'
            : 'text-[var(--color-text-disabled)] opacity-40 cursor-default'"
          title="Next day (⌘])"
          :aria-disabled="!canGoNext"
          @click="onNext"
        >→</button>
      </div>
      <span class="text-title font-bold text-[var(--color-text-primary)] tracking-[-0.3px]">{{ title }}</span>
      <span
        v-if="isToday"
        data-test="today-badge"
        class="ml-sm align-middle text-micro font-semibold
               text-[var(--color-brand-link)] bg-[var(--color-brand-soft-bg)] px-sm py-2xs rounded-[var(--radius-md)]"
      >Today</span>
    </div>
    <span class="text-secondary text-[var(--color-text-secondary)]">
      <span class="mono">{{ entryCount }}</span> {{ countLabel }} · <span class="mono">{{ total }}</span>
    </span>
  </div>
</template>
