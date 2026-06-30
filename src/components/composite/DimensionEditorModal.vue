<script setup lang="ts">
import { ref, computed, watch, nextTick } from "vue";
import type { Dimension } from "../../types";

const props = defineProps<{
  open: boolean;
  dimensions: Dimension[];
  rootPath: string;
  year: number;
  month: number;
}>();

const emit = defineEmits<{ close: []; saved: [Dimension[]] }>();

const overlayRef = ref<HTMLElement>();
const showDiscard = ref(false);
const draft = ref<Dimension[]>([]);
const selectedIndex = ref(0);

watch(() => props.open, (o) => {
  if (!o) return;
  draft.value = JSON.parse(JSON.stringify(props.dimensions));
  selectedIndex.value = 0;
  showDiscard.value = false;
  nextTick(() => overlayRef.value?.focus());
}, { immediate: true });

const selectedDimension = computed(() => draft.value[selectedIndex.value] ?? null);

function selectDim(index: number) { selectedIndex.value = index; }

function requestClose() { emit("close"); }

// Keyboard: esc to close, cmd+enter to save (placeholder)
function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") { e.preventDefault(); requestClose(); }
}

const monthLabel = new Date(props.year, props.month - 1, 1)
  .toLocaleDateString("en-US", { month: "long", year: "numeric" });
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      ref="overlayRef"
      data-test="overlay" tabindex="-1"
      @keydown="onKeydown"
      @click.self="requestClose"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
    >
      <div
        role="dialog" aria-modal="true"
        class="relative w-[660px] max-w-[92vw] max-h-[88vh] flex flex-col bg-[var(--color-surface)]
               border border-[var(--color-border-form)] rounded-[var(--radius-lg)]
               shadow-[var(--shadow-popover)] overflow-hidden"
      >
        <!-- Header -->
        <div class="flex justify-between items-start px-2xl pt-xl pb-lg border-b border-[var(--color-divider)]">
          <div>
            <div class="text-title font-bold text-[var(--color-text-primary)] tracking-[-0.3px]">Edit Dimensions</div>
            <div class="text-secondary text-[var(--color-text-muted)] mt-2xs">Editing {{ monthLabel }}</div>
          </div>

        </div>

        <!-- Body: two-column layout -->
        <div class="flex-1 flex min-h-0">
          <!-- Left panel: dimension list -->
          <div class="w-[210px] flex-shrink-0 border-r border-[var(--color-divider)] bg-[var(--color-surface-muted)] p-md flex flex-col">
            <div class="flex-1 space-y-2xs">
              <div
                v-for="(dim, i) in draft"
                :key="dim.key"
                data-test="dim-row"
                :class="[
                  'flex items-center gap-sm px-sm py-sm rounded-[var(--radius-form-lg)] cursor-pointer',
                  i === selectedIndex ? 'bg-[var(--color-brand-soft-bg)]' : ''
                ]"
                @click="selectDim(i)"
              >
                <div
                  class="w-[3px] h-[16px] rounded-[1px] flex-shrink-0"
                  :style="{ background: `var(--dim-bar-${dim.key})` }"
                ></div>
                <span class="text-body text-[var(--color-text-primary)] flex-1">{{ dim.name }}</span>
                <span class="text-micro text-[var(--color-text-muted)]">{{ dim.source }}</span>
              </div>
            </div>
            <button class="text-secondary font-semibold text-[var(--color-brand-link)] text-left mt-sm cursor-pointer">
              + Add dimension
            </button>
          </div>

          <!-- Right panel placeholder -->
          <div class="flex-1 px-2xl py-xl">
            <input
              v-if="selectedDimension"
              placeholder="Dimension name"
              :value="selectedDimension.name"
              class="text-body text-[var(--color-text-primary)] bg-[var(--color-surface)] border border-[var(--color-border-form)] rounded-[var(--radius-form)] px-sm py-xs w-full"
            />
            <p v-else class="text-secondary text-[var(--color-text-muted)]">Right panel — next task.</p>
          </div>
        </div>

        <!-- Footer -->
        <div class="flex justify-end gap-sm px-2xl py-lg border-t border-[var(--color-divider)]">
          <button
            data-test="cancel"
            class="text-secondary font-semibold text-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer"
            @click="requestClose"
          >Cancel</button>
          <button
            data-test="save"
            class="text-secondary font-semibold text-white bg-[var(--color-brand-solid)] hover:bg-[var(--color-brand-link)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer disabled:opacity-50"
          >Save</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
