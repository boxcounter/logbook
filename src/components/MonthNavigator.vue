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
  <div class="relative bg-[var(--color-surface)] rounded-[var(--radius-card)] shadow-[var(--shadow-card)] p-4 text-center">
    <div class="flex items-center justify-center gap-[16px]">
      <button
        class="w-[34px] h-[34px] rounded-full border border-[var(--color-border-decorative)] bg-[var(--color-surface)] text-[var(--color-text-secondary)] text-[16px] flex items-center justify-center transition-all duration-150 hover:bg-[var(--color-divider)]"
        @click="shiftMonth(-1)"
      >&larr;</button>
      <span
        class="text-[16px] font-bold cursor-pointer hover:opacity-80 transition-opacity select-none"
        style="background: linear-gradient(135deg, var(--color-brand-gradient-from), var(--color-brand-gradient-to)); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text;"
        @click="handleLabelClick(); openPopover()"
      >
        {{ monthLabel(month) }} {{ year }}
        <span v-if="availableMonths !== null" class="text-[var(--text-xs)] text-[var(--color-text-secondary)]" style="-webkit-text-fill-color: var(--color-text-secondary);">&#9662;</span>
      </span>
      <button
        class="w-[34px] h-[34px] rounded-full border border-[var(--color-border-decorative)] bg-[var(--color-surface)] text-[var(--color-text-secondary)] text-[16px] flex items-center justify-center transition-all duration-150 hover:bg-[var(--color-divider)]"
        @click="shiftMonth(1)"
      >&rarr;</button>
    </div>

    <!-- Quick-jump popover -->
    <div
      v-if="showPopover && availableMonths !== null"
      class="absolute top-full left-1/2 -translate-x-1/2 mt-[4px] bg-[var(--color-surface)] border border-[var(--color-border-decorative)] rounded-[var(--radius-popover)] shadow-[var(--shadow-popover)] p-3 z-10 flex gap-2"
    >
      <select
        v-model="selectedYear"
        class="text-[var(--text-sm)] border-2 border-[var(--color-border-form)] rounded-[var(--radius-form)] px-2 py-1 bg-[var(--color-surface)] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-brand-solid)] focus:shadow-[var(--shadow-focus-ring)]"
      >
        <option v-for="y in availableYears" :key="y" :value="y">{{ y }}</option>
      </select>
      <select
        class="text-[var(--text-sm)] border-2 border-[var(--color-border-form)] rounded-[var(--radius-form)] px-2 py-1 bg-[var(--color-surface)] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-brand-solid)] focus:shadow-[var(--shadow-focus-ring)]"
        @change="onMonthChange(parseInt(($event.target as HTMLSelectElement).value, 10))"
      >
        <option v-for="m in monthsForYear" :key="m" :value="m" :selected="m === month && selectedYear === year">
          {{ monthLabel(m) }}
        </option>
      </select>
    </div>
  </div>
</template>
