<script setup lang="ts">
import { ref, computed } from 'vue';
import type { Entry } from '../../types';
import { formatDuration, resolveDelta } from '../../utils/format';
import { useStore } from '../../stores/useStore';
import AppChip from '../base/AppChip.vue';

const props = defineProps<{
  entry: Entry;
  index: number;
}>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
  updateDimensions: [entryId: string, dimensions: Record<string, string>];
}>();

const store = useStore();

const editingItem = ref(false);
const editingDuration = ref(false);
const editingDimensions = ref(false);

const itemInput = ref('');
const durInput = ref('');

function startEditItem() {
  itemInput.value = props.entry.item;
  editingItem.value = true;
}
function commitItem() {
  editingItem.value = false;
  const newItem = itemInput.value.trim() || '(untitled)';
  if (newItem !== props.entry.item) {
    emit('update', props.entry.id, newItem, props.entry.duration);
  }
}

function startEditDuration() {
  durInput.value = String(props.entry.duration);
  editingDuration.value = true;
}
function commitDuration() {
  editingDuration.value = false;
  const newDur = resolveDelta(durInput.value, props.entry.duration);
  if (newDur !== props.entry.duration) {
    emit('update', props.entry.id, props.entry.item, newDur);
  }
}

function chipColor(key: string): 'category' | 'biz' | 'importance' | 'goal' | 'missing' {
  const map: Record<string, 'category' | 'biz' | 'importance' | 'goal'> = {
    'category': 'category',
    'business-line': 'biz',
    'importance-urgency': 'importance',
    'goal': 'goal',
  };
  return map[key] || 'category';
}

const orderedDimensions = computed(() => store.config?.dimensions || []);

const goalOptions = computed(() => {
  const goals = new Set<string>();
  for (const c of store.commitments) {
    for (const g of c.goals) goals.add(g);
  }
  return [...goals];
});

function onDimChange(dimKey: string, value: string) {
  emit('updateDimensions', props.entry.id, { ...props.entry.dimensions, [dimKey]: value });
}
</script>

<template>
  <div
    class="group flex items-center gap-[12px] py-[10px] px-[12px] -mx-[12px] rounded-[8px]
           border-b border-[var(--color-divider)] last:border-b-0
           transition-all duration-200
           hover:bg-[var(--color-divider)] hover:translate-x-[2px]"
    :class="{ 'bg-[#fafaff] shadow-[inset_3px_0_0_var(--color-brand-solid)]': editingItem || editingDuration || editingDimensions }"
  >
    <!-- Row number -->
    <span class="text-[var(--app-text-xs)] text-[var(--color-text-secondary)] w-[20px] text-right flex-shrink-0 tabular-nums">
      {{ index + 1 }}
    </span>

    <!-- Content area: item text + chips share flex-1 space -->
    <div class="flex-1 min-w-0 flex items-center gap-[8px] flex-wrap">
      <!-- Item text -->
      <template v-if="editingItem">
        <input
          v-model="itemInput"
          class="flex-1 px-[8px] py-[3px] border-2 border-[var(--color-brand-solid)]
                 rounded-[var(--radius-form)] text-[var(--app-text-base)] leading-[1.4]
                 outline-none bg-[#fafaff] min-w-0
                 shadow-[var(--shadow-focus-ring)]"
          @keydown.enter.prevent="commitItem"
          @keydown.escape.prevent="editingItem = false"
          @blur="commitItem"
          autofocus
        />
      </template>
      <template v-else>
        <span
          class="text-[var(--app-text-base)] min-w-0 cursor-default
                 rounded px-[2px] -mx-[2px] hover:bg-[var(--color-divider)]"
          @dblclick="startEditItem"
        >
          {{ entry.item }}
        </span>
      </template>

      <!-- Dimension editing -->
      <template v-if="editingDimensions">
        <select
          v-for="dim in orderedDimensions"
          :key="dim.key"
          :value="entry.dimensions[dim.key] || ''"
          class="px-[10px] py-[3px] border-2 border-[var(--color-border-form)]
                 rounded-[var(--radius-form)] text-[var(--app-text-base)] leading-[1.4]
                 bg-[var(--color-surface)] text-[var(--color-text-secondary)]
                 focus:border-[var(--color-brand-solid)] focus:bg-[#fafaff]
                 focus:shadow-[var(--shadow-focus-ring)] outline-none
                 transition-all duration-200"
          @change="onDimChange(dim.key, ($event.target as HTMLSelectElement).value)"
          @blur="editingDimensions = false"
        >
          <option value="">-- {{ dim.name }} --</option>
          <template v-if="dim.source === 'monthly'">
            <option v-for="g in goalOptions" :key="g" :value="g">{{ g }}</option>
          </template>
          <template v-else>
            <option v-for="v in (dim.values || [])" :key="v" :value="v">{{ v }}</option>
          </template>
        </select>
      </template>
      <template v-else>
        <AppChip
          v-for="dim in orderedDimensions.filter(d => entry.dimensions[d.key])"
          :key="dim.key"
          :color="chipColor(dim.key)"
          :value="entry.dimensions[dim.key]"
          @click="editingDimensions = true"
        />
      </template>
    </div>

    <!-- Duration -->
    <template v-if="editingDuration">
      <input
        v-model="durInput"
        class="w-[56px] text-right px-[8px] py-[3px]
               border-2 border-[#8b5cf6] rounded-[var(--radius-form)]
               text-[var(--app-text-base)] leading-[1.4] tabular-nums
               outline-none bg-[#fafaff]
               shadow-[0_0_0_4px_rgba(139,92,246,0.12),0_0_20px_rgba(139,92,246,0.06)]"
        @keydown.enter.prevent="commitDuration"
        @keydown.escape.prevent="editingDuration = false"
        @blur="commitDuration"
        autofocus
      />
    </template>
    <template v-else>
      <span
        class="text-[var(--app-text-base)] text-[var(--color-text-secondary)] tabular-nums
               flex-shrink-0 cursor-default rounded px-[4px] hover:bg-[var(--color-divider)]"
        @dblclick="startEditDuration"
      >
        {{ formatDuration(entry.duration) }}
      </span>
    </template>

    <!-- Delete -->
    <button
      class="text-[var(--color-text-secondary)] hover:text-[var(--color-danger)]
             text-[18px] leading-none flex-shrink-0 p-[2px]
             opacity-0 group-hover:opacity-100 transition-opacity duration-150 cursor-pointer"
      @click="$emit('delete', entry.id)"
    >
      &times;
    </button>
  </div>
</template>
