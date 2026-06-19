<script setup lang="ts">
import { computed } from "vue";
import type { Dimension, Commitment } from "../types";

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
  values: Record<string, string>;
}>();

const emit = defineEmits<{
  "update:values": [values: Record<string, string>];
}>();

const effectiveDimensions = computed(() => props.dimensions.filter((d) => d.source !== "monthly"));
const monthlyDimension = computed(() => props.dimensions.find((d) => d.source === "monthly"));

const goalOptions = computed(() => {
  const goals = new Set<string>();
  for (const c of props.commitments) {
    for (const g of c.goals) goals.add(g);
  }
  return [...goals];
});

function setValue(key: string, value: string) {
  emit("update:values", { ...props.values, [key]: value });
}

// Chips for display
const activeChips = computed(() => {
  const chips: { dim: Dimension; value: string }[] = [];
  for (const dim of props.dimensions) {
    const val = props.values[dim.key];
    if (val) chips.push({ dim, value: val });
  }
  return chips;
});
</script>

<template>
  <div>
    <!-- Chips row -->
    <div class="flex flex-wrap gap-1.5 mb-2">
      <span
        v-for="chip in activeChips"
        :key="chip.dim.key"
        class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs cursor-pointer border"
        :class="{
          'bg-[var(--color-chip-goal-bg)] border-[var(--color-chip-goal-border)] text-[var(--color-chip-goal-text)]': chip.dim.source === 'monthly',
          'bg-[var(--color-chip-category-bg)] border-[var(--color-chip-category-border)] text-[var(--color-chip-category-text)]': chip.dim.key === 'category',
          'bg-[var(--color-chip-biz-bg)] border-[var(--color-chip-biz-border)] text-[var(--color-chip-biz-text)]': chip.dim.key === 'business-line' || chip.dim.source === 'static',
        }"
      >
        {{ chip.value }}
      </span>
    </div>
    <!-- Selects (toggleable) -->
    <div class="flex flex-col gap-2">
      <div v-for="dim in effectiveDimensions" :key="dim.key" class="flex items-center gap-2">
        <label class="text-[var(--app-text-sm)] text-[var(--color-text-secondary)] w-16 shrink-0">
          {{ dim.name }}<span v-if="dim.required" class="text-[var(--color-danger)]"> *</span>
        </label>
        <select
          :value="values[dim.key] || ''"
          class="flex-1 px-[10px] py-[6px] text-[var(--app-text-sm)] border-2 border-[var(--color-border-form)] rounded-[var(--radius-form)] bg-[var(--color-surface)] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-brand-solid)] focus:shadow-[var(--shadow-focus-ring)] transition-all duration-200"
          @change="setValue(dim.key, ($event.target as HTMLSelectElement).value)"
        >
          <option value="">--</option>
          <option v-for="v in dim.values" :key="v" :value="v">{{ v }}</option>
        </select>
      </div>
      <div v-if="monthlyDimension" class="flex items-center gap-2">
        <label class="text-[var(--app-text-sm)] text-[var(--color-text-secondary)] w-16 shrink-0">
          {{ monthlyDimension.name }}<span v-if="monthlyDimension.required" class="text-red-500"> *</span>
        </label>
        <select
          :value="values[monthlyDimension.key] || ''"
          class="flex-1 px-[10px] py-[6px] text-[var(--app-text-sm)] border-2 border-[var(--color-border-form)] rounded-[var(--radius-form)] bg-[var(--color-surface)] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-brand-solid)] focus:shadow-[var(--shadow-focus-ring)] transition-all duration-200"
          @change="setValue(monthlyDimension.key, ($event.target as HTMLSelectElement).value)"
        >
          <option value="">--</option>
          <option v-for="g in goalOptions" :key="g" :value="g">{{ g }}</option>
        </select>
      </div>
      <div class="text-[var(--app-text-micro)] text-[var(--color-text-secondary)] mt-1">
        <span class="text-[var(--color-danger)]">*</span> required
      </div>
    </div>
  </div>
</template>
