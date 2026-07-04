<script setup lang="ts">
defineProps<{
  message: string;
  rootPath: string;
}>();
</script>

<template>
  <div class="flex flex-col items-center justify-center min-h-screen gap-lg p-lg">
    <div class="max-w-[28rem] text-center text-secondary">
      <p class="mb-md">{{ message }}</p>
      <p class="text-micro text-gray">
        数据目录: {{ rootPath }}
      </p>
    </div>
  </div>
</template>
