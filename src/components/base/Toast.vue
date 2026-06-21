<script setup lang="ts">
defineProps<{
  show: boolean;
  message: string;
  undoLabel?: string;
}>();

defineEmits<{
  undo: [];
  dismiss: [];
}>();
</script>

<template>
  <Teleport to="body">
    <Transition name="toast">
      <div
        v-if="show"
        class="fixed bottom-[24px] left-1/2 -translate-x-1/2
               flex items-center gap-md
               bg-[var(--color-text-primary)] text-white
               px-xl py-md rounded-[var(--radius-card)]
               shadow-[var(--shadow-toast)] z-50 text-secondary"
      >
        <span>{{ message }}</span>
        <button
          v-if="undoLabel"
          class="font-semibold text-[#a5b4fc] hover:text-[#c7d2fe] cursor-pointer transition-colors"
          @click="$emit('undo')"
        >
          {{ undoLabel }}
        </button>
        <button
          class="text-[var(--color-text-secondary)] hover:text-white text-body leading-none cursor-pointer transition-colors"
          @click="$emit('dismiss')"
        >
          &times;
        </button>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.toast-enter-active { transition: all var(--motion-base) var(--ease-out); }
.toast-leave-active { transition: all var(--motion-base) var(--ease-in); }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translate(-50%, 1rem); }
</style>
