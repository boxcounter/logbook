<script setup lang="ts">
import { ref, computed, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { VueDraggable } from "vue-draggable-plus";
import type { Dimension } from "../../types";
import { dimensionHues, dimBar } from "../../utils/dimensionColor";
import { logError } from "../../utils/errorLog";

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
const error = ref("");
const saving = ref(false);
const savingTemplate = ref(false);
const templateSaved = ref(false);

// Add dimension form
const showAddForm = ref(false);
const newDimName = ref("");
const newDimKey = ref("");
const newDimSource = ref<"static" | "commitments:goals" | "commitments:role">("static");
const addFormError = ref("");

// Show deleted
const showDeleted = ref(false);

const hasDeleted = computed(() => draft.value.some(d => d.deleted));

watch(() => props.open, (o) => {
  if (!o) return;
  draft.value = JSON.parse(JSON.stringify(props.dimensions));
  selectedIndex.value = 0;
  showDiscard.value = false;
  error.value = "";
  saving.value = false;
  showAddForm.value = false;
  showDeleted.value = false;
  newDimName.value = "";
  newDimKey.value = "";
  newDimSource.value = "static";
  addFormError.value = "";
  nextTick(() => overlayRef.value?.focus());
}, { immediate: true });

const selectedDimension = computed(() => draft.value[selectedIndex.value] ?? null);

const draftHues = computed(() => dimensionHues(draft.value));

const isDirty = computed(() =>
  JSON.stringify(draft.value) !== JSON.stringify(props.dimensions),
);

function selectDim(index: number) { selectedIndex.value = index; }

function requestClose() {
  if (isDirty.value) { showDiscard.value = true; return; }
  emit("close");
}

function confirmDiscard() { showDiscard.value = false; emit("close"); }
function keepEditing() { showDiscard.value = false; }

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") { e.preventDefault(); requestClose(); }
  if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) { e.preventDefault(); save(); }
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

function removeValue(index: number) {
  if (!selectedDimension.value?.values) return;
  selectedDimension.value.values = selectedDimension.value.values.filter((_, i) => i !== index);
}

function onValueEnter(index: number, e?: KeyboardEvent) {
  if (e?.isComposing) return;
  if (!selectedDimension.value?.values) return;
  const values = selectedDimension.value.values;
  if (index === values.length - 1 && values[index].trim() === "") return;
  values.splice(index + 1, 0, "");
}

function toggleDelete() {
  if (selectedDimension.value) {
    selectedDimension.value.deleted = !selectedDimension.value.deleted;
  }
}

// ── Add dimension form ────────────────────────────────────────────

function resetAddForm() {
  showAddForm.value = false;
  newDimName.value = "";
  newDimKey.value = "";
  newDimSource.value = "static";
  addFormError.value = "";
}

function validateNewDim(): string | null {
  const key = newDimKey.value.trim();
  if (!key) return "Key is required";
  if (!/^[a-zA-Z0-9_-]+$/.test(key)) return "Only letters, numbers, hyphens, and underscores allowed";

  const duplicate = draft.value.find(d => d.key === key);
  if (duplicate?.deleted) return `Key '${key}' already exists (deleted). Restore it or choose a different key.`;
  if (duplicate) return `Key '${key}' already exists.`;

  if (newDimSource.value === "commitments:goals" && draft.value.some(d => d.source === "commitments:goals" && !d.deleted)) {
    return "Only one commitments:goals source dimension allowed";
  }

  if (newDimSource.value === "commitments:role" && draft.value.some(d => d.source === "commitments:role" && !d.deleted)) {
    return "Only one commitments:role source dimension allowed";
  }

  return null;
}

function createDimension() {
  const err = validateNewDim();
  if (err) {
    addFormError.value = err;
    return;
  }

  const dim: Dimension = {
    name: newDimName.value.trim(),
    key: newDimKey.value.trim(),
    source: newDimSource.value,
    values: newDimSource.value === "static" ? [] : undefined,
    required: false,
    deleted: false,
  };

  draft.value = [...draft.value, dim];
  selectedIndex.value = draft.value.length - 1;
  resetAddForm();
}

// ── Save ──────────────────────────────────────────────────────────

async function save() {
  saving.value = true;
  error.value = "";
  try {
    const cleaned = draft.value.map(d => {
      if (d.source === "static" && d.values) {
        return { ...d, values: d.values.filter(v => v.trim() !== "") };
      }
      return d;
    });
    const result = await invoke<Dimension[]>("save_dimensions", {
      rootPath: props.rootPath,
      year: props.year,
      month: props.month,
      dimensions: cleaned,
    });
    emit("saved", result);
    emit("close");
  } catch (e: unknown) {
    error.value = typeof e === "string" ? e : (e as Error).message ?? "Save failed";
  } finally {
    saving.value = false;
  }
}

async function saveAsTemplate() {
  savingTemplate.value = true;
  error.value = "";
  templateSaved.value = false;
  try {
    const active = draft.value.filter(d => !d.deleted);
    await invoke("save_dimensions_template", {
      rootPath: props.rootPath,
      dimensions: active,
    });
    templateSaved.value = true;
    setTimeout(() => { templateSaved.value = false; }, 2000);
  } catch (e: unknown) {
    const msg = typeof e === "string" ? e : (e as Error).message ?? "Save template failed";
    error.value = msg;
    logError("DimensionEditorModal.saveAsTemplate", msg);
  } finally {
    savingTemplate.value = false;
  }
}

const monthLabel = computed(() =>
  new Date(props.year, props.month - 1, 1)
    .toLocaleDateString("en-US", { month: "long", year: "numeric" })
);
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
            <div class="text-secondary text-[var(--color-text-muted)] mt-2xs">
              Editing {{ monthLabel }}
              <span class="text-[var(--color-text-disabled)]">|</span>
              <button
                data-test="save-as-template"
                class="ml-2xs text-secondary font-semibold text-[var(--color-brand-link)] cursor-pointer disabled:opacity-50 disabled:cursor-default"
                :disabled="savingTemplate"
                @click="saveAsTemplate"
              >{{ savingTemplate ? 'Saving...' : templateSaved ? 'Saved!' : 'Save as template' }}</button>
            </div>
          </div>

        </div>

        <!-- Body: two-column layout -->
        <div class="flex-1 flex min-h-0">
          <!-- Left panel: dimension list -->
          <div class="w-[210px] flex-shrink-0 border-r border-[var(--color-divider)] bg-[var(--color-surface-muted)] p-md flex flex-col">
            <div class="flex-1 space-y-2xs">
              <VueDraggable
                v-model="draft"
                handle=".drag-grip-dim"
                :animation="150"
                class="space-y-2xs"
              >
                <template v-for="(dim, index) in draft" :key="dim.key">
                  <div
                    data-test="dim-row"
                    :class="[
                      'flex items-center gap-sm px-sm py-sm rounded-[var(--radius-form-lg)] cursor-pointer',
                      index === selectedIndex ? 'bg-[var(--color-brand-soft-bg)]' : '',
                      dim.deleted ? (showDeleted ? 'opacity-40' : 'hidden') : '',
                    ]"
                    @click="selectDim(index)"
                  >
                    <span
                      data-test="drag-grip-dim"
                      :class="[
                        'text-[var(--color-text-disabled)] select-none px-2xs',
                        dim.deleted ? '' : 'cursor-grab drag-grip-dim',
                      ]">⠿</span>
                    <div
                      class="w-[3px] h-[16px] rounded-[1px] flex-shrink-0"
                      :style="{ background: dimBar(draftHues.get(dim.key) ?? null) }"
                    ></div>
                    <span class="text-body text-[var(--color-text-primary)] flex-1">{{ dim.name }}</span>
                    <span class="text-micro text-[var(--color-text-muted)]">{{ dim.source }}</span>
                  </div>
                </template>
              </VueDraggable>

              <!-- Add dimension form -->
              <div
                v-if="showAddForm"
                data-test="add-dim-form"
                class="border border-[var(--color-brand-solid)] rounded-[var(--radius-form-lg)] bg-[var(--color-brand-soft-bg)] p-sm space-y-xs"
              >
                <input
                  v-model="newDimName"
                  data-test="add-dim-name"
                  placeholder="Dimension name"
                  class="w-full px-sm py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                         text-body text-[var(--color-text-primary)] bg-[var(--color-surface)]
                         outline-none focus:border-[var(--color-brand-solid)] placeholder-[var(--color-placeholder)]"
                  @keydown.enter.prevent="createDimension"
                />
                <input
                  v-model="newDimKey"
                  data-test="add-dim-key"
                  placeholder="Key (e.g. project)"
                  class="w-full px-sm py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                         text-body text-[var(--color-text-primary)] bg-[var(--color-surface)]
                         outline-none focus:border-[var(--color-brand-solid)] placeholder-[var(--color-placeholder)]"
                  @keydown.enter.prevent="createDimension"
                />
                <select
                  v-model="newDimSource"
                  data-test="add-dim-source"
                  class="w-full px-sm py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                         text-body text-[var(--color-text-primary)] bg-[var(--color-surface)]
                         outline-none focus:border-[var(--color-brand-solid)]"
                >
                  <option value="static">Static</option>
                  <option value="commitments:goals">Commitments: Goals</option>
                  <option value="commitments:role">Commitments: Role</option>
                </select>
                <div
                  v-if="addFormError"
                  data-test="add-dim-error"
                  class="text-micro text-[var(--color-danger)]"
                >{{ addFormError }}</div>
                <div class="flex justify-end gap-sm">
                  <button
                    data-test="add-dim-cancel"
                    class="text-secondary font-semibold text-[var(--color-text-muted)] rounded-[var(--radius-form)] px-sm py-xs cursor-pointer"
                    @click="resetAddForm"
                  >Cancel</button>
                  <button
                    data-test="add-dim-create"
                    class="text-secondary font-semibold text-white bg-[var(--color-brand-solid)] rounded-[var(--radius-form)] px-sm py-xs cursor-pointer"
                    @click="createDimension"
                  >Create</button>
                </div>
              </div>
            </div>

            <!-- Show deleted toggle -->
            <label
              v-if="hasDeleted"
              data-test="show-deleted-toggle"
              class="flex items-center gap-xs cursor-pointer mt-sm"
            >
              <input
                type="checkbox"
                v-model="showDeleted"
                class="rounded-[var(--radius-form)]"
              />
              <span class="text-secondary text-[var(--color-text-secondary)]">Show deleted</span>
            </label>

            <button
              data-test="add-dim-btn"
              class="text-secondary font-semibold text-[var(--color-brand-link)] text-left mt-sm cursor-pointer"
              @click="showAddForm = !showAddForm"
            >+ Add dimension</button>
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
                  :disabled="selectedDimension.deleted"
                  @input="updateDimName"
                  class="text-title font-semibold text-[var(--color-text-primary)] bg-transparent
                         border-0 border-b-2 border-[var(--color-border-form)] rounded-none
                         px-0 pb-xs w-full outline-none focus:border-[var(--color-brand-solid)]
                         disabled:opacity-40"
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
                      data-test="required-checkbox"
                      :checked="selectedDimension.required"
                      :disabled="selectedDimension.deleted"
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
                  <VueDraggable
                    v-model="selectedDimension.values"
                    handle=".drag-grip-val"
                    :animation="150"
                    class="space-y-xs"
                  >
                    <div
                      v-for="(val, i) in selectedDimension.values"
                      :key="i"
                      class="flex items-center gap-sm"
                    >
                      <span class="text-[var(--color-text-disabled)] select-none px-2xs" :class="selectedDimension.deleted ? '' : 'cursor-grab drag-grip-val'">⠿</span>
                       <input
                        data-test="value-input"
                        :value="val"
                        :disabled="selectedDimension.deleted"
                        @input="updateValue(i, $event)"
                        @keydown.enter.exact.prevent="onValueEnter(i, $event)"
                        class="flex-1 px-sm py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                               text-body text-[var(--color-text-primary)]
                               bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]
                               disabled:opacity-40"
                      />
                      <button
                        v-if="!selectedDimension.deleted"
                        data-test="delete-value"
                        class="text-body cursor-pointer px-xs transition-[color] duration-[var(--motion-fast)]
                               text-[var(--color-text-disabled)] hover:text-[var(--color-danger)]"
                        @click="removeValue(i)"
                      >&times;</button>
                    </div>
                  </VueDraggable>

                  <button
                    v-if="!selectedDimension.deleted"
                    data-test="add-value-btn"
                    class="self-start mt-sm text-secondary font-medium text-[var(--color-brand-link)] cursor-pointer hover:underline"
                    @click="selectedDimension.values = [...selectedDimension.values, '']"
                  >+ Add Value</button>
                </template>

                <!-- Monthly info card -->
                <template v-if="selectedDimension.source === 'commitments:goals'">
                  <div class="border border-[var(--color-border-form)] rounded-[var(--radius-form-lg)] bg-[var(--color-surface-muted)] p-md">
                    <p class="text-secondary text-[var(--color-text-muted)]">Values are derived from commitment goals.</p>
                  </div>
                </template>
                <template v-if="selectedDimension.source === 'commitments:role'">
                  <div class="border border-[var(--color-border-form)] rounded-[var(--radius-form-lg)] bg-[var(--color-surface-muted)] p-md">
                    <p class="text-secondary text-[var(--color-text-muted)]">Values are derived from commitment roles.</p>
                  </div>
                </template>
              </div>

              <!-- Delete / Restore dimension button -->
              <div class="border-t border-[var(--color-divider)] px-2xl py-lg flex-shrink-0">
                <button
                  data-test="delete-dim"
                  class="text-secondary font-semibold cursor-pointer transition-[color] duration-[var(--motion-fast)]"
                  :class="selectedDimension.deleted
                    ? 'text-[var(--color-brand-link)]'
                    : 'text-[var(--color-text-disabled)] hover:text-[var(--color-danger)]'"
                  @click="toggleDelete"
                >{{ selectedDimension.deleted ? 'Restore' : 'Delete dimension' }}</button>
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
        <div class="flex flex-col">
          <div
            v-if="error"
            data-test="save-error"
            class="text-secondary text-[var(--color-danger)] px-2xl py-sm"
          >{{ error }}</div>
          <div class="flex justify-end gap-sm px-2xl py-lg border-t border-[var(--color-divider)]">
            <button
              data-test="cancel"
              class="text-secondary font-semibold text-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer"
              @click="requestClose"
            >Cancel</button>
            <button
              data-test="save"
              class="text-secondary font-semibold text-white bg-[var(--color-brand-solid)] hover:bg-[var(--color-brand-link)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer disabled:opacity-50"
              :disabled="saving"
              @click="save"
            >{{ saving ? 'Saving...' : 'Save' }}</button>
          </div>
        </div>

        <!-- Discard confirmation overlay -->
        <div
          v-if="showDiscard"
          data-test="discard-confirm"
          class="absolute inset-0 flex items-center justify-center bg-black/10"
          @click.self="keepEditing"
        >
          <div class="bg-[var(--color-surface)] border border-[var(--color-border-form)] rounded-[var(--radius-card)] shadow-[var(--shadow-toast)] p-lg max-w-[300px]">
            <div class="text-body font-semibold text-[var(--color-text-primary)] mb-xs">Discard changes?</div>
            <p class="text-secondary text-[var(--color-text-muted)] mb-md">You have unsaved changes to dimensions.</p>
            <div class="flex justify-end gap-sm">
              <button
                data-test="keep-editing"
                class="text-secondary font-semibold text-[var(--color-text-secondary)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer"
                @click="keepEditing"
              >Keep editing</button>
              <button
                data-test="discard-yes"
                class="text-secondary font-semibold text-[var(--color-danger)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer"
                @click="confirmDiscard"
              >Discard</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
