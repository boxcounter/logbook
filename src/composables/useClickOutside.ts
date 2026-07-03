import { watch, onUnmounted, type Ref } from "vue";

export function useClickOutside(
  rootRef: Ref<HTMLElement | null>,
  openRef: Ref<boolean>,
) {
  function handler(e: MouseEvent) {
    if (rootRef.value && !rootRef.value.contains(e.target as Node)) {
      openRef.value = false;
    }
  }

  watch(openRef, (open) => {
    if (open) document.addEventListener("mousedown", handler, true);
    else document.removeEventListener("mousedown", handler, true);
  });

  onUnmounted(() => document.removeEventListener("mousedown", handler, true));
}
