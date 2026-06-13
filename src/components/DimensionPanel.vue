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
          'bg-blue-50 border-blue-200 text-blue-800': chip.dim.source === 'monthly',
          'bg-green-50 border-green-200 text-green-800': chip.dim.key === 'category',
          'bg-amber-50 border-amber-200 text-amber-800': chip.dim.key === 'business-line' || chip.dim.source === 'static',
        }"
      >
        {{ chip.value }}
      </span>
    </div>
    <!-- Selects (toggleable) -->
    <div class="flex flex-col gap-2">
      <div v-for="dim in effectiveDimensions" :key="dim.key" class="flex items-center gap-2">
        <label class="text-xs text-gray-500 w-16 shrink-0">
          {{ dim.name }}<span v-if="dim.required" class="text-red-500"> *</span>
        </label>
        <select
          :value="values[dim.key] || ''"
          class="flex-1 px-2 py-1 text-xs border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
          @change="setValue(dim.key, ($event.target as HTMLSelectElement).value)"
        >
          <option value="">--</option>
          <option v-for="v in dim.values" :key="v" :value="v">{{ v }}</option>
        </select>
      </div>
      <div v-if="monthlyDimension" class="flex items-center gap-2">
        <label class="text-xs text-gray-500 w-16 shrink-0">
          {{ monthlyDimension.name }}<span v-if="monthlyDimension.required" class="text-red-500"> *</span>
        </label>
        <select
          :value="values[monthlyDimension.key] || ''"
          class="flex-1 px-2 py-1 text-xs border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
          @change="setValue(monthlyDimension.key, ($event.target as HTMLSelectElement).value)"
        >
          <option value="">--</option>
          <option v-for="g in goalOptions" :key="g" :value="g">{{ g }}</option>
        </select>
      </div>
      <div class="text-[10px] text-gray-400 mt-1">
        <span class="text-red-500">*</span> required
      </div>
    </div>
  </div>
</template>
