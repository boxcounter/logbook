<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { VueDraggable } from "vue-draggable-plus";
import RoleCard from "./RoleCard.vue";
import type { Commitment, CommitmentProgress, RoleRowModel, GoalRowModel } from "../../types";
import { formatDuration } from "../../utils/format";
import { goalLoggedMinutes } from "../../utils/commitments";

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

const draft = ref<RoleRowModel[]>([]);
const error = ref("");
const saving = ref(false);
const showErrors = ref(false);
const showDiscard = ref(false);

function buildDraft() {
  draft.value = props.commitments.map((c): RoleRowModel => ({
    role: c.role, allocation: c.allocation, origRole: c.role, key: nextKey(),
    goals: c.goals.map((g): GoalRowModel => ({ name: g, origName: g, key: nextKey() })),
  }));
  error.value = "";
  showErrors.value = false;
  showDiscard.value = false;
}
watch(() => props.open, (o) => { if (o) buildDraft(); }, { immediate: true });

function toCommitments(rows: RoleRowModel[]): Commitment[] {
  return rows.map(r => ({
    role: r.role.trim(),
    allocation: r.allocation,
    goals: r.goals.map(g => g.name.trim()).filter(n => n !== ""),
  }));
}

function addRole() {
  draft.value.push({ role: "", allocation: NEW_ROLE_ALLOC, origRole: null, key: nextKey(), goals: [{ name: "", origName: null, key: nextKey() }] });
}

function removeRole(r: RoleRowModel) {
  const i = draft.value.findIndex(x => x.key === r.key);
  if (i >= 0) draft.value.splice(i, 1);
}

const monthLabel = computed(() =>
  new Date(props.selectedYear, props.selectedMonth - 1, 1).toLocaleDateString("en-US", { month: "long", year: "numeric" })
);

const committedHours = computed(() => draft.value.reduce((s, r) => s + (r.allocation || 0), 0));
const loggedTotal = computed(() => props.progress.reduce((s, p) => s + p.spent_minutes, 0));

function dupSet(names: string[]): Set<string> {
  const seen = new Set<string>(), dup = new Set<string>();
  for (const n of names) { const t = n.trim(); if (!t) continue; if (seen.has(t)) dup.add(t); else seen.add(t); }
  return dup;
}
const dupRoles = computed(() => dupSet(draft.value.map(r => r.role)));
const dupGoals = computed(() => dupSet(draft.value.flatMap(r => r.goals.map(g => g.name))));

function validate(): string | null {
  if (draft.value.length === 0) return "At least one role is required";
  for (const r of draft.value) {
    if (r.role.trim() === "") return "Role name is required";
    if (dupRoles.value.has(r.role.trim())) return "Duplicate role name — each role must be unique";
    for (const g of r.goals) {
      if (g.name.trim() === "" && goalLoggedMinutes(props.progress, g.origName) > 0) return "Goal with logged time can't be empty";
    }
  }
  if (dupGoals.value.size > 0) return "Duplicate goal name — each goal must be unique";
  return null;
}

const isDirty = computed(() =>
  JSON.stringify(toCommitments(draft.value)) !==
  JSON.stringify(props.commitments.map(c => ({
    role: c.role.trim(),
    allocation: c.allocation,
    goals: c.goals.map(g => g.trim()).filter(n => n !== ""),
  })))
);
function requestClose() { if (isDirty.value) { showDiscard.value = true; return; } emit("close"); }
function confirmDiscard() { showDiscard.value = false; emit("close"); }
function keepEditing() { showDiscard.value = false; }

async function save() {
  const msg = validate();
  if (msg) { showErrors.value = true; error.value = msg; return; }
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
function onModalKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") { e.preventDefault(); save(); return; }
  if (e.key === "Escape") { e.preventDefault(); requestClose(); }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      data-test="overlay" tabindex="-1"
      @keydown="onModalKeydown"
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
          <VueDraggable v-model="draft" handle=".drag-grip-role" :animation="150">
            <RoleCard
              v-for="r in draft" :key="r.key"
              :role="r" :progress="progress" :next-key="nextKey"
              :show-errors="showErrors" :dup-roles="dupRoles" :dup-goals="dupGoals"
              @delete="removeRole(r)"
            />
          </VueDraggable>

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
            @click="requestClose"
          >Cancel</button>
          <button
            data-test="save" :disabled="saving"
            class="text-[length:var(--app-text-sm)] font-semibold text-white bg-[var(--color-brand-solid)] hover:bg-[var(--color-brand-link)] rounded-[var(--radius-form)] px-[14px] py-[6px] cursor-pointer disabled:opacity-50"
            @click="save"
          >{{ saving ? "Saving…" : "Save" }}</button>
        </div>

        <div v-if="showDiscard" data-test="discard-confirm" class="absolute inset-0 flex items-center justify-center bg-black/10">
          <div class="bg-[var(--color-surface)] border border-[var(--color-border-form)] rounded-[var(--radius-card)] shadow-[var(--shadow-toast)] p-[16px] max-w-[300px]">
            <div class="text-[length:var(--app-text-base)] font-semibold text-[var(--color-text-primary)] mb-[4px]">Discard changes?</div>
            <div class="text-[length:var(--app-text-xs)] text-[var(--color-text-secondary)] mb-[14px]">Your edits to this month's commitments won't be saved.</div>
            <div class="flex justify-end gap-[8px]">
              <button class="text-[length:var(--app-text-xs)] font-semibold text-[var(--color-text-muted)] px-[12px] py-[6px] cursor-pointer" @click="keepEditing">Keep editing</button>
              <button data-test="discard-yes" class="text-[length:var(--app-text-xs)] font-semibold text-[var(--color-danger)] bg-red-50 rounded-[var(--radius-form)] px-[12px] py-[6px] cursor-pointer" @click="confirmDiscard">Discard</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
