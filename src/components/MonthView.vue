<!-- src/components/MonthView.vue -->
<script setup lang="ts">
import { inject, computed, watch, ref, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { useStore } from "../stores/useStore";
import HeatmapCalendar from "./HeatmapCalendar.vue";
import CommitmentsPanel from "./CommitmentsPanel.vue";
import DayHeader from "./DayHeader.vue";
import EntryList from "./EntryList.vue";
import EntryComposer from "./EntryComposer.vue";
import DimensionEditorModal from "./composite/DimensionEditorModal.vue";
import type { DayFile, Entry, CommitmentProgress, Commitment, MonthDimensions, Dimension } from "../types";
import { logError, logInfo } from "../utils/errorLog";
import { datesInMonth, yearMonthFromDate, parseDate, addDays } from "../utils/dates";

const store = useStore();
const inputRef = ref<InstanceType<typeof EntryComposer> | null>(null);

// Dimension editor modal
const showDimEditor = ref(false);

function openDimEditor() { showDimEditor.value = true; }

function onDimensionsSaved(dims: Dimension[]) {
  store.dimensions = dims;
  store.fromTemplate = false;
  showDimEditor.value = false;
}

// Newly-added entry highlight (spec §5.2 step 7): mark the id, clear after 1.5s.
const justAddedId = ref<string | null>(null);
let highlightTimer: ReturnType<typeof setTimeout> | null = null;

const appVersion = ref("");

const selectedYear = computed(() => yearMonthFromDate(store.currentDate).year);
const selectedMonth = computed(() => yearMonthFromDate(store.currentDate).month);

function todayStr(): string {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
}
const isSelectedToday = computed(() => store.currentDate === todayStr());

const dayEntries = computed(() => store.today?.entries || []);
const dayTotalMinutes = computed(() => dayEntries.value.reduce((s, e) => s + e.duration, 0));

const dayTitle = computed(() => {
  const d = parseDate(store.currentDate);
  return d.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" });
});

const triggerUndoToast = inject<(undoFn: () => void) => void>("triggerUndoToast", () => {});
const triggerSavedToast = inject<(msg: string) => void>("triggerSavedToast", () => {});

// ---- Month loading ----
async function loadMonth(year: number, month: number, defaultDay?: number) {
  const now = new Date();
  const isCurrentMonth = year === now.getFullYear() && month === now.getMonth() + 1;
  let day: number;
  if (defaultDay !== undefined) day = defaultDay;
  else if (isCurrentMonth) day = now.getDate();
  else day = new Date(year, month, 0).getDate();

  const dateStr = `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
  store.currentDate = dateStr;

  const dates = datesInMonth(dateStr);
  const map: Record<string, Entry[]> = {};
  for (const date of dates) {
    try {
      const df = (await invoke("get_entries", { rootPath: store.rootPath, date })) as DayFile;
      map[date] = df.entries;
    } catch (e) { logError("MonthView.loadMonth", e); map[date] = []; }
  }
  store.monthEntries = map;
  await loadCommitmentProgress(year, month);
  await loadCommitments(year, month);
  await loadMonthDimensions(year, month);
  if (store.currentDate in map) {
    store.today = { note: null, entries: map[store.currentDate] };
    loadDayNote(store.currentDate);
  }
}

async function loadCommitmentProgress(year: number, month: number) {
  try {
    store.commitmentProgress = (await invoke("get_commitment_progress", { rootPath: store.rootPath, year, month })) as CommitmentProgress[];
  } catch (e) { logError("MonthView.loadCommitmentProgress", e); store.commitmentProgress = []; }
}

async function loadCommitments(year: number, month: number) {
  try {
    store.commitments = (await invoke("get_commitments", { rootPath: store.rootPath, year, month })) as Commitment[];
  } catch (e) { logError("MonthView.loadCommitments", e); store.commitments = []; }
}

// Refresh the dimension set for the viewed month: a month's own snapshot if
// instantiated, else the global template (from_template = true → preview state).
async function loadMonthDimensions(year: number, month: number) {
  try {
    const md = (await invoke("get_month_dimensions", { rootPath: store.rootPath, year, month })) as MonthDimensions;
    // Only adopt a well-formed response; never wipe dimensions on a malformed/missing one.
    if (md && Array.isArray(md.dimensions)) {
      store.dimensions = md.dimensions;
      store.fromTemplate = md.from_template;
    }
  } catch (e) { logError("MonthView.loadMonthDimensions", e); }
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
    const df = (await invoke("get_entries", { rootPath: store.rootPath, date: dateStr })) as DayFile;
    if (store.today) store.today.note = df.note;
  } catch (e) { logError("MonthView.loadDayNote", e); }
}

async function handleSelectDay(dateStr: string) {
  store.currentDate = dateStr;
  if (dateStr in store.monthEntries) {
    store.today = { note: null, entries: store.monthEntries[dateStr] };
    await loadDayNote(dateStr);
  }
}

async function handleNavigate({ year, month }: { year: number; month: number }) {
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
  const newEntry = { item, duration: String(durationMinutes), dimensions: finalDimensions };
  try {
    const result = await invoke("append_entry", { rootPath: store.rootPath, date: store.currentDate, entry: newEntry });
    inputRef.value?.clearInput();
    const added = result as Entry;
    if (store.today) {
      const entries = [...store.today.entries, added];
      store.today = { ...store.today, entries };
      store.monthEntries[store.currentDate] = entries;
    }
    justAddedId.value = added.id;
    if (highlightTimer) clearTimeout(highlightTimer);
    highlightTimer = setTimeout(() => { justAddedId.value = null; }, 1500);
    await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
  } catch (e) { logError("MonthView.handleSubmit", e); }
}

// ---- Entry mutations ----
async function handleUpdateEntry(entryId: string, item: string, durationMinutes: number) {
  const entries = store.today?.entries;
  if (!entries) return;
  const entry = entries.find(e => e.id === entryId);
  if (!entry) return;
  const update: Record<string, unknown> = {};
  if (item !== entry.item) update.item = item;
  if (durationMinutes !== entry.duration) update.duration = String(durationMinutes);
  if (Object.keys(update).length === 0) return;
  try {
    const df = (await invoke("update_entry", { rootPath: store.rootPath, date: store.currentDate, entryId, update })) as DayFile;
    store.today = df;
    store.monthEntries[store.currentDate] = df.entries;
    await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
    triggerSavedToast("Saved");
  } catch (e) { logError("MonthView.handleUpdateEntry", e); }
}

async function handleUpdateDimensions(entryId: string, dimensions: Record<string, string>) {
  try {
    const df = (await invoke("update_entry", { rootPath: store.rootPath, date: store.currentDate, entryId, update: { dimensions } })) as DayFile;
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
  }, 5000);
  triggerUndoToast(() => {
    cancelled = true;
    if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
    if (entries.findIndex(e => e.id === entryId) === -1) entries.splice(idx, 0, removed);
  });
}

// ---- Day note (inline) ----
const noteRef = ref<HTMLDivElement>();
watch(() => store.today?.note, (n) => {
  if (noteRef.value && noteRef.value.textContent !== (n || "")) noteRef.value.textContent = n || "";
}, { immediate: true });

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
  try { await invoke("set_day_note", { rootPath: store.rootPath, date: store.currentDate, note: text }); }
  catch (e) { logError("MonthView.saveNote", e); }
}

let noteSnapshot = "";
function onNoteFocus() {
  noteSnapshot = noteRef.value?.textContent || "";
}
function onNoteEsc(e: KeyboardEvent) {
  e.preventDefault();
  if (noteRef.value) noteRef.value.textContent = noteSnapshot;
  noteRef.value?.blur(); // triggers saveNote with unchanged content (no-op write)
}
function onNoteEnter(e: KeyboardEvent) {
  if (e.isComposing) return; // let an IME Enter confirm its candidate, don't commit the note
  e.preventDefault(); // the note is single-line; don't insert a newline
  noteRef.value?.blur(); // commit via the existing blur → saveNote, clearing the caret
}

// ---- File path ----
const dayFilePath = computed(() => {
  if (!store.rootPath) return "";
  const d = store.currentDate;
  return `${d.slice(0, 4)}/${d.slice(5, 7)}/${d}.md`;
});
const displayPath = computed(() => (store.rootPath ? `…/${dayFilePath.value}` : ""));
async function revealDayFile() {
  if (!store.rootPath) return;
  try { await invoke("reveal_day_file", { rootPath: store.rootPath, date: store.currentDate }); }
  catch (e) { logError("MonthView.revealDayFile", e); }
}

// ---- Keyboard month navigation (⌘[ / ⌘]) ----
function shiftMonth(delta: number) {
  let m = selectedMonth.value + delta;
  let y = selectedYear.value;
  if (m < 1) { m = 12; y--; } else if (m > 12) { m = 1; y++; }
  loadMonth(y, m);
}
function shiftDay(delta: number) {
  if (delta > 0 && isSelectedToday.value) return; // never navigate into the future
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
  const t = todayStr();
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
  getVersion().then(v => { appVersion.value = v; }).catch(() => {});
  if (store.rootPath) {
    const { year, month } = yearMonthFromDate(store.currentDate);
    await loadMonth(year, month);
  }
});
onUnmounted(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
  if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
  if (highlightTimer) clearTimeout(highlightTimer);
});

logInfo("MonthView", "mounted");
</script>

<template>
  <div class="flex min-h-[calc(100vh-64px)] bg-[var(--color-surface)] border border-[var(--color-border-form)] rounded-[var(--radius-lg)] overflow-hidden">
    <!-- Sidebar -->
    <aside class="w-[280px] flex-shrink-0 flex flex-col gap-0 bg-[var(--color-surface-muted)] border-r border-[var(--color-divider)] px-lg py-xl">
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
        :commitments="store.commitments"
        :root-path="store.rootPath"
        :selected-year="selectedYear"
        :selected-month="selectedMonth"
        @saved="onCommitmentsSaved"
      />
    </aside>

    <!-- Main -->
    <main class="flex-1 min-w-0 flex flex-col px-2xl py-xl">
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

      <p v-if="store.fromTemplate" class="mb-sm text-micro text-[var(--color-text-disabled)]">
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
        <span v-if="appVersion" class="text-micro text-[var(--color-text-disabled)]">v{{ appVersion }}</span>
        <button
          class="text-micro text-[var(--color-text-disabled)] hover:text-[var(--color-text-secondary)] cursor-pointer"
          :title="store.rootPath + '/' + dayFilePath"
          @click="revealDayFile"
        >{{ displayPath }}</button>
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
