<!-- src/components/QuickJumpPopover.vue -->
<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
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

const rootEl = ref<HTMLDivElement>();

onMounted(() => rootEl.value?.focus());

const selectedYear = ref(props.year);
const selectedMonth = ref(props.month);

watch(selectedYear, (newYear) => {
  const available = props.availableMonths.filter(m => m.year === newYear).map(m => m.month);
  if (available.length > 0 && !available.includes(selectedMonth.value)) {
    selectedMonth.value = available[available.length - 1];
  }
});

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
    ref="rootEl"
    tabindex="-1"
    class="flex gap-sm items-center bg-[var(--color-surface)] border border-[var(--color-border-form)]
           rounded-[var(--radius-form-lg)] shadow-[var(--shadow-quickjump)] px-md py-sm outline-none"
    @keydown.esc="emit('close')"
  >
    <select
      v-model.number="selectedYear"
      class="text-secondary text-[var(--color-text-primary)] bg-[var(--color-surface)]
             border border-[var(--color-border-form)] rounded-[var(--radius-form-lg)] px-sm py-xs outline-none"
    >
      <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
    </select>
    <select
      v-model.number="selectedMonth"
      class="text-secondary text-[var(--color-text-primary)] bg-[var(--color-surface)]
             border border-[var(--color-border-form)] rounded-[var(--radius-form-lg)] px-sm py-xs outline-none"
      @change="onMonthChange(selectedMonth)"
    >
      <option
        v-for="m in monthsForYear" :key="m" :value="m"
      >{{ MONTH_NAMES[m - 1] }}</option>
    </select>
    <span class="text-micro text-[var(--color-text-secondary)] whitespace-nowrap">Go</span>
  </div>
</template>
