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
const newValue = ref("");

watch(() => props.open, (o) => {
  if (!o) return;
  draft.value = JSON.parse(JSON.stringify(props.dimensions));
  selectedIndex.value = 0;
  showDiscard.value = false;
  newValue.value = "";
  nextTick(() => overlayRef.value?.focus());
}, { immediate: true });

const selectedDimension = computed(() => draft.value[selectedIndex.value] ?? null);

function selectDim(index: number) { selectedIndex.value = index; }

function requestClose() { emit("close"); }

// Keyboard: esc to close, cmd+enter to save (placeholder)
function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") { e.preventDefault(); requestClose(); }
}

function updateDimName(e: Event) {
  const target = e.target as HTMLInputElement;
  if (selectedDimension.value) {
    selectedDimension.value.name = target.value;
  }
}

function updateValue(index: number, e: Event) {
  const target = e.target as HTMLInputElement;
  if (!selectedDimension.value?.values) return;
  selectedDimension.value.values = selectedDimension.value.values.map((v, i) =>
    i === index ? target.value : v,
  );
}

function addValue() {
  const val = newValue.value.trim();
  if (!val || !selectedDimension.value?.values) return;
  selectedDimension.value.values = [...selectedDimension.value.values, val];
  newValue.value = "";
}

function removeValue(index: number) {
  if (!selectedDimension.value?.values) return;
  selectedDimension.value.values = selectedDimension.value.values.filter((_, i) => i !== index);
}

function toggleDelete() {
  if (selectedDimension.value) {
    selectedDimension.value.deleted = !selectedDimension.value.deleted;
  }
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

          <!-- Right panel -->
          <template v-if="selectedDimension">
            <div class="flex-1 flex flex-col min-h-0">
              <!-- Scrollable content -->
              <div class="flex-1 overflow-y-auto px-2xl py-xl">
                <!-- Name input: text-title, bottom border only -->
                <input
                  placeholder="Dimension name"
                  :value="selectedDimension.name"
                  @input="updateDimName"
                  class="text-title font-semibold text-[var(--color-text-primary)] bg-transparent
                         border-0 border-b-2 border-[var(--color-border-form)] rounded-none
                         px-0 pb-xs w-full outline-none focus:border-[var(--color-brand-solid)]"
                />

                <!-- Meta row: key + source + required -->
                <div class="flex items-center gap-lg mt-md flex-wrap">
                  <div class="flex items-center gap-xs">
                    <span class="text-micro uppercase tracking-wider text-[var(--color-text-disabled)]">Key</span>
                    <code class="text-body font-mono text-[var(--color-text-secondary)] bg-[var(--color-surface-muted)] px-sm py-2xs rounded-[var(--radius-sm)]">{{ selectedDimension.key }}</code>
                    <span class="text-micro text-[var(--color-text-disabled)]">(locked)</span>
                  </div>
                  <div class="flex items-center gap-xs">
                    <span class="text-micro uppercase tracking-wider text-[var(--color-text-disabled)]">Source</span>
                    <span class="text-secondary font-bold uppercase text-[var(--color-text-secondary)] bg-[var(--color-surface-muted)] px-sm py-2xs rounded-[var(--radius-sm)]">{{ selectedDimension.source }}</span>
                    <span class="text-micro text-[var(--color-text-disabled)]">(locked)</span>
                  </div>
                  <label class="flex items-center gap-xs ml-auto cursor-pointer">
                    <input
                      type="checkbox"
                      :checked="selectedDimension.required"
                      @change="selectedDimension.required = ($event.target as HTMLInputElement).checked"
                      class="rounded-[var(--radius-form)]"
                    />
                    <span class="text-secondary text-[var(--color-text-secondary)]">Required</span>
                  </label>
                </div>

                <div class="border-t border-[var(--color-divider)] my-lg"></div>

                <!-- Values section (static dimensions only) -->
                <template v-if="selectedDimension.source === 'static' && selectedDimension.values">
                  <div class="text-micro uppercase tracking-wider font-semibold text-[var(--color-text-disabled)] mb-sm">Values</div>
                  <div class="space-y-xs">
                    <div
                      v-for="(val, i) in selectedDimension.values"
                      :key="i"
                      class="flex items-center gap-sm"
                    >
                      <span class="text-[var(--color-text-disabled)] select-none px-2xs cursor-grab">⠿</span>
                      <input
                        data-test="value-input"
                        :value="val"
                        @input="updateValue(i, $event)"
                        class="flex-1 px-sm py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                               text-body text-[var(--color-text-primary)]
                               bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                      />
                      <button
                        data-test="delete-value"
                        class="text-body cursor-pointer px-xs transition-[color] duration-[var(--motion-fast)]
                               text-[var(--color-text-disabled)] hover:text-[var(--color-danger)]"
                        @click="removeValue(i)"
                      >&times;</button>
                    </div>
                  </div>

                  <!-- New value input -->
                  <div class="flex items-center gap-sm mt-sm">
                    <span class="text-[var(--color-text-disabled)] select-none px-2xs invisible">⠿</span>
                    <input
                      v-model="newValue"
                      placeholder="New value"
                      class="flex-1 px-sm py-xs border border-dashed border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-body text-[var(--color-text-primary)] placeholder-[var(--color-placeholder)]
                             bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                      @keydown.enter.exact.prevent="addValue"
                    />
                    <button
                      data-test="add-value"
                      class="text-secondary font-semibold text-[var(--color-brand-link)] px-sm py-xs cursor-pointer"
                      @click="addValue"
                    >+</button>
                  </div>
                </template>

                <!-- Monthly info card -->
                <template v-if="selectedDimension.source === 'monthly'">
                  <div class="border border-[var(--color-border-form)] rounded-[var(--radius-form-lg)] bg-[var(--color-surface-muted)] p-md">
                    <p class="text-secondary text-[var(--color-text-muted)]">Values are derived from commitment goals.</p>
                  </div>
                </template>
              </div>

              <!-- Delete dimension button -->
              <div class="border-t border-[var(--color-divider)] px-2xl py-lg flex-shrink-0">
                <button
                  data-test="delete-dim"
                  class="text-secondary font-semibold cursor-pointer transition-[color] duration-[var(--motion-fast)]"
                  :class="selectedDimension.deleted
                    ? 'text-[var(--color-danger)]'
                    : 'text-[var(--color-text-disabled)] hover:text-[var(--color-danger)]'"
                  @click="toggleDelete"
                >{{ selectedDimension.deleted ? 'Restore dimension' : 'Delete dimension' }}</button>
              </div>
            </div>
          </template>
          <template v-else>
            <div class="flex-1 flex items-center justify-center px-2xl">
              <p class="text-secondary text-[var(--color-text-muted)]">Select a dimension to edit</p>
            </div>
          </template>
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
