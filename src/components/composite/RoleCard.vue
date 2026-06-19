<script setup lang="ts">
import { computed } from "vue";
import draggable from "vuedraggable";
import GoalRow from "./GoalRow.vue";
import { formatDuration } from "../../utils/format";
import type { CommitmentProgress } from "../../types";

interface GoalRowModel { name: string; origName: string | null; key: number }
interface RoleRowModel { role: string; allocation: number; goals: GoalRowModel[]; origRole: string | null; key: number }

const props = defineProps<{
  role: RoleRowModel;
  progress: CommitmentProgress[];
  nextKey: () => number;
}>();

const STEP = 5;
const MIN_ALLOC = 5;

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

function goalLogged(origName: string | null): number {
  if (!origName) return 0;
  for (const p of props.progress) {
    const g = p.goals.find(x => x.name === origName);
    if (g) return g.spent_minutes;
  }
  return 0;
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
</script>

<template>
  <div class="bg-[var(--color-page-bg)] border border-[var(--color-divider)] rounded-[var(--radius-form-lg)] p-[16px] mb-[12px]" data-test="role-card">
    <div class="flex items-center gap-[8px]">
      <span data-test="drag-grip-role" class="drag-grip-role cursor-grab text-[var(--color-text-disabled)] select-none px-[2px]">⠿</span>
      <input
        v-model="role.role" data-test="role-name" placeholder="Role"
        class="flex-1 px-[10px] py-[6px] border border-[var(--color-border-form)] rounded-[var(--radius-form)]
               text-[length:var(--app-text-base)] font-semibold text-[var(--color-text-primary)]
               bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
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
      <draggable v-model="role.goals" item-key="key" handle=".drag-grip-goal" tag="div" class="flex flex-col gap-[8px]" :animation="150">
        <template #item="{ element: g, index: gi }">
          <GoalRow :goal="g" :logged="goalLogged(g.origName)" @remove="role.goals.splice(gi, 1)" />
        </template>
      </draggable>
      <button
        data-test="add-goal"
        class="self-start mt-[8px] text-[length:var(--app-text-xs)] font-medium text-[var(--color-brand-link)] cursor-pointer hover:underline"
        @click="addGoal"
      >+ Add Goal</button>
    </div>
  </div>
</template>
