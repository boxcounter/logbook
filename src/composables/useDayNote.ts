import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppStore } from "../stores/useStore";
import { logError } from "../utils/errorLog";

export function useDayNote(store: AppStore) {
  const noteRef = ref<HTMLDivElement>();

  watch(
    () => store.today?.note,
    (n) => {
      if (noteRef.value && noteRef.value.textContent !== (n || "")) {
        noteRef.value.textContent = n || "";
      }
    },
    { immediate: true },
  );

  function onNotePaste(e: ClipboardEvent) {
    e.preventDefault();
    const text = e.clipboardData?.getData("text/plain") || "";
    const sel = window.getSelection();
    if (sel && sel.rangeCount > 0) {
      const range = sel.getRangeAt(0);
      range.deleteContents();
      range.insertNode(document.createTextNode(text));
      range.collapse(false);
    }
  }

  function onNoteInput() {
    if (noteRef.value && noteRef.value.innerHTML !== noteRef.value.textContent) {
      noteRef.value.textContent = noteRef.value.textContent || "";
    }
  }

  async function saveNote() {
    const text = noteRef.value?.textContent || "";
    try {
      await invoke("set_day_note", { rootPath: store.rootPath, date: store.currentDate, note: text });
    } catch (e) {
      logError("useDayNote.saveNote", e);
    }
  }

  let noteSnapshot = "";

  function onNoteFocus() {
    noteSnapshot = noteRef.value?.textContent || "";
  }

  function onNoteEsc(e: KeyboardEvent) {
    e.preventDefault();
    if (noteRef.value) noteRef.value.textContent = noteSnapshot;
    if (noteSnapshot === (store.today?.note ?? "")) return;
    noteRef.value?.blur();
  }

  function onNoteEnter(e: KeyboardEvent) {
    if (e.isComposing) return;
    e.preventDefault();
    noteRef.value?.blur();
  }

  return { noteRef, saveNote, onNotePaste, onNoteInput, onNoteFocus, onNoteEsc, onNoteEnter };
}
