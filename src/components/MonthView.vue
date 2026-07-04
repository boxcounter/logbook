<!-- src/components/MonthView.vue -->
<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, nextTick } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useStore } from "../stores/useStore";
import { useDayNote } from "../composables/useDayNote";
import { useFileActions } from "../composables/useFileActions";
import { useMonthData } from "../composables/useMonthData";
import { useEntryActions } from "../composables/useEntryActions";
import HeatmapCalendar from "./HeatmapCalendar.vue";
import CommitmentsPanel from "./CommitmentsPanel.vue";
import DayHeader from "./DayHeader.vue";
import EntryList from "./EntryList.vue";
import EntryComposer from "./EntryComposer.vue";
import DimensionEditorModal from "./composite/DimensionEditorModal.vue";
import type { Dimension } from "../types";
import { logError, logInfo } from "../utils/errorLog";
import { yearMonthFromDate, parseDate, addDays, formatDate } from "../utils/dates";
import ConfigErrorBanner from "./ConfigErrorBanner.vue";

const store = useStore();
const inputRef = ref<InstanceType<typeof EntryComposer> | null>(null);

// Dimension editor modal
const showDimEditor = ref(false);

function openDimEditor() { showDimEditor.value = true; }

function onDimensionsSaved(dims: Dimension[]) {
  store.dimensions = dims;
  store.usingDefaultDimensions = false;
  showDimEditor.value = false;
}

const selectedYear = computed(() => yearMonthFromDate(store.currentDate).year);
const selectedMonth = computed(() => yearMonthFromDate(store.currentDate).month);

const isSelectedToday = computed(() => store.currentDate === formatDate(new Date()));

const dayEntries = computed(() => store.today?.entries || []);
const dayTotalMinutes = computed(() => dayEntries.value.reduce((s, e) => s + e.duration, 0));

const dayTitle = computed(() => {
  const d = parseDate(store.currentDate);
  return d.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" });
});

const { noteRef, saveNote, onNotePaste, onNoteInput, onNoteFocus, onNoteEsc, onNoteEnter } = useDayNote(store);
const { dayFilePath, displayPath, revealDayFile, copyFilePath, copiedFeedback } = useFileActions(store);
const {
  loadMonth,
  onCommitmentsSaved,
  handleSelectDay,
  handleNavigate,
  handleRequestMonths,
} = useMonthData(store, guardUnsaved);
const { handleSubmit, handleUpdateEntry, handleUpdateDimensions, handleDeleteEntry, justAddedId } = useEntryActions(store, inputRef);

function guardUnsaved(): boolean {
  if (inputRef.value?.hasUnsavedContent?.()) {
    return confirm("Discard unsaved entry?");
  }
  return true;
}

// ---- Keyboard month navigation (⌘[ / ⌘]) ----
function shiftMonth(delta: number) {
  if (!guardUnsaved()) return;
  let m = selectedMonth.value + delta;
  let y = selectedYear.value;
  if (m < 1) { m = 12; y--; } else if (m > 12) { m = 1; y++; }
  loadMonth(y, m);
}
function shiftDay(delta: number) {
  if (delta > 0 && isSelectedToday.value) return; // never navigate into the future
  if (!guardUnsaved()) return;
  const next = addDays(store.currentDate, delta);
  if (next in store.monthEntries) {
    handleSelectDay(next);
  } else {
    const { year, month } = yearMonthFromDate(next);
    loadMonth(year, month, parseInt(next.slice(8, 10), 10));
  }
}
// Jump back to today (⌘T) and focus the entry input so typing can start at once.
async function goToToday() {
  const t = formatDate(new Date());
  if (store.currentDate !== t) {
    if (t in store.monthEntries) await handleSelectDay(t);
    else {
      const { year, month } = yearMonthFromDate(t);
      await loadMonth(year, month, parseInt(t.slice(8, 10), 10));
    }
  }
  await nextTick(); // wait for EntryComposer (today-only) to render before focusing
  inputRef.value?.focusInput();
}
function onGlobalKeydown(e: KeyboardEvent) {
  if (!(e.metaKey || e.ctrlKey)) return;
  if (e.key === "[") {
    e.preventDefault();
    e.shiftKey ? shiftMonth(-1) : shiftDay(-1);
  } else if (e.key === "]") {
    e.preventDefault();
    e.shiftKey ? shiftMonth(1) : shiftDay(1);
  } else if (e.key === "t" || e.key === "T") {
    e.preventDefault();
    goToToday();
  }
}

onMounted(async () => {
  window.addEventListener("keydown", onGlobalKeydown);
  getVersion()
    .then(v => { getCurrentWindow().setTitle("Logbook v" + v); })
    .catch((e: unknown) => { logError("MonthView.setTitle", e); });
  if (store.rootPath) {
    const { year, month } = yearMonthFromDate(store.currentDate);
    await loadMonth(year, month);
  }
});
onUnmounted(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
});

logInfo("MonthView", "mounted");
</script>

<template>
  <div class="flex min-h-[calc(100vh-64px)] bg-[var(--color-surface)] border border-[var(--color-border-form)] rounded-[var(--radius-lg)] overflow-hidden">
    <!-- Sidebar -->
    <aside class="w-[320px] flex-shrink-0 flex flex-col gap-0 bg-[var(--color-surface-muted)] border-r border-[var(--color-divider)] px-lg py-xl">
      <HeatmapCalendar
        :year="selectedYear"
        :month="selectedMonth"
        :selected-date="store.currentDate"
        :month-entries="store.monthEntries"
        :available-months="store.availableMonths"
        @navigate="handleNavigate"
        @select-day="handleSelectDay"
        @request-months="handleRequestMonths"
      />
      <div class="border-t border-[var(--color-divider)] my-xl"></div>
      <CommitmentsPanel
        :progress="store.commitmentProgress"
        :progress-result="store.commitmentProgressResult"
        :commitments="store.commitments"
        :root-path="store.rootPath"
        :selected-year="selectedYear"
        :selected-month="selectedMonth"
        @saved="onCommitmentsSaved"
      />
    </aside>

    <!-- Main -->
    <main class="flex-1 min-w-0 flex flex-col px-2xl py-xl">
      <ConfigErrorBanner
        v-if="store.configErrors.length > 0 && store.status === 'ready'"
      />
      <DayHeader
        :title="dayTitle"
        :is-today="isSelectedToday"
        :entry-count="dayEntries.length"
        :total-minutes="dayTotalMinutes"
        :can-go-next="!isSelectedToday"
        @prev-day="shiftDay(-1)"
        @next-day="shiftDay(1)"
      />

      <div class="mt-xs mb-sm py-xs">
        <div
          ref="noteRef"
          class="text-secondary italic text-[var(--color-text-secondary)] cursor-text px-sm py-sm rounded-[var(--radius-form-lg)] outline-none hover:bg-[var(--color-page-bg)]"
          contenteditable="true"
          data-placeholder="Add a note…"
          @blur="saveNote"
          @paste="onNotePaste"
          @input="onNoteInput"
          @focus="onNoteFocus"
          @keydown.esc="onNoteEsc"
          @keydown.enter="onNoteEnter"
        ></div>
      </div>

      <p v-if="store.usingDefaultDimensions" class="mb-sm text-micro text-[var(--color-text-disabled)]">
        Using default template (no custom dimensions this month)
      </p>

      <EntryList
        :entries="dayEntries"
        :just-added-id="justAddedId"
        :is-today="isSelectedToday"
        @update="handleUpdateEntry"
        @delete="handleDeleteEntry"
        @update-dimensions="handleUpdateDimensions"
      />

      <div v-if="isSelectedToday" class="mt-md">
        <EntryComposer
          ref="inputRef"
          :dimensions="store.dimensions"
          :commitments="store.commitments"
          @submit="handleSubmit"
          @edit-dimensions="openDimEditor"
        />
      </div>

      <DimensionEditorModal
        :open="showDimEditor"
        :dimensions="store.dimensions"
        :root-path="store.rootPath"
        :year="selectedYear"
        :month="selectedMonth"
        @close="showDimEditor = false"
        @saved="onDimensionsSaved"
      />

      <div v-if="store.rootPath" class="mt-sm text-right flex justify-end items-baseline gap-md">
        <button
          class="text-micro text-[var(--color-text-disabled)] hover:text-[var(--color-text-secondary)] cursor-pointer"
          :title="store.rootPath + '/' + dayFilePath"
          @click="revealDayFile"
          @contextmenu.prevent="copyFilePath"
        >{{ copiedFeedback ? 'Copied!' : displayPath }}</button>
      </div>
    </main>
  </div>
</template>

<style scoped>
[contenteditable]:empty::before {
  content: attr(data-placeholder);
  color: var(--color-placeholder);
}
</style>
