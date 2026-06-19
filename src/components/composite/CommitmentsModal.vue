<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import draggable from "vuedraggable";
import RoleCard from "./RoleCard.vue";
import type { Commitment, CommitmentProgress } from "../../types";
import { formatDuration } from "../../utils/format";

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

const monthLabel = computed(() =>
  new Date(props.selectedYear, props.selectedMonth - 1, 1).toLocaleDateString("en-US", { month: "long", year: "numeric" })
);

const committedHours = computed(() => draft.value.reduce((s, r) => s + (r.allocation || 0), 0));
const loggedTotal = computed(() => props.progress.reduce((s, p) => s + p.spent_minutes, 0));

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
          <div class="text-right text-[length:var(--app-text-xs-alt)] text-[var(--color-text-muted)] leading-[1.8]">
            <div>Committed <span data-test="committed" class="mono font-bold text-[var(--color-brand-link)]">{{ committedHours }}h</span></div>
            <div>Logged <span data-test="logged" class="mono font-semibold text-[var(--color-text-primary)]">{{ formatDuration(loggedTotal) }}</span></div>
          </div>
        </div>

        <!-- Body -->
        <div class="px-[28px] pt-[16px] pb-[4px] overflow-y-auto">
          <draggable v-model="draft" item-key="key" handle=".drag-grip-role" tag="div" :animation="150">
            <template #item="{ element: r }">
              <RoleCard :role="r" :progress="progress" :next-key="nextKey" />
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
