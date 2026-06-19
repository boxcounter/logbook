<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Commitment } from '../../types';
import AppButton from '../base/AppButton.vue';

const props = defineProps<{
  commitments: Commitment[];
  rootPath: string;
  selectedYear: number;
  selectedMonth: number;
}>();

const emit = defineEmits<{
  saved: [];
  cancel: [];
}>();

const editingCommitments = ref<Commitment[]>(
  JSON.parse(JSON.stringify(props.commitments))
);
const error = ref('');
const saving = ref(false);

function addRole() {
  editingCommitments.value.push({ role: '', allocation: 0, goals: [] });
}
function removeRole(index: number) {
  if (editingCommitments.value.length > 1) editingCommitments.value.splice(index, 1);
}
function addGoal(roleIndex: number) {
  editingCommitments.value[roleIndex].goals.push('');
}
function removeGoal(roleIndex: number, goalIndex: number) {
  editingCommitments.value[roleIndex].goals.splice(goalIndex, 1);
}

function preValidate(): string | null {
  if (editingCommitments.value.length === 0) return 'At least one role is required';
  for (const c of editingCommitments.value) {
    if (!c.role.trim()) return 'Role name cannot be empty';
    if (!c.allocation || c.allocation <= 0) return `Allocation for '${c.role || 'unnamed'}' must be > 0`;
    for (const g of c.goals) {
      if (!g.trim()) return 'Goal name cannot be empty';
    }
  }
  return null;
}

async function save() {
  const err = preValidate();
  if (err) { error.value = err; return; }
  saving.value = true;
  error.value = '';
  try {
    await invoke('set_commitments', {
      rootPath: props.rootPath,
      year: props.selectedYear,
      month: props.selectedMonth,
      commitments: editingCommitments.value.map(c => ({
        role: c.role.trim(),
        allocation: c.allocation,
        goals: c.goals.map(g => g.trim()).filter(g => g !== ''),
      })),
    });
    emit('saved');
  } catch (e) {
    error.value = typeof e === 'string' ? e : String(e);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div>
    <div v-if="error" class="mb-[12px] p-[8px] bg-red-50 border border-red-200 rounded-[var(--radius-form)] text-[length:var(--app-text-sm)] text-[var(--color-danger)]">
      {{ error }}
    </div>

    <div v-for="(c, ri) in editingCommitments" :key="ri" class="mb-[16px] last:mb-0">
      <div class="flex items-center gap-[8px] mb-[10px]">
        <input
          v-model="c.role"
          placeholder="Role"
          class="w-[130px] px-[12px] py-[8px] border-2 border-[var(--color-border-form)]
                 rounded-[var(--radius-form)] text-[length:var(--app-text-base)]
                 bg-[var(--color-surface)] text-[var(--color-text-primary)]
                 outline-none transition-all duration-200
                 focus:border-[var(--color-brand-solid)] focus:bg-[#fafaff]
                 focus:shadow-[var(--shadow-focus-ring)]"
        />
        <span class="text-[length:var(--app-text-sm)] text-[var(--color-text-secondary)]">Alloc:</span>
        <input
          v-model.number="c.allocation"
          type="number" min="1" placeholder="hours"
          class="w-[56px] text-center px-[12px] py-[8px] border-2 border-[var(--color-border-form)]
                 rounded-[var(--radius-form)] text-[length:var(--app-text-base)]
                 bg-[var(--color-surface)] text-[var(--color-text-primary)]
                 outline-none transition-all duration-200
                 focus:border-[var(--color-brand-solid)] focus:bg-[#fafaff]
                 focus:shadow-[var(--shadow-focus-ring)]"
        />
        <span class="text-[length:var(--app-text-sm)] text-[var(--color-text-secondary)]">h</span>
        <button
          v-if="editingCommitments.length > 1"
          class="ml-auto text-[length:var(--app-text-sm)] text-[var(--color-text-secondary)]
                 hover:text-[var(--color-danger)] cursor-pointer transition-colors"
          @click="removeRole(ri)"
        >
          Delete Role
        </button>
      </div>

      <div class="ml-[20px] flex flex-col gap-[8px]">
        <div v-for="(_g, gi) in c.goals" :key="gi" class="flex items-center gap-[8px]">
          <input
            v-model="c.goals[gi]"
            placeholder="Goal name"
            class="flex-1 px-[12px] py-[8px] border-2 border-[var(--color-border-form)]
                   rounded-[var(--radius-form)] text-[length:var(--app-text-base)]
                   bg-[var(--color-surface)] text-[var(--color-text-primary)]
                   outline-none transition-all duration-200
                   focus:border-[var(--color-brand-solid)] focus:bg-[#fafaff]
                   focus:shadow-[var(--shadow-focus-ring)]"
          />
          <button
            class="text-[var(--color-text-secondary)] hover:text-[var(--color-danger)]
                   cursor-pointer text-[14px] transition-colors"
            @click="removeGoal(ri, gi)"
          >
            &times;
          </button>
        </div>
        <button
          class="text-[length:var(--app-text-sm)] text-[var(--color-brand-link)] font-medium cursor-pointer hover:underline"
          @click="addGoal(ri)"
        >
          + Add Goal
        </button>
      </div>

      <hr v-if="ri < editingCommitments.length - 1" class="my-[12px] border-[var(--color-divider)]" />
    </div>

    <button
      class="text-[length:var(--app-text-sm)] text-[var(--color-brand-link)] font-medium cursor-pointer hover:underline block mb-[16px]"
      @click="addRole"
    >
      + Add Role
    </button>

    <div class="flex justify-end gap-[8px] pt-[12px] border-t border-[var(--color-divider)]">
      <AppButton variant="secondary" size="sm" @click="$emit('cancel')">Cancel</AppButton>
      <AppButton size="sm" :disabled="saving" @click="save">
        {{ saving ? 'Saving...' : 'Save' }}
      </AppButton>
    </div>
  </div>
</template>
