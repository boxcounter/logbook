<script setup lang="ts">
import { ref, nextTick, computed, onMounted, onUnmounted } from "vue";
import type { Entry } from "../types";
import { formatDuration, resolveDelta } from "../utils/format";
import { useStore } from "../stores/useStore";

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

// Item text editing
const editingItem = ref(false);
const itemInput = ref("");
const itemInputEl = ref<HTMLInputElement | null>(null);

function startEditItem() {
  itemInput.value = props.entry.item;
  editingItem.value = true;
  nextTick(() => {
    const el = itemInputEl.value;
    if (el) {
      el.focus();
      el.setSelectionRange(el.value.length, el.value.length);
    }
  });
}

function commitItem() {
  editingItem.value = false;
  const newItem = itemInput.value.trim() || "(untitled)";
  if (newItem !== props.entry.item) {
    emit("update", props.entry.id, newItem, props.entry.duration);
  }
}

function cancelItem() {
  editingItem.value = false;
}

function handleItemKey(e: KeyboardEvent) {
  if (e.key === "Enter") { e.preventDefault(); commitItem(); }
  if (e.key === "Escape") { e.preventDefault(); cancelItem(); }
}

// Duration editing
const editingDuration = ref(false);
const durInput = ref("");
const durInputEl = ref<HTMLInputElement | null>(null);

function startEditDuration() {
  durInput.value = String(props.entry.duration);
  editingDuration.value = true;
  nextTick(() => {
    const el = durInputEl.value;
    if (el) {
      el.focus();
      el.setSelectionRange(el.value.length, el.value.length);
    }
  });
}

function commitDuration() {
  editingDuration.value = false;
  const newDur = resolveDelta(durInput.value, props.entry.duration);
  if (newDur !== props.entry.duration) {
    emit("update", props.entry.id, props.entry.item, newDur);
  }
}

function cancelDuration() {
  editingDuration.value = false;
}

function handleDurKey(e: KeyboardEvent) {
  if (e.key === "Enter") { e.preventDefault(); commitDuration(); }
  if (e.key === "Escape") { e.preventDefault(); cancelDuration(); }
}

// #3: Dimension editing
const editingDimensions = ref(false);
const dimSelectsEl = ref<HTMLElement | null>(null);

// All dimensions in config order (not splitting static/monthly)
const orderedDimensions = computed(() =>
  store.config?.dimensions || []
);

const goalOptions = computed(() => {
  const goals = new Set<string>();
  for (const c of store.commitments) {
    for (const g of c.goals) goals.add(g);
  }
  return [...goals];
});

function dimLabel(dims: Record<string, string>): string {
  const configDims = store.config?.dimensions || [];
  return configDims
    .map((d) => dims[d.key])
    .filter((v): v is string => !!v)
    .join(" · ");
}

function startEditDimensions() {
  editingDimensions.value = true;
}

function handleDimChange(key: string, value: string) {
  const newDims = { ...props.entry.dimensions, [key]: value };
  emit("updateDimensions", props.entry.id, newDims);
}

// Click outside → close dimension selects
function onClickOutside(e: MouseEvent) {
  if (!editingDimensions.value) return;
  const el = dimSelectsEl.value;
  if (el && !el.contains(e.target as Node)) {
    editingDimensions.value = false;
  }
}

onMounted(() => document.addEventListener("click", onClickOutside));
onUnmounted(() => document.removeEventListener("click", onClickOutside));
</script>

<template>
  <div class="flex items-start gap-3 py-3 border-b border-gray-100 last:border-b-0 group">
    <span class="text-xs text-gray-400 w-5 text-right pt-0.5 tabular-nums shrink-0">{{ index + 1 }}</span>
    <div class="flex-1 min-w-0">
      <!-- Item text: double-click to edit -->
      <div v-if="editingItem" class="text-sm">
        <input
          ref="itemInputEl"
          v-model="itemInput"
          class="w-full px-1 py-0.5 border-2 border-blue-500 rounded text-sm outline-none"
          @keydown="handleItemKey"
          @blur="commitItem"
        />
      </div>
      <div
        v-else
        class="text-sm text-gray-800 cursor-default rounded px-0.5 -mx-0.5 hover:bg-gray-50"
        @dblclick="startEditItem"
      >
        {{ entry.item }}
      </div>
      <!-- #3: Dimension display / editing -->
      <div
        v-if="dimLabel(entry.dimensions) || editingDimensions"
        class="text-xs text-gray-400 mt-0.5 cursor-default rounded px-0.5 -mx-0.5 hover:bg-gray-50"
        @click.stop="startEditDimensions"
      >
        <template v-if="editingDimensions">
          <div ref="dimSelectsEl" class="flex flex-wrap gap-1.5 mt-1" @click.stop>
            <select
              v-for="dim in orderedDimensions"
              :key="dim.key"
              :value="entry.dimensions[dim.key] || ''"
              class="px-1.5 py-0.5 text-xs border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
              @change="handleDimChange(dim.key, ($event.target as HTMLSelectElement).value)"
            >
              <option value="">-- {{ dim.name }} --</option>
              <template v-if="dim.source === 'monthly'">
                <option v-for="g in goalOptions" :key="g" :value="g">{{ g }}</option>
              </template>
              <template v-else>
                <option v-for="v in (dim.values || [])" :key="v" :value="v">{{ v }}</option>
              </template>
            </select>
          </div>
        </template>
        <template v-else>
          {{ dimLabel(entry.dimensions) }}
        </template>
      </div>
    </div>
    <!-- Duration: double-click to edit -->
    <span v-if="editingDuration" class="text-sm shrink-0">
      <input
        ref="durInputEl"
        v-model="durInput"
        class="w-14 text-right px-1 py-0.5 border-2 border-blue-500 rounded text-sm outline-none tabular-nums"
        @keydown="handleDurKey"
        @blur="commitDuration"
      />
    </span>
    <span
      v-else
      class="text-sm text-gray-600 tabular-nums shrink-0 cursor-default rounded px-1 hover:bg-gray-50"
      @dblclick="startEditDuration"
    >
      {{ formatDuration(entry.duration) }}
    </span>
    <!-- Delete: hover only -->
    <button
      class="text-gray-400 hover:text-red-500 text-base leading-none shrink-0 opacity-0 group-hover:opacity-100 transition-opacity p-0.5"
      title="Delete"
      @click="emit('delete', props.entry.id)"
    >
      ×
    </button>
  </div>
</template>
