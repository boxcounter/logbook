<script setup lang="ts">
// TODO(#40): CommitmentsModal and DimensionEditorModal share ~150 lines of modal
// skeleton (Teleport, overlay, discard confirmation, save/cancel footer, isDirty
// via JSON.stringify, keyboard esc). Extract a BaseModal or useModal composable.
import { ref, computed, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { VueDraggable } from "vue-draggable-plus";
import type { Commitment, CommitmentProgress, RoleRowModel, GoalRowModel } from "../../types";
import { formatDurationCompact } from "../../utils/format";
import { logError } from "../../utils/errorLog";
import { isIMEEvent } from "../../utils/ime";
import { goalLoggedMinutes } from "../../utils/commitments";

const props = defineProps<{
  open: boolean;
  commitments: Commitment[];
  progress: CommitmentProgress[];
  rootPath: string;
  selectedYear: number;
  selectedMonth: number;
}>();

const emit = defineEmits<{ saved: [Commitment[]]; close: [] }>();

const NEW_ROLE_ALLOC = 5;
let _key = 0;
const nextKey = () => ++_key;

const draft = ref<RoleRowModel[]>([]);
const selectedIndex = ref(0);
const error = ref("");
const saving = ref(false);
const showErrors = ref(false);
const showDiscard = ref(false);
const confirmingDelete = ref(false);
const overlayRef = ref<HTMLElement>();
const roleNameInputRef = ref<HTMLInputElement>();

function buildDraft() {
  draft.value = props.commitments.map((c): RoleRowModel => ({
    role: c.role, allocation: c.allocation, origRole: c.role, key: nextKey(),
    goals: c.goals.map((g): GoalRowModel => ({ name: g, origName: g, key: nextKey() })),
  }));
  selectedIndex.value = 0;
  error.value = "";
  showErrors.value = false;
  showDiscard.value = false;
  confirmingDelete.value = false;
}
watch(() => props.open, (o) => {
  if (!o) return;
  buildDraft();
  nextTick(() => overlayRef.value?.focus());
}, { immediate: true });

function toCommitments(rows: RoleRowModel[]): Commitment[] {
  return rows.map(r => ({
    role: r.role.trim(),
    allocation: r.allocation,
    goals: r.goals.map(g => g.name.trim()).filter(n => n !== ""),
  }));
}

const monthLabel = computed(() =>
  new Date(props.selectedYear, props.selectedMonth - 1, 1)
    .toLocaleDateString("en-US", { month: "long", year: "numeric" })
);

const committedHours = computed(() => draft.value.reduce((s, r) => s + (r.allocation || 0), 0));
const loggedTotal = computed(() =>
  props.progress.reduce((s, p) => s + p.goal_spent_minutes + p.general_spent_minutes, 0)
);

const selectedRole = computed(() => draft.value[selectedIndex.value] ?? null);

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
  const saved = toCommitments(draft.value);
  try {
    await invoke("set_commitments", {
      rootPath: props.rootPath,
      year: props.selectedYear,
      month: props.selectedMonth,
      commitments: saved,
    });
    emit("saved", saved);
    emit("close");
  } catch (e) {
    logError("CommitmentsModal.save", e);
    error.value = typeof e === "string" ? e : String(e);
  } finally {
    saving.value = false;
  }
}

function onModalKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") { e.preventDefault(); save(); return; }
  if (e.key === "Escape") { e.preventDefault(); requestClose(); }
}

// ── Left panel ──────────────────────────────────────────────────

function addRole() {
  draft.value.push({
    role: "", allocation: NEW_ROLE_ALLOC, origRole: null, key: nextKey(),
    goals: [{ name: "", origName: null, key: nextKey() }],
  });
  selectedIndex.value = draft.value.length - 1;
  confirmingDelete.value = false;
  nextTick(() => roleNameInputRef.value?.focus());
}

function selectRole(index: number) { selectedIndex.value = index; confirmingDelete.value = false; }

function navigateRole(delta: 1 | -1) {
  if (draft.value.length <= 1) return;
  selectedIndex.value = (selectedIndex.value + delta + draft.value.length) % draft.value.length;
  confirmingDelete.value = false;
}

function removeRole(r: RoleRowModel) {
  const i = draft.value.findIndex(x => x.key === r.key);
  if (i < 0) return;
  draft.value.splice(i, 1);
  if (selectedIndex.value >= draft.value.length) {
    selectedIndex.value = Math.max(0, draft.value.length - 1);
  }
  confirmingDelete.value = false;
}

function roleSpentMinutes(r: RoleRowModel): number {
  if (!r.origRole) return 0;
  const p = props.progress.find(x => x.role === r.origRole);
  return (p?.goal_spent_minutes ?? 0) + (p?.general_spent_minutes ?? 0);
}

// ── Right panel (selected role) ──────────────────────────────────

const STEP = 5;
const MIN_ALLOC = 5;

function stepAlloc(delta: number) {
  if (!selectedRole.value) return;
  selectedRole.value.allocation = Math.max(MIN_ALLOC, (selectedRole.value.allocation || 0) + delta);
}

function onAllocInput(e: Event) {
  const el = e.target as HTMLInputElement;
  const v = Math.floor(Number(el.value));
  const next = Number.isFinite(v) && v >= 1 ? v : 1;
  if (selectedRole.value) selectedRole.value.allocation = next;
  if (el.value !== String(next)) el.value = String(next);
}

function addGoal() {
  if (!selectedRole.value) return;
  selectedRole.value.goals.push({ name: "", origName: null, key: nextKey() });
}

function onGoalEnter(g: GoalRowModel, e?: KeyboardEvent) {
  if (e && isIMEEvent(e)) return;
  if (!selectedRole.value) return;
  const goals = selectedRole.value.goals;
  const gi = goals.findIndex(x => x.key === g.key);
  if (gi === -1) return;
  if (gi === goals.length - 1 && goals[gi].name.trim() === "") return;
  goals.splice(gi + 1, 0, { name: "", origName: null, key: nextKey() });
  nextTick(() => {
    const inputs = document.querySelectorAll('[data-test="goal-name"]');
    (inputs[gi + 1] as HTMLInputElement)?.focus();
  });
}

function goalLogged(origName: string | null): number {
  return goalLoggedMinutes(props.progress, origName);
}

function removeGoal(g: GoalRowModel) {
  if (goalLogged(g.origName) > 0) return;
  if (!selectedRole.value) return;
  const i = selectedRole.value.goals.findIndex(x => x.key === g.key);
  if (i >= 0) selectedRole.value.goals.splice(i, 1);
}

const roleNameInvalid = computed(() => {
  if (!selectedRole.value) return false;
  const t = selectedRole.value.role.trim();
  return t === "" || dupRoles.value.has(t);
});

function goalNameInvalid(g: GoalRowModel): boolean {
  const t = g.name.trim();
  if (t === "") return goalLogged(g.origName) > 0;
  return dupGoals.value.has(t);
}

const roleSpent = computed(() => {
  if (!selectedRole.value?.origRole) return 0;
  const p = props.progress.find(x => x.role === selectedRole.value!.origRole);
  return (p?.goal_spent_minutes ?? 0) + (p?.general_spent_minutes ?? 0);
});

const allocMinutes = computed(() => (selectedRole.value?.allocation ?? 0) * 60);
const isOver = computed(() => roleSpent.value > allocMinutes.value);
const barPct = computed(() => {
  const a = allocMinutes.value;
  if (a <= 0) return roleSpent.value > 0 ? 100 : 0;
  return Math.min(100, Math.round((roleSpent.value / a) * 100));
});
const overBy = computed(() => formatDurationCompact(roleSpent.value - allocMinutes.value));

const roleDeletable = computed(() => {
  if (!selectedRole.value) return false;
  return selectedRole.value.goals.every(g => goalLogged(g.origName) === 0);
});

function requestDelete() { if (roleDeletable.value) confirmingDelete.value = true; }
function cancelDelete() { confirmingDelete.value = false; }

const leftRoleBar = (r: RoleRowModel): number => {
  const a = r.allocation * 60;
  const s = roleSpentMinutes(r);
  if (a <= 0) return s > 0 ? 100 : 0;
  return Math.min(100, Math.round((s / a) * 100));
};
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      ref="overlayRef"
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
        <div class="flex justify-between items-start px-2xl pt-xl pb-lg border-b border-[var(--color-divider)]">
          <div>
            <div class="text-title font-bold text-[var(--color-text-primary)] tracking-[-0.3px]">Edit Commitments</div>
            <div class="text-secondary text-[var(--color-text-muted)] mt-2xs">{{ monthLabel }}</div>
          </div>
          <div class="text-right text-secondary text-[var(--color-text-muted)]">
            <div>Committed <span data-test="committed" class="mono font-bold text-[var(--color-brand-link)]">{{ committedHours }}h</span></div>
            <div>Logged <span data-test="logged" class="mono font-semibold text-[var(--color-text-primary)]">{{ formatDurationCompact(loggedTotal) }}</span></div>
          </div>
        </div>

        <!-- Body: two-column -->
        <div class="flex-1 flex min-h-0">
          <!-- Left panel: role list -->
          <div class="w-[210px] flex-shrink-0 border-r border-[var(--color-divider)] bg-[var(--color-surface-muted)] p-md flex flex-col">
            <VueDraggable
              v-model="draft"
              handle=".drag-grip-role"
              :animation="150"
              class="flex-1 space-y-2xs"
            >
              <div
                v-for="(r, index) in draft" :key="r.key"
                class="flex items-center gap-sm px-sm py-xs rounded-[var(--radius-form-lg)] cursor-pointer border"
                :class="index === selectedIndex
                  ? 'bg-[var(--color-brand-soft-bg)] border-[var(--color-brand-link)]'
                  : 'border-transparent'"
                :data-test="index === selectedIndex ? 'role-row-selected' : 'role-row'"
                tabindex="0"
                @click="selectRole(index)"
                @keydown.up.prevent="navigateRole(-1)"
                @keydown.down.prevent="navigateRole(1)"
              >
                <span data-test="drag-grip-role" class="drag-grip-role cursor-grab text-[var(--color-text-disabled)] select-none px-2xs">⠿</span>
                <div
                  class="w-[3px] h-[16px] rounded-[1px] flex-shrink-0"
                  :style="{
                    background: leftRoleBar(r) >= 100
                      ? 'var(--color-warning)'
                      : `linear-gradient(to right, var(--color-brand-gradient-from), var(--color-brand-gradient-to))`,
                  }"
                ></div>
                <span class="text-body text-[var(--color-text-primary)] flex-1 truncate">{{ r.role || 'New role' }}</span>
                <span class="text-micro text-[var(--color-text-muted)]">
                  {{ formatDurationCompact(roleSpentMinutes(r)) }} / {{ r.allocation }}h
                </span>
              </div>
            </VueDraggable>

            <button
              data-test="add-role"
              class="text-secondary font-semibold text-[var(--color-brand-link)] mt-sm text-left cursor-pointer"
              @click="addRole"
            >+ Add Role</button>
          </div>

          <!-- Right panel -->
          <template v-if="selectedRole">
            <div class="flex-1 flex flex-col min-h-0">
              <div class="flex-1 overflow-y-auto px-2xl py-xl">
                <!-- Role name + delete -->
                <div class="flex items-center gap-sm">
                  <input
                    ref="roleNameInputRef"
                    v-model="selectedRole.role"
                    data-test="role-name" placeholder="Role"
                    class="flex-1 text-title font-semibold text-[var(--color-text-primary)] bg-transparent
                           border-0 border-b-2 border-[var(--color-border-form)] rounded-none
                           px-0 pb-xs outline-none focus:border-[var(--color-brand-solid)]"
                    :class="showErrors && roleNameInvalid ? 'border-[var(--color-danger)]' : ''"
                  />
                  <span v-if="confirmingDelete" class="inline-flex items-center gap-sm text-secondary">
                    <span class="text-[var(--color-danger)] whitespace-nowrap">Delete role?</span>
                    <button type="button" data-test="role-delete-confirm" class="font-semibold text-[var(--color-danger)] cursor-pointer" @click="removeRole(selectedRole)">Delete</button>
                    <button type="button" data-test="role-delete-cancel" class="font-semibold text-[var(--color-text-muted)] cursor-pointer" @click="cancelDelete">Cancel</button>
                  </span>
                  <button
                    v-else
                    data-test="role-delete" :disabled="!roleDeletable"
                    :title="roleDeletable ? 'Delete role' : `Has logged time — can't delete this month`"
                    class="text-secondary cursor-pointer px-xs py-xs transition-[color] duration-[var(--motion-fast)]
                           text-[var(--color-text-muted)] hover:text-[var(--color-danger)]
                           disabled:text-[var(--color-text-disabled)] disabled:cursor-not-allowed disabled:hover:text-[var(--color-text-disabled)]"
                    @click="requestDelete"
                  >Delete</button>
                </div>

                <!-- Allocation stepper -->
                <div class="flex items-center gap-sm mt-md">
                  <span class="inline-flex items-center gap-xs">
                    <button
                      data-test="alloc-dec" :disabled="selectedRole.allocation <= MIN_ALLOC"
                      class="w-[24px] h-[26px] flex items-center justify-center border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-body text-[var(--color-text-secondary)] bg-[var(--color-surface)]
                             hover:border-[var(--color-brand-solid)] hover:text-[var(--color-brand-link)]
                             disabled:text-[var(--color-text-disabled)] disabled:cursor-not-allowed disabled:hover:border-[var(--color-border-form)]
                             cursor-pointer transition-[border-color,color] duration-[var(--motion-fast)]"
                      @click="stepAlloc(-STEP)"
                    >&minus;</button>
                    <input
                      :value="selectedRole.allocation" type="number" data-test="alloc"
                      class="w-[52px] text-center px-xs py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-body font-semibold text-[var(--color-text-primary)] mono
                             bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                      @input="onAllocInput($event)"
                      @keydown.up.prevent="stepAlloc(STEP)"
                      @keydown.down.prevent="stepAlloc(-STEP)"
                    />
                    <button
                      data-test="alloc-inc"
                      class="w-[24px] h-[26px] flex items-center justify-center border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-body text-[var(--color-text-secondary)] bg-[var(--color-surface)]
                             hover:border-[var(--color-brand-solid)] hover:text-[var(--color-brand-link)]
                             cursor-pointer transition-[border-color,color] duration-[var(--motion-fast)]"
                      @click="stepAlloc(STEP)"
                    >+</button>
                    <span class="text-secondary text-[var(--color-text-muted)]">h</span>
                  </span>
                </div>

                <!-- Progress bar -->
                <div class="flex items-center gap-sm mt-sm">
                  <div class="flex-1 h-[4px] bg-[var(--color-divider)] rounded-full overflow-hidden">
                    <div
                      data-test="bar-fill"
                      class="h-full rounded-full transition-[width] duration-[var(--motion-fast)]"
                      :class="isOver ? 'bg-[var(--color-warning)]' : 'bg-gradient-to-r from-[var(--color-brand-gradient-from)] to-[var(--color-brand-gradient-to)]'"
                      :style="{ width: barPct + '%' }"
                    ></div>
                  </div>
                  <span
                    data-test="role-spent" class="text-secondary whitespace-nowrap"
                    :class="isOver ? 'text-[var(--color-warning)] font-semibold' : 'text-[var(--color-text-muted)]'"
                  >
                    <span class="mono" :class="isOver ? '' : 'text-[var(--color-text-primary)] font-semibold'">{{ formatDurationCompact(roleSpent) }}</span>
                    <template v-if="isOver"> · over by {{ overBy }}</template>
                    <template v-else> logged</template>
                  </span>
                </div>

                <div class="border-t border-[var(--color-divider)] my-lg"></div>

                <!-- Goal list -->
                <VueDraggable v-model="selectedRole.goals" handle=".drag-grip-goal" :animation="150" class="flex flex-col gap-sm">
                  <div v-for="g in selectedRole.goals" :key="g.key" class="flex items-center gap-sm">
                    <span data-test="drag-grip-goal" class="drag-grip-goal cursor-grab text-[var(--color-text-disabled)] select-none px-2xs">⠿</span>
                    <input
                      v-model="g.name" data-test="goal-name" placeholder="Goal name"
                      @keydown.enter.exact.prevent="onGoalEnter(g, $event)"
                      class="flex-1 px-sm py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                             text-secondary text-[var(--color-text-secondary)]
                             bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                      :class="showErrors && goalNameInvalid(g) ? 'border-[var(--color-danger)]' : ''"
                    />
                    <span
                      data-test="goal-logged"
                      class="text-secondary mono whitespace-nowrap min-w-[46px] text-right"
                      :class="goalLogged(g.origName) > 0 ? 'text-[var(--color-text-muted)]' : 'text-[var(--color-text-disabled)]'"
                    >{{ goalLogged(g.origName) > 0 ? formatDurationCompact(goalLogged(g.origName)) : "0" }}</span>
                    <button
                      data-test="goal-remove" :disabled="goalLogged(g.origName) > 0"
                      :title="goalLogged(g.origName) > 0 ? `${formatDurationCompact(goalLogged(g.origName))} logged — rename instead` : 'Remove goal'"
                      class="text-body cursor-pointer px-xs transition-[color] duration-[var(--motion-fast)]
                             text-[var(--color-text-disabled)] hover:text-[var(--color-danger)]
                             disabled:text-[var(--color-divider)] disabled:cursor-not-allowed disabled:hover:text-[var(--color-divider)]"
                      @click="removeGoal(g)"
                    >&times;</button>
                  </div>
                </VueDraggable>
                <button
                  data-test="add-goal"
                  class="self-start mt-sm text-secondary font-medium text-[var(--color-brand-link)] cursor-pointer hover:underline"
                  @click="addGoal"
                >+ Add Goal</button>
              </div>
            </div>
          </template>
          <template v-else>
            <div class="flex-1 flex items-center justify-center px-2xl">
              <p class="text-secondary text-[var(--color-text-muted)]">No roles yet. Add a role to get started.</p>
            </div>
          </template>
        </div>

        <!-- Footer -->
        <div v-if="error" class="px-2xl pb-sm text-secondary text-[var(--color-danger)]" data-test="save-error">{{ error }}</div>
        <div class="flex justify-end gap-sm px-2xl py-lg border-t border-[var(--color-divider)]">
          <button
            data-test="cancel"
            class="text-secondary font-semibold text-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer"
            @click="requestClose"
          >Cancel</button>
          <button
            data-test="save" :disabled="saving"
            class="text-secondary font-semibold text-white bg-[var(--color-brand-solid)] hover:bg-[var(--color-brand-link)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer disabled:opacity-50"
            @click="save"
          >{{ saving ? "Saving…" : "Save" }}</button>
        </div>

        <!-- Discard confirmation overlay -->
        <div v-if="showDiscard" data-test="discard-confirm" class="absolute inset-0 flex items-center justify-center bg-black/10">
          <div class="bg-[var(--color-surface)] border border-[var(--color-border-form)] rounded-[var(--radius-card)] shadow-[var(--shadow-toast)] p-lg max-w-[300px]">
            <div class="text-body font-semibold text-[var(--color-text-primary)] mb-xs">Discard changes?</div>
            <div class="text-secondary text-[var(--color-text-secondary)] mb-md">Your edits to this month's commitments won't be saved.</div>
            <div class="flex justify-end gap-sm">
              <button class="text-secondary font-semibold text-[var(--color-text-muted)] px-md py-sm cursor-pointer" @click="keepEditing">Keep editing</button>
              <button data-test="discard-yes" class="text-secondary font-semibold text-[var(--color-danger)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer" @click="confirmDiscard">Discard</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
