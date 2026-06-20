<!-- src/components/QuickJumpPopover.vue -->
<script setup lang="ts">
import { ref, computed } from "vue";
import type { AvailableMonth } from "../stores/useStore";

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

const props = defineProps<{
  year: number;
  month: number;
  availableMonths: AvailableMonth[];
}>();

const emit = defineEmits<{ jump: [{ year: number; month: number }]; close: [] }>();

const selectedYear = ref(props.year);

const years = computed(() => {
  const ys = [...new Set(props.availableMonths.map(m => m.year))];
  ys.sort((a, b) => b - a);
  return ys;
});

const monthsForYear = computed(() =>
  props.availableMonths
    .filter(m => m.year === selectedYear.value)
    .map(m => m.month)
    .sort((a, b) => a - b)
);

function onMonthChange(month: number) {
  emit("jump", { year: selectedYear.value, month });
}
</script>

<template>
  <div
    class="flex gap-[8px] items-center bg-[var(--color-surface)] border border-[var(--color-border-form)]
           rounded-[var(--radius-form-lg)] shadow-[var(--shadow-quickjump)] px-[12px] py-[10px]"
    @keydown.esc="emit('close')"
  >
    <select
      v-model.number="selectedYear"
      class="text-[length:var(--app-text-xs)] text-[var(--color-text-primary)] bg-[var(--color-surface)]
             border border-[var(--color-border-form)] rounded-[var(--radius-form-lg)] px-[8px] py-[4px] outline-none"
    >
      <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
    </select>
    <select
      class="text-[length:var(--app-text-xs)] text-[var(--color-text-primary)] bg-[var(--color-surface)]
             border border-[var(--color-border-form)] rounded-[var(--radius-form-lg)] px-[8px] py-[4px] outline-none"
      @change="onMonthChange(parseInt(($event.target as HTMLSelectElement).value, 10))"
    >
      <option
        v-for="m in monthsForYear" :key="m" :value="m"
        :selected="m === month && selectedYear === year"
      >{{ MONTH_NAMES[m - 1] }}</option>
    </select>
    <span class="text-[length:var(--app-text-2xs)] text-[var(--color-text-secondary)] whitespace-nowrap">Go</span>
  </div>
</template>
