<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
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

watch(() => props.open, (o) => {
  if (!o) return;
  showDiscard.value = false;
  nextTick(() => overlayRef.value?.focus());
}, { immediate: true });

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

        <!-- Body placeholder -->
        <div class="flex-1 overflow-y-auto px-2xl py-xl">
          <p class="text-secondary text-[var(--color-text-muted)]">Dimension editor body — coming in next task.</p>
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
