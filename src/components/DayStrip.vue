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
    class="flex overflow-x-auto border border-gray-200 rounded-lg bg-white py-1.5 px-1"
  >
    <button
      v-for="(dateStr, idx) in dates"
      :key="dateStr"
      :data-day="dateStr"
      class="flex-shrink-0 w-9 h-11 flex flex-col items-center justify-center rounded text-xs transition-colors"
      :class="[
        dateStr === selectedDate
          ? 'bg-blue-600 text-white font-semibold'
          : isFuture(dateStr)
            ? 'text-gray-300 cursor-default'
            : isToday(dateStr)
              ? 'text-gray-700 font-semibold hover:bg-gray-100 cursor-pointer'
              : 'text-gray-600 hover:bg-gray-100 cursor-pointer',
        (idx + 1) % 7 === 0 ? 'mr-2' : '',
      ]"
      @click="handleClick(dateStr)"
    >
      <span>{{ dayNumber(dateStr) }}</span>
      <span
        v-if="hasEntries(dateStr)"
        data-dot
        class="inline-block w-1.5 h-1.5 rounded-full mt-0.5"
        :class="dateStr === selectedDate ? 'bg-white' : 'bg-blue-500'"
      ></span>
    </button>
  </div>
</template>
