import { watch, onUnmounted, type Ref } from "vue";

/**
 * Close a popover/editor on mousedown outside its root element.
 *
 * @param rootRef   The element whose bounds define "inside"
 * @param openRef   When true, the mousedown listener is active; set to false to close
 * @param opts.beforeClose  Optional guard: called before closing; return false to cancel
 */
export function useClickOutside(
  rootRef: Ref<HTMLElement | null>,
  openRef: Ref<boolean>,
  opts?: { beforeClose?: () => boolean },
) {
  function handler(e: MouseEvent) {
    if (rootRef.value && !rootRef.value.contains(e.target as Node)) {
      if (opts?.beforeClose && !opts.beforeClose()) return;
      openRef.value = false;
    }
  }

  watch(openRef, (open) => {
    if (open) document.addEventListener("mousedown", handler, true);
    else document.removeEventListener("mousedown", handler, true);
  }, { immediate: true });

  onUnmounted(() => {
    document.removeEventListener("mousedown", handler, true);
  });
}
