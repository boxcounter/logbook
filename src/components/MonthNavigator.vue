<script setup lang="ts">
import { ref, computed } from "vue";
import type { AvailableMonth } from "../stores/useStore";

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

const props = defineProps<{
  year: number;
  month: number; // 1-based
  availableMonths: AvailableMonth[] | null; // null = not yet loaded
}>();

const emit = defineEmits<{
  navigate: [{ year: number; month: number }];
  requestMonths: [];
}>();

const showPopover = ref(false);

function monthLabel(m: number): string {
  return MONTH_NAMES[m - 1];
}

function shiftMonth(delta: number) {
  let newMonth = props.month + delta;
  let newYear = props.year;
  if (newMonth < 1) {
    newMonth = 12;
    newYear--;
  } else if (newMonth > 12) {
    newMonth = 1;
    newYear++;
  }
  emit("navigate", { year: newYear, month: newMonth });
}

function handleLabelClick() {
  if (props.availableMonths === null) {
    emit("requestMonths");
    return;
  }
  showPopover.value = !showPopover.value;
}

// Unique years from availableMonths
const availableYears = computed(() => {
  if (!props.availableMonths) return [];
  const years = [...new Set(props.availableMonths.map(m => m.year))];
  years.sort((a, b) => b - a);
  return years;
});

// Selected year in the popover (defaults to current year)
const selectedYear = ref(props.year);

// Months for the year selected in the popover
const monthsForYear = computed(() => {
  if (!props.availableMonths) return [];
  return props.availableMonths
    .filter(m => m.year === selectedYear.value)
    .map(m => m.month)
    .sort((a, b) => a - b);
});

// Reset selectedYear when popover opens
function openPopover() {
  selectedYear.value = props.year;
}

function onMonthChange(month: number) {
  emit("navigate", { year: selectedYear.value, month });
  showPopover.value = false;
}
</script>

<template>
  <div class="relative bg-white rounded-lg border border-gray-200 p-3 text-center">
    <div class="flex items-center justify-center gap-3">
      <button
        class="text-gray-500 hover:text-gray-700 transition-colors text-base px-1"
        @click="shiftMonth(-1)"
      >←</button>
      <span
        class="text-base font-bold text-gray-800 cursor-pointer hover:text-blue-600 transition-colors select-none"
        @click="handleLabelClick(); openPopover()"
      >
        {{ monthLabel(month) }} {{ year }}
        <span v-if="availableMonths !== null" class="text-xs text-gray-400">▾</span>
      </span>
      <button
        class="text-gray-500 hover:text-gray-700 transition-colors text-base px-1"
        @click="shiftMonth(1)"
      >→</button>
    </div>

    <!-- Quick-jump popover -->
    <div
      v-if="showPopover && availableMonths !== null"
      class="absolute top-full left-1/2 -translate-x-1/2 mt-1 bg-white border border-gray-200 rounded-lg shadow-lg p-3 z-10 flex gap-2"
    >
      <select
        v-model="selectedYear"
        class="text-sm border border-gray-300 rounded px-2 py-1 focus:outline-none focus:ring-1 focus:ring-blue-500"
      >
        <option
          v-for="y in availableYears"
          :key="y"
          :value="y"
        >{{ y }}</option>
      </select>
      <select
        class="text-sm border border-gray-300 rounded px-2 py-1 focus:outline-none focus:ring-1 focus:ring-blue-500"
        @change="onMonthChange(parseInt(($event.target as HTMLSelectElement).value, 10))"
      >
        <option
          v-for="m in monthsForYear"
          :key="m"
          :value="m"
          :selected="m === month && selectedYear === year"
        >{{ monthLabel(m) }}</option>
      </select>
    </div>
  </div>
</template>
