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
               flex items-center gap-[12px]
               bg-[var(--color-text-primary)] text-white
               px-[20px] py-[12px] rounded-[10px]
               shadow-[var(--shadow-toast)] z-50 text-[13px]"
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
          class="text-[var(--color-text-secondary)] hover:text-white text-[16px] leading-none cursor-pointer transition-colors"
          @click="$emit('dismiss')"
        >
          &times;
        </button>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.toast-enter-active { transition: all 0.2s ease-out; }
.toast-leave-active { transition: all 0.2s ease-in; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translate(-50%, 1rem); }
</style>
