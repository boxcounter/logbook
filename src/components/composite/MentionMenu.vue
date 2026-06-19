<script setup lang="ts">
import { ref, computed } from 'vue';
import type { Dimension, Commitment } from '../../types';

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
  dimValues: Record<string, string>;
}>();

const emit = defineEmits<{
  select: [dimKey: string, value: string];
  close: [];
}>();

const phase = ref<'dim' | 'val'>('dim');
const activeDimKey = ref<string | null>(null);

const goalOptions = computed(() => {
  const goals = new Set<string>();
  for (const c of props.commitments) for (const g of c.goals) goals.add(g);
  return [...goals];
});

function selectDim(dimKey: string) {
  activeDimKey.value = dimKey;
  phase.value = 'val';
}

function selectVal(value: string) {
  if (activeDimKey.value) {
    emit('select', activeDimKey.value, value);
    const allFilled = props.dimensions
      .filter(d => d.required)
      .every(d => props.dimValues[d.key]);
    if (allFilled) {
      emit('close');
    } else {
      phase.value = 'dim';
      activeDimKey.value = null;
    }
  }
}

function goBack() { phase.value = 'dim'; activeDimKey.value = null; }
</script>

<template>
  <div
    class="bg-[var(--color-surface)] border border-[var(--color-border-decorative)]
           rounded-[var(--radius-popover)] shadow-[var(--shadow-popover)] overflow-hidden
           w-[240px] text-[var(--app-text-base)]
           animate-[popoverIn_0.2s_cubic-bezier(0.16,1,0.3,1)]"
  >
    <!-- Dim phase header -->
    <div
      v-if="phase === 'dim'"
      class="px-[14px] py-[8px] text-[10px] text-[var(--color-text-secondary)]
             uppercase tracking-wider font-bold border-b border-[var(--color-divider)]
             flex items-center gap-[8px]"
    >
      <span class="bg-[var(--color-brand-soft-bg)] text-[var(--color-brand-link)]
                   px-[6px] py-[2px] rounded-[4px] text-[9px] font-bold">
        DIM
      </span>
      Pick a dimension
    </div>

    <!-- Val phase header -->
    <div
      v-else
      class="px-[14px] py-[8px] text-[12px] font-medium
             border-b border-[var(--color-divider)]
             flex items-center gap-[8px]
             bg-[#faf5ff] text-[#7c3aed]"
    >
      <button class="font-bold hover:text-[#5b21b6] cursor-pointer leading-none" @click="goBack">
        &larr;
      </button>
      Pick a value for
      <b class="text-[#5b21b6]">
        {{ props.dimensions.find(d => d.key === activeDimKey)?.name || '' }}
      </b>
    </div>

    <!-- Dim phase items -->
    <template v-if="phase === 'dim'">
      <div
        v-for="d in dimensions" :key="d.key"
        class="px-[14px] py-[10px] text-[14px] flex items-center gap-[8px]
               cursor-pointer transition-colors duration-100
               hover:bg-[var(--color-divider)]"
        @click="selectDim(d.key)"
      >
        <span class="w-[4px] h-[24px] rounded-[2px] flex-shrink-0"
              :style="{ background: 'var(--color-brand-solid)' }">
        </span>
        {{ d.name }}
        <span class="ml-auto text-[11px]" :class="dimValues[d.key] ? 'text-[var(--color-success)]' : 'text-[var(--color-text-secondary)]'">
          {{ dimValues[d.key] ? dimValues[d.key] + ' ✓' : d.required ? 'Required' : '' }}
        </span>
      </div>
      <!-- Footer: dot progress -->
      <div
        class="px-[14px] py-[8px] text-[11px] border-t border-[var(--color-divider)]
               flex items-center gap-[6px] text-[var(--color-text-secondary)]"
      >
        <span
          v-for="d in dimensions.filter(d => d.required)" :key="d.key"
          class="w-[6px] h-[6px] rounded-full"
          :class="dimValues[d.key] ? 'bg-[var(--color-success)]' : 'bg-[var(--color-divider)]'"
        />
        {{ dimensions.filter(d => d.required && !dimValues[d.key]).length }} to go
      </div>
    </template>

    <!-- Val phase items -->
    <template v-else>
      <div
        v-for="v in (activeDimKey && dimensions.find(d => d.key === activeDimKey)?.source === 'monthly'
          ? goalOptions
          : (activeDimKey ? dimensions.find(d => d.key === activeDimKey)?.values || [] : []))"
        :key="v"
        class="px-[14px] py-[10px] text-[14px] cursor-pointer transition-colors duration-100
               hover:bg-[var(--color-divider)]"
        @click="selectVal(v)"
      >
        {{ v }}
      </div>
      <div
        class="px-[14px] py-[8px] text-[11px] border-t border-[var(--color-divider)]
               text-[var(--color-text-secondary)]"
      >
        &larr; Back &middot; Type to filter
      </div>
    </template>
  </div>
</template>

<style scoped>
@keyframes popoverIn {
  from { opacity: 0; transform: translateY(-4px) scale(0.97); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
</style>
