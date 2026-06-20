<script setup lang="ts">
import { computed, ref } from "vue";
import { VueDraggable } from "vue-draggable-plus";
import GoalRow from "./GoalRow.vue";
import { formatDuration } from "../../utils/format";
import { goalLoggedMinutes } from "../../utils/commitments";
import type { CommitmentProgress, RoleRowModel, GoalRowModel } from "../../types";

const props = defineProps<{
  role: RoleRowModel;
  progress: CommitmentProgress[];
  nextKey: () => number;
  showErrors: boolean;
  dupRoles: Set<string>;
  dupGoals: Set<string>;
}>();
const emit = defineEmits<{ delete: [] }>();

const STEP = 5;
const MIN_ALLOC = 5;
const allocInput = ref<HTMLInputElement | null>(null);

function stepAlloc(delta: number) {
  props.role.allocation = Math.max(MIN_ALLOC, (props.role.allocation || 0) + delta);
}
// Allocation has two floors: the stepper buttons enforce a soft MIN_ALLOC (5h)
// floor, while direct typing only clamps to the hard >0 floor (1) the backend
// requires. Typed values of 1–4 are therefore legal.
function onAllocInput(e: Event) {
  const el = e.target as HTMLInputElement;
  const v = Math.floor(Number(el.value));
  const next = Number.isFinite(v) && v >= 1 ? v : 1;
  props.role.allocation = next;
  // Re-sync the DOM in case the clamped value equals the previous model value
  // (no model change → no Vue patch → the field would otherwise stay desynced,
  // e.g. clearing the field while already at the floor).
  if (el.value !== String(next)) el.value = String(next);
}

function addGoal() {
  props.role.goals.push({ name: "", origName: null, key: props.nextKey() });
}

function onGoalEnter(g: GoalRowModel) {
  const goals = props.role.goals;
  const gi = goals.findIndex(x => x.key === g.key);
  if (gi === -1) return;
  if (gi === goals.length - 1 && goals[gi].name.trim() === "") return;
  goals.splice(gi + 1, 0, { name: "", origName: null, key: props.nextKey() });
}

function goalLogged(origName: string | null): number {
  return goalLoggedMinutes(props.progress, origName);
}

const roleNameInvalid = computed(() => {
  const t = props.role.role.trim();
  return t === "" || props.dupRoles.has(t);
});
function goalNameInvalid(g: GoalRowModel): boolean {
  const t = g.name.trim();
  if (t === "") return goalLogged(g.origName) > 0;
  return props.dupGoals.has(t);
}

const roleSpent = computed(() => {
  if (!props.role.origRole) return 0;
  return props.progress.find(p => p.role === props.role.origRole)?.spent_minutes ?? 0;
});
const allocMinutes = computed(() => props.role.allocation * 60);
const isOver = computed(() => roleSpent.value > allocMinutes.value);
const barPct = computed(() => {
  const a = allocMinutes.value;
  if (a <= 0) return roleSpent.value > 0 ? 100 : 0;
  return Math.min(100, Math.round((roleSpent.value / a) * 100));
});
const overBy = computed(() => formatDuration(roleSpent.value - allocMinutes.value));

const confirming = ref(false);
const roleDeletable = computed(() => props.role.goals.every(g => goalLogged(g.origName) === 0));
function requestDelete() { if (roleDeletable.value) confirming.value = true; }
function confirmDelete() { confirming.value = false; emit("delete"); }
function cancelDelete() { confirming.value = false; }
function removeGoal(g: GoalRowModel) {
  // Authoritative guard: the button's native `disabled` is a UX affordance only
  // (programmatic clicks still fire the handler), so block logged-goal removal here.
  if (goalLogged(g.origName) > 0) return;
  const i = props.role.goals.findIndex(x => x.key === g.key);
  if (i >= 0) props.role.goals.splice(i, 1);
}
</script>

<template>
  <div class="bg-[var(--color-page-bg)] border border-[var(--color-divider)] rounded-[var(--radius-form-lg)] p-[16px] mb-[12px]" data-test="role-card">
    <div class="flex items-center gap-[8px]">
      <span data-test="drag-grip-role" class="drag-grip-role cursor-grab text-[var(--color-text-disabled)] select-none px-[2px]">⠿</span>
      <input
        v-model="role.role" data-test="role-name" placeholder="Role"
        @keydown.enter.exact.prevent="allocInput?.focus()"
        class="flex-1 px-[10px] py-[6px] border border-[var(--color-border-form)] rounded-[var(--radius-form)]
               text-[length:var(--app-text-base)] font-semibold text-[var(--color-text-primary)]
               bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
        :class="showErrors && roleNameInvalid ? 'border-[var(--color-danger)]' : ''"
      />
      <span class="inline-flex items-center gap-[5px]">
        <button
          data-test="alloc-dec" :disabled="role.allocation <= MIN_ALLOC"
          class="w-[24px] h-[26px] flex items-center justify-center border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                 text-[length:var(--app-text-base)] text-[var(--color-text-secondary)] bg-[var(--color-surface)]
                 hover:border-[var(--color-brand-solid)] hover:text-[var(--color-brand-link)]
                 disabled:text-[var(--color-text-disabled)] disabled:cursor-not-allowed disabled:hover:border-[var(--color-border-form)]
                 cursor-pointer transition-[border-color,color] duration-150"
          @click="stepAlloc(-STEP)"
        >&minus;</button>
        <input
          ref="allocInput"
          :value="role.allocation" type="number" data-test="alloc"
          class="w-[42px] text-center px-[4px] py-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                 text-[length:var(--app-text-base)] font-semibold text-[var(--color-text-primary)] mono
                 bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
          @input="onAllocInput($event)"
          @keydown.up.prevent="stepAlloc(STEP)"
          @keydown.down.prevent="stepAlloc(-STEP)"
        />
        <button
          data-test="alloc-inc"
          class="w-[24px] h-[26px] flex items-center justify-center border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                 text-[length:var(--app-text-base)] text-[var(--color-text-secondary)] bg-[var(--color-surface)]
                 hover:border-[var(--color-brand-solid)] hover:text-[var(--color-brand-link)]
                 cursor-pointer transition-[border-color,color] duration-150"
          @click="stepAlloc(STEP)"
        >+</button>
        <span class="text-[length:var(--app-text-xs-alt)] text-[var(--color-text-muted)]">h</span>
      </span>
      <span v-if="confirming" class="inline-flex items-center gap-[10px] text-[length:var(--app-text-xs)]">
        <span class="text-[var(--color-danger)] whitespace-nowrap">Delete role?</span>
        <button type="button" data-test="role-delete-confirm" class="font-semibold text-[var(--color-danger)] cursor-pointer" @click="confirmDelete">Delete</button>
        <button type="button" data-test="role-delete-cancel" class="font-semibold text-[var(--color-text-muted)] cursor-pointer" @click="cancelDelete">Cancel</button>
      </span>
      <button
        v-else
        data-test="role-delete" :disabled="!roleDeletable"
        :title="roleDeletable ? 'Delete role' : `Has logged time — can't delete this month`"
        class="text-[length:var(--app-text-xs-alt)] cursor-pointer px-[5px] py-[4px] transition-[color] duration-150
               text-[var(--color-text-muted)] hover:text-[var(--color-danger)]
               disabled:text-[var(--color-text-disabled)] disabled:cursor-not-allowed disabled:hover:text-[var(--color-text-disabled)]"
        @click="requestDelete"
      >Delete</button>
    </div>

    <div class="flex items-center gap-[8px] mt-[8px]">
      <div class="flex-1 h-[4px] bg-[var(--color-divider)] rounded-[2px] overflow-hidden">
        <div
          data-test="bar-fill"
          class="h-full rounded-[2px] transition-[width] duration-150"
          :class="isOver ? 'bg-[var(--color-warning)]' : 'bg-gradient-to-r from-[var(--color-brand-gradient-from)] to-[var(--color-brand-gradient-to)]'"
          :style="{ width: barPct + '%' }"
        ></div>
      </div>
      <span
        data-test="role-spent" class="text-[length:var(--app-text-xs-alt)] whitespace-nowrap"
        :class="isOver ? 'text-[var(--color-warning)] font-semibold' : 'text-[var(--color-text-muted)]'"
      >
        <span class="mono" :class="isOver ? '' : 'text-[var(--color-text-primary)] font-semibold'">{{ formatDuration(roleSpent) }}</span>
        <template v-if="isOver"> · over by {{ overBy }}</template>
        <template v-else> logged</template>
      </span>
    </div>

    <div class="mt-[12px]">
      <VueDraggable v-model="role.goals" handle=".drag-grip-goal" :animation="150" :force-fallback="true" :fallback-on-body="true" class="flex flex-col gap-[8px]">
        <GoalRow
          v-for="g in role.goals" :key="g.key"
          :goal="g" :logged="goalLogged(g.origName)" :invalid="showErrors && goalNameInvalid(g)"
          @remove="removeGoal(g)" @enter="onGoalEnter(g)"
        />
      </VueDraggable>
      <button
        data-test="add-goal"
        class="self-start mt-[8px] text-[length:var(--app-text-xs)] font-medium text-[var(--color-brand-link)] cursor-pointer hover:underline"
        @click="addGoal"
      >+ Add Goal</button>
    </div>
  </div>
</template>
