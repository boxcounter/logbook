<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import draggable from "vuedraggable";
import type { Commitment, CommitmentProgress } from "../../types";

interface GoalRow { name: string; origName: string | null; key: number }
interface RoleRow { role: string; allocation: number; goals: GoalRow[]; origRole: string | null; key: number }

const props = defineProps<{
  open: boolean;
  commitments: Commitment[];
  progress: CommitmentProgress[];
  rootPath: string;
  selectedYear: number;
  selectedMonth: number;
}>();

const emit = defineEmits<{ saved: []; close: [] }>();

const NEW_ROLE_ALLOC = 5;
let _key = 0;
const nextKey = () => ++_key;

const draft = ref<RoleRow[]>([]);
const error = ref("");
const saving = ref(false);

function buildDraft() {
  draft.value = props.commitments.map(c => ({
    role: c.role, allocation: c.allocation, origRole: c.role, key: nextKey(),
    goals: c.goals.map(g => ({ name: g, origName: g, key: nextKey() })),
  }));
  error.value = "";
}
watch(() => props.open, (o) => { if (o) buildDraft(); }, { immediate: true });

function toCommitments(rows: RoleRow[]): Commitment[] {
  return rows.map(r => ({
    role: r.role.trim(),
    allocation: r.allocation,
    goals: r.goals.map(g => g.name.trim()).filter(n => n !== ""),
  }));
}

function addRole() {
  draft.value.push({ role: "", allocation: NEW_ROLE_ALLOC, origRole: null, key: nextKey(), goals: [{ name: "", origName: null, key: nextKey() }] });
}
function addGoal(ri: number) {
  draft.value[ri].goals.push({ name: "", origName: null, key: nextKey() });
}

const STEP = 5;
const MIN_ALLOC = 5;
function stepAlloc(ri: number, delta: number) {
  draft.value[ri].allocation = Math.max(MIN_ALLOC, (draft.value[ri].allocation || 0) + delta);
}
// Allocation has two floors: the stepper buttons enforce a soft MIN_ALLOC (5h)
// floor, while direct typing only clamps to the hard >0 floor (1) the backend
// requires. Typed values of 1–4 are therefore legal.
function onAllocInput(ri: number, e: Event) {
  const el = e.target as HTMLInputElement;
  const v = Math.floor(Number(el.value));
  const next = Number.isFinite(v) && v >= 1 ? v : 1;
  draft.value[ri].allocation = next;
  // Re-sync the DOM in case the clamped value equals the previous model value
  // (no model change → no Vue patch → the field would otherwise stay desynced,
  // e.g. clearing the field while already at the floor).
  if (el.value !== String(next)) el.value = String(next);
}

const monthLabel = computed(() =>
  new Date(props.selectedYear, props.selectedMonth - 1, 1).toLocaleDateString("en-US", { month: "long", year: "numeric" })
);

async function save() {
  saving.value = true;
  error.value = "";
  try {
    await invoke("set_commitments", {
      rootPath: props.rootPath,
      year: props.selectedYear,
      month: props.selectedMonth,
      commitments: toCommitments(draft.value),
    });
    emit("saved");
    emit("close");
  } catch (e) {
    error.value = typeof e === "string" ? e : String(e);
  } finally {
    saving.value = false;
  }
}
function cancel() { emit("close"); }
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      data-test="overlay" tabindex="-1"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
    >
      <div
        role="dialog" aria-modal="true"
        class="relative w-[660px] max-w-[92vw] max-h-[88vh] flex flex-col bg-[var(--color-surface)]
               border border-[var(--color-border-form)] rounded-[var(--radius-lg)]
               shadow-[var(--shadow-popover)] overflow-hidden"
      >
        <!-- Header -->
        <div class="flex justify-between items-start px-[28px] pt-[24px] pb-[16px] border-b border-[var(--color-divider)]">
          <div>
            <div class="text-[length:var(--app-text-xl)] font-bold text-[var(--color-text-primary)] tracking-[-0.3px]">Edit Commitments</div>
            <div class="text-[length:var(--app-text-xs)] text-[var(--color-text-muted)] mt-[2px]">{{ monthLabel }}</div>
          </div>
        </div>

        <!-- Body -->
        <div class="px-[28px] pt-[16px] pb-[4px] overflow-y-auto">
          <draggable v-model="draft" item-key="key" handle=".drag-grip-role" tag="div" :animation="150">
            <template #item="{ element: r, index: ri }">
              <div class="bg-[var(--color-page-bg)] border border-[var(--color-divider)] rounded-[var(--radius-form-lg)] p-[16px] mb-[12px]" data-test="role-card">
                <div class="flex items-center gap-[8px]">
                  <span data-test="drag-grip-role" class="drag-grip-role cursor-grab text-[var(--color-text-disabled)] select-none px-[2px]">⠿</span>
                  <input
                    v-model="r.role" data-test="role-name" placeholder="Role"
                    class="flex-1 px-[10px] py-[6px] border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                           text-[length:var(--app-text-base)] font-semibold text-[var(--color-text-primary)]
                           bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                  />
                  <span class="inline-flex items-center gap-[5px]">
                    <button
                      data-test="alloc-dec" :disabled="r.allocation <= MIN_ALLOC"
                      class="w-[24px] h-[26px] flex items-center justify-center border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-[length:var(--app-text-base)] text-[var(--color-text-secondary)] bg-[var(--color-surface)]
                             hover:border-[var(--color-brand-solid)] hover:text-[var(--color-brand-link)]
                             disabled:text-[var(--color-text-disabled)] disabled:cursor-not-allowed disabled:hover:border-[var(--color-border-form)]
                             cursor-pointer transition-[border-color,color] duration-150"
                      @click="stepAlloc(ri, -STEP)"
                    >&minus;</button>
                    <input
                      :value="r.allocation" type="number" data-test="alloc"
                      class="w-[42px] text-center px-[4px] py-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-[length:var(--app-text-base)] font-semibold text-[var(--color-text-primary)] mono
                             bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                      @input="onAllocInput(ri, $event)"
                      @keydown.up.prevent="stepAlloc(ri, STEP)"
                      @keydown.down.prevent="stepAlloc(ri, -STEP)"
                    />
                    <button
                      data-test="alloc-inc"
                      class="w-[24px] h-[26px] flex items-center justify-center border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-[length:var(--app-text-base)] text-[var(--color-text-secondary)] bg-[var(--color-surface)]
                             hover:border-[var(--color-brand-solid)] hover:text-[var(--color-brand-link)]
                             cursor-pointer transition-[border-color,color] duration-150"
                      @click="stepAlloc(ri, STEP)"
                    >+</button>
                    <span class="text-[length:var(--app-text-xs-alt)] text-[var(--color-text-muted)]">h</span>
                  </span>
                </div>

                <div class="mt-[12px]">
                  <draggable v-model="r.goals" item-key="key" handle=".drag-grip-goal" tag="div" class="flex flex-col gap-[8px]" :animation="150">
                    <template #item="{ element: g, index: gi }">
                      <div class="flex items-center gap-[8px]">
                        <span data-test="drag-grip-goal" class="drag-grip-goal cursor-grab text-[var(--color-text-disabled)] select-none px-[2px]">⠿</span>
                        <input
                          v-model="g.name" data-test="goal-name" placeholder="Goal name"
                          class="flex-1 px-[10px] py-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                                 text-[length:var(--app-text-sm)] text-[var(--color-text-secondary)]
                                 bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                        />
                        <button
                          data-test="goal-remove"
                          class="text-[length:var(--app-text-base)] text-[var(--color-text-disabled)] hover:text-[var(--color-danger)] cursor-pointer px-[4px]"
                          @click="r.goals.splice(gi, 1)"
                        >&times;</button>
                      </div>
                    </template>
                  </draggable>
                  <button
                    data-test="add-goal"
                    class="self-start mt-[8px] text-[length:var(--app-text-xs)] font-medium text-[var(--color-brand-link)] cursor-pointer hover:underline"
                    @click="addGoal(ri)"
                  >+ Add Goal</button>
                </div>
              </div>
            </template>
          </draggable>

          <button
            data-test="add-role"
            class="my-[4px] py-[6px] text-[length:var(--app-text-sm)] font-semibold text-[var(--color-brand-link)] cursor-pointer hover:underline"
            @click="addRole"
          >+ Add Role</button>
        </div>

        <!-- Footer -->
        <div v-if="error" class="px-[28px] pb-[8px] text-[length:var(--app-text-xs-alt)] text-[var(--color-danger)]">{{ error }}</div>
        <div class="flex justify-end gap-[8px] px-[28px] py-[16px] border-t border-[var(--color-divider)]">
          <button
            data-test="cancel"
            class="text-[length:var(--app-text-sm)] font-semibold text-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)] rounded-[var(--radius-form)] px-[14px] py-[6px] cursor-pointer"
            @click="cancel"
          >Cancel</button>
          <button
            data-test="save" :disabled="saving"
            class="text-[length:var(--app-text-sm)] font-semibold text-white bg-[var(--color-brand-solid)] hover:bg-[var(--color-brand-link)] rounded-[var(--radius-form)] px-[14px] py-[6px] cursor-pointer disabled:opacity-50"
            @click="save"
          >{{ saving ? "Saving…" : "Save" }}</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
