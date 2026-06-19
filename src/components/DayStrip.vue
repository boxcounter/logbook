<script setup lang="ts">
import { ref, onMounted, nextTick } from "vue";
import type { Entry } from "../types";

const props = defineProps<{
  dates: string[];
  selectedDate: string;
  monthEntries: Record<string, Entry[]>;
}>();

const emit = defineEmits<{
  selectDay: [date: string];
}>();

const stripRef = ref<HTMLDivElement>();

function isToday(dateStr: string): boolean {
  const now = new Date();
  const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  return dateStr === today;
}

function isFuture(dateStr: string): boolean {
  const now = new Date();
  now.setHours(0, 0, 0, 0);
  const [y, m, d] = dateStr.split("-").map(Number);
  const target = new Date(y, m - 1, d);
  target.setHours(0, 0, 0, 0);
  return target > now;
}

function hasEntries(dateStr: string): boolean {
  const entries = props.monthEntries[dateStr];
  return entries !== undefined && entries.length > 0;
}

function dayNumber(dateStr: string): number {
  return parseInt(dateStr.split("-")[2], 10);
}

function handleClick(dateStr: string) {
  if (isFuture(dateStr)) return;
  emit("selectDay", dateStr);
}

// Scroll to make selected date visible on mount
onMounted(async () => {
  await nextTick();
  if (!stripRef.value) return;
  const selected = stripRef.value.querySelector(`[data-day="${props.selectedDate}"]`);
  if (selected) {
    selected.scrollIntoView({ inline: "center", block: "nearest", behavior: "instant" });
  }
});
</script>

<template>
  <div
    ref="stripRef"
    class="flex overflow-x-auto gap-[4px] px-[10px] py-[8px] bg-[var(--color-surface)] rounded-[var(--radius-card)] shadow-[var(--shadow-card)]"
  >
    <button
      v-for="(dateStr, idx) in dates"
      :key="dateStr"
      :data-day="dateStr"
      class="flex-shrink-0 w-[38px] h-[44px] flex flex-col items-center justify-center rounded-full text-[var(--app-text-base)] transition-all duration-150"
      :class="[
        dateStr === selectedDate
          ? 'bg-gradient-to-br from-[var(--color-brand-gradient-from)] to-[var(--color-brand-gradient-to)] text-white font-bold'
          : isFuture(dateStr)
            ? 'text-[var(--color-text-secondary)] opacity-40 cursor-default'
            : isToday(dateStr)
              ? 'text-[var(--color-text-primary)] font-semibold hover:bg-[var(--color-divider)] cursor-pointer'
              : 'text-[var(--color-text-secondary)] hover:bg-[var(--color-divider)] cursor-pointer',
        (idx + 1) % 7 === 0 ? 'mr-[8px]' : '',
      ]"
      @click="handleClick(dateStr)"
    >
      <span>{{ dayNumber(dateStr) }}</span>
      <span
        v-if="hasEntries(dateStr)"
        data-dot
        class="inline-block w-[5px] h-[5px] rounded-full mt-[2px]"
        :class="dateStr === selectedDate ? 'bg-white' : 'bg-[var(--color-brand-gradient-from)]'"
      ></span>
    </button>
  </div>
</template>
