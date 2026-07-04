<!-- src/components/MonthView.vue -->
<script setup lang="ts">
import { inject, computed, ref, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useStore } from "../stores/useStore";
import { useDayNote } from "../composables/useDayNote";
import { useFileActions } from "../composables/useFileActions";
import HeatmapCalendar from "./HeatmapCalendar.vue";
import CommitmentsPanel from "./CommitmentsPanel.vue";
import DayHeader from "./DayHeader.vue";
import EntryList from "./EntryList.vue";
import EntryComposer from "./EntryComposer.vue";
import DimensionEditorModal from "./composite/DimensionEditorModal.vue";
import type { DayFile, Entry, Commitment, CommitmentProgressResult, MonthDimensions, Dimension } from "../types";
import { UNDO_TOAST_KEY, SAVED_TOAST_KEY } from "../types";
import { logError, logInfo } from "../utils/errorLog";
import { yearMonthFromDate, parseDate, addDays, formatDate } from "../utils/dates";
import { HIGHLIGHT_DURATION, UNDO_DELETE_DELAY } from "../utils/constants";
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

// Newly-added entry highlight (spec §5.2 step 7): mark the id, clear after 1.5s.
const justAddedId = ref<string | null>(null);
let highlightTimer: ReturnType<typeof setTimeout> | null = null;

const selectedYear = computed(() => yearMonthFromDate(store.currentDate).year);
const selectedMonth = computed(() => yearMonthFromDate(store.currentDate).month);

const isSelectedToday = computed(() => store.currentDate === formatDate(new Date()));

const dayEntries = computed(() => store.today?.entries || []);
const dayTotalMinutes = computed(() => dayEntries.value.reduce((s, e) => s + e.duration, 0));

const dayTitle = computed(() => {
  const d = parseDate(store.currentDate);
  return d.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" });
});

const triggerUndoToast = inject(UNDO_TOAST_KEY, () => {});
const triggerSavedToast = inject(SAVED_TOAST_KEY, () => {});
const { noteRef, saveNote, onNotePaste, onNoteInput, onNoteFocus, onNoteEsc, onNoteEnter } = useDayNote(store);
const { dayFilePath, displayPath, revealDayFile, copyFilePath, copiedFeedback } = useFileActions(store);

// ---- Month loading ----
async function loadMonth(year: number, month: number, defaultDay?: number) {
  store.configErrors = [];
  store.commitments = [];
  store.commitmentProgress = [];
  store.commitmentProgressResult = null;
  const now = new Date();
  const isCurrentMonth = year === now.getFullYear() && month === now.getMonth() + 1;
  let day: number;
  if (defaultDay !== undefined) day = defaultDay;
  else if (isCurrentMonth) day = now.getDate();
  else day = new Date(year, month, 0).getDate();

  const dateStr = `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
  store.currentDate = dateStr;

  try {
    store.monthEntries = await invoke<Record<string, Entry[]>>("get_month_entries", { rootPath: store.rootPath, year, month });
  } catch (e) {
    logError("MonthView.loadMonth", e);
    store.monthEntries = {};
    const msg = String(e);
    if (msg.includes("dimensions")) {
      store.configErrors = [{ kind: "ConfigError", message: msg }];
      store.configCategory = "in_place";
      return;
    }
  }
  await loadCommitmentProgress(year, month);
  await loadCommitments(year, month);
  await loadMonthDimensions(year, month);
  if (store.currentDate in store.monthEntries) {
    store.today = { note: null, entries: store.monthEntries[store.currentDate] };
    loadDayNote(store.currentDate);
  }
}

async function loadCommitmentProgress(year: number, month: number) {
  try {
    const result = await invoke<CommitmentProgressResult>("get_commitment_progress", { rootPath: store.rootPath, year, month });
    store.commitmentProgress = result.roles;
    store.commitmentProgressResult = result;
  } catch (e) {
    logError("MonthView.loadCommitmentProgress", e);
    store.commitmentProgress = [];
    store.commitmentProgressResult = null;
    const msg = String(e);
    if (msg.includes("dimensions")) {
      store.configErrors = [{ kind: "ConfigError", message: msg }];
      store.configCategory = "in_place";
    }
  }
}

async function loadCommitments(year: number, month: number) {
  try {
    store.commitments = await invoke<Commitment[]>("get_commitments", { rootPath: store.rootPath, year, month });
  } catch (e) { logError("MonthView.loadCommitments", e); store.commitments = []; }
}

// Refresh the dimension set for the viewed month: a month's own snapshot if
// instantiated, else the global template (usingDefaultDimensions = true → preview state).
async function loadMonthDimensions(year: number, month: number) {
  try {
    const md = await invoke<MonthDimensions>("get_month_dimensions", { rootPath: store.rootPath, year, month });
    // Only adopt a well-formed response; never wipe dimensions on a malformed/missing one.
    if (md && Array.isArray(md.dimensions)) {
      store.dimensions = md.dimensions;
      store.usingDefaultDimensions = md.usingDefaultDimensions;
    }
  } catch (e) {
    logError("MonthView.loadMonthDimensions", e);
    const msg = String(e);
    if (msg.includes("dimensions")) {
      store.configErrors = [{ kind: "ConfigError", message: msg }];
      store.configCategory = "in_place";
    }
  }
}

// Optimistically reflect the just-saved commitments so the panel's Edit/Set-up
// gating and the next modal open use fresh data immediately, rather than waiting
// for the `commitments-changed` file-watcher round-trip. Progress is recomputed
// from the backend since logged totals can shift with goal renames.
async function onCommitmentsSaved(commitments: Commitment[]) {
  store.commitments = commitments;
  await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
}

async function loadDayNote(dateStr: string) {
  try {
    const df = await invoke<DayFile>("get_entries", { rootPath: store.rootPath, date: dateStr });
    if (store.today) store.today.note = df.note;
  } catch (e) { logError("MonthView.loadDayNote", e); }
}

async function handleSelectDay(dateStr: string) {
  if (!guardUnsaved()) return;
  store.currentDate = dateStr;
  if (dateStr in store.monthEntries) {
    store.today = { note: null, entries: store.monthEntries[dateStr] };
    await loadDayNote(dateStr);
  }
}

async function handleNavigate({ year, month }: { year: number; month: number }) {
  if (!guardUnsaved()) return;
  await loadMonth(year, month);
}

async function handleRequestMonths() {
  if (store.availableMonths !== null) return;
  try {
    store.availableMonths = (await invoke("get_available_months", { rootPath: store.rootPath })) as { year: number; month: number }[];
  } catch (e) { logError("MonthView.handleRequestMonths", e); store.availableMonths = []; }
}

// ---- Append (absorbed from the deleted QuickEntry) ----
function sanitizeValues(vals: Record<string, string>): Record<string, string> {
  const validKeys = new Set(store.dimensions.map(d => d.key));
  const cleaned: Record<string, string> = {};
  for (const [k, v] of Object.entries(vals)) if (validKeys.has(k) && v) cleaned[k] = v;
  return cleaned;
}

async function handleSubmit(item: string, durationMinutes: number, dimensions: Record<string, string>) {
  const finalDimensions = sanitizeValues(dimensions);
  const newEntry = { item, duration: String(durationMinutes) + 'm', dimensions: finalDimensions };
  try {
    const result = await invoke("append_entry", { rootPath: store.rootPath, date: store.currentDate, entry: newEntry });
    const added = result as Entry;
    if (store.today) {
      const entries = [...store.today.entries, added];
      store.today = { ...store.today, entries };
      store.monthEntries[store.currentDate] = entries;
    }
    justAddedId.value = added.id;
    if (highlightTimer) clearTimeout(highlightTimer);
    highlightTimer = setTimeout(() => { justAddedId.value = null; }, HIGHLIGHT_DURATION);
    await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
    // Only clear on success so the user's input survives a failed save.
    inputRef.value?.clearInput();
  } catch (e) {
    logError("MonthView.handleSubmit", e);
    triggerSavedToast("Failed to save entry");
  }
}

// ---- Entry mutations ----
async function handleUpdateEntry(entryId: string, item: string, durationMinutes: number) {
  const entries = store.today?.entries;
  if (!entries) return;
  const entry = entries.find(e => e.id === entryId);
  if (!entry) return;
  const update: Record<string, unknown> = {};
  if (item !== entry.item) update.item = item;
  if (durationMinutes !== entry.duration) update.duration = String(durationMinutes) + 'm';
  if (Object.keys(update).length === 0) return;
  try {
    const df = await invoke<DayFile>("update_entry", { rootPath: store.rootPath, date: store.currentDate, entryId, update });
    store.today = df;
    store.monthEntries[store.currentDate] = df.entries;
    await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
    triggerSavedToast("Saved");
  } catch (e) { logError("MonthView.handleUpdateEntry", e); }
}

async function handleUpdateDimensions(entryId: string, dimensions: Record<string, string>) {
  try {
    const df = await invoke<DayFile>("update_entry", { rootPath: store.rootPath, date: store.currentDate, entryId, update: { dimensions } });
    store.today = df;
    store.monthEntries[store.currentDate] = df.entries;
    await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
    triggerSavedToast("Saved");
  } catch (e) { logError("MonthView.handleUpdateDimensions", e); }
}

let pendingDeleteTimer: ReturnType<typeof setTimeout> | null = null;
async function handleDeleteEntry(entryId: string) {
  const entries = store.today?.entries;
  if (!entries) return;
  const idx = entries.findIndex(e => e.id === entryId);
  if (idx === -1) return;
  // Snapshot the date NOW: the timer fires in 5s, by which point the user may
  // have navigated to another day, so reading store.currentDate at fire time
  // would delete from the wrong day (F5).
  const date = store.currentDate;
  const { year, month } = yearMonthFromDate(date);
  const [removed] = entries.splice(idx, 1);
  let cancelled = false;
  pendingDeleteTimer = setTimeout(async () => {
    if (cancelled) return;
    try {
      await invoke("delete_entry", { rootPath: store.rootPath, date, entryId });
      store.monthEntries[date] = [...entries];
      await loadCommitmentProgress(year, month);
    } catch (e) {
      logError("MonthView.handleDeleteEntry", e);
      if (entries.findIndex(e => e.id === entryId) === -1) entries.splice(idx, 0, removed);
    }
  }, UNDO_DELETE_DELAY);
  triggerUndoToast(() => {
    cancelled = true;
    if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
    if (entries.findIndex(e => e.id === entryId) === -1) {
      entries.splice(idx, 0, removed);
      // Keep the month cache in sync with the restored entry.
      store.monthEntries[date] = [...entries];
    }
  });
}

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

function onBeforeUnload(e: BeforeUnloadEvent) {
  if (pendingDeleteTimer) {
    e.preventDefault();
  }
}

onMounted(async () => {
  window.addEventListener("keydown", onGlobalKeydown);
  window.addEventListener("beforeunload", onBeforeUnload);
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
  window.removeEventListener("beforeunload", onBeforeUnload);
  if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
  if (highlightTimer) clearTimeout(highlightTimer);
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
