<!-- src/components/HeatmapCalendar.vue -->
<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from "vue";
import type { Entry } from "../types";
import type { AvailableMonth } from "../stores/useStore";
import { datesInMonth, parseDate, formatDate } from "../utils/dates";
import { heatLevel } from "../utils/heatmap";
import QuickJumpPopover from "./QuickJumpPopover.vue";

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

const props = defineProps<{
  year: number;
  month: number;
  selectedDate: string;
  monthEntries: Record<string, Entry[]>;
  availableMonths: AvailableMonth[] | null;
}>();

const emit = defineEmits<{
  navigate: [{ year: number; month: number }];
  selectDay: [date: string];
  requestMonths: [];
}>();

const showJump = ref(false);

const dates = computed(() => datesInMonth(`${props.year}-${String(props.month).padStart(2, "0")}-01`));

// Monday-first leading blank count for the first cell.
const leadingBlanks = computed(() => {
  const jsDay = parseDate(dates.value[0]).getDay(); // 0=Sun..6=Sat
  return (jsDay + 6) % 7;
});

function dayMinutes(date: string): number {
  return (props.monthEntries[date] || []).reduce((s, e) => s + e.duration, 0);
}

const monthTotalHours = computed(() => {
  let total = 0;
  for (const d of dates.value) total += dayMinutes(d);
  return Math.round((total / 60) * 10) / 10;
});

function isFuture(date: string): boolean {
  const now = new Date(); now.setHours(0, 0, 0, 0);
  const [y, m, d] = date.split("-").map(Number);
  const t = new Date(y, m - 1, d); t.setHours(0, 0, 0, 0);
  return t > now;
}

const cellBg: Record<string, string> = {
  empty: "bg-[var(--heatmap-empty)] text-[var(--heatmap-empty-text)]",
  light: "bg-[var(--heatmap-light)] text-[var(--heatmap-light-text)]",
  mid: "bg-[var(--heatmap-mid)] text-[var(--heatmap-mid-text)]",
  heavy: "bg-[var(--heatmap-heavy)] text-[var(--heatmap-heavy-text)] font-bold",
};

function cellClass(date: string): string {
  const base = cellBg[heatLevel(dayMinutes(date))];
  const rings: string[] = [];
  if (date === formatDate(new Date())) rings.push("shadow-[0_0_0_2px_var(--heatmap-today-ring)]");
  if (date === props.selectedDate) rings.push("shadow-[0_0_0_2px_var(--heatmap-selected-ring)]");
  return [base, ...rings, isFuture(date) ? "opacity-40 cursor-default" : "cursor-pointer hover:scale-[1.15]"].join(" ");
}

function dayNum(date: string): number {
  return parseInt(date.split("-")[2], 10);
}

function clickDay(date: string) {
  if (isFuture(date)) return;
  emit("selectDay", date);
}

function shift(delta: number) {
  let m = props.month + delta;
  let y = props.year;
  if (m < 1) { m = 12; y--; } else if (m > 12) { m = 1; y++; }
  emit("navigate", { year: y, month: m });
}

function onLabelClick() {
  if (props.availableMonths === null) { emit("requestMonths"); return; }
  showJump.value = !showJump.value;
}

function onJump(payload: { year: number; month: number }) {
  showJump.value = false;
  emit("navigate", payload);
}

// Close the jump popover on a click anywhere outside the label trigger + popover
// (jumpAnchor wraps both). The listener only lives while the popover is open.
const jumpAnchor = ref<HTMLElement>();
function onDocMousedown(e: MouseEvent) {
  if (jumpAnchor.value && !jumpAnchor.value.contains(e.target as Node)) {
    showJump.value = false;
  }
}
watch(showJump, (open) => {
  if (open) document.addEventListener("mousedown", onDocMousedown, true);
  else document.removeEventListener("mousedown", onDocMousedown, true);
});
onUnmounted(() => document.removeEventListener("mousedown", onDocMousedown, true));
</script>

<template>
  <div>
    <div ref="jumpAnchor">
      <!-- Nav row -->
      <div class="flex items-center justify-between mb-sm">
        <span data-test="prev-month" class="text-secondary text-[var(--color-text-secondary)] cursor-pointer px-xs py-2xs hover:text-[var(--color-text-primary)]" title="Previous month (⌘⇧[)" @click="shift(-1)">←</span>
        <span data-test="month-label" class="text-body font-bold text-[var(--color-text-primary)] cursor-pointer" @click="onLabelClick">
          {{ MONTH_NAMES[month - 1] }}
          <span class="font-normal text-secondary text-[var(--color-text-secondary)]">{{ year }} ▾</span>
        </span>
        <span data-test="next-month" class="text-secondary text-[var(--color-text-secondary)] cursor-pointer px-xs py-2xs hover:text-[var(--color-text-primary)]" title="Next month (⌘⇧])" @click="shift(1)">→</span>
      </div>

      <QuickJumpPopover
        v-if="showJump && availableMonths !== null"
        :year="year" :month="month" :available-months="availableMonths"
        class="mb-sm"
        @jump="onJump"
        @close="showJump = false"
      />
    </div>

    <!-- Weekday headers -->
    <div class="grid grid-cols-7 gap-xs text-center text-micro text-[var(--color-text-secondary)] mb-xs">
      <span>M</span><span>T</span><span>W</span><span>T</span><span>F</span><span>S</span><span>S</span>
    </div>

    <!-- Day grid -->
    <div class="grid grid-cols-7 gap-xs text-center">
      <span v-for="n in leadingBlanks" :key="'blank-' + n"></span>
      <span
        v-for="date in dates" :key="date"
        data-test="day-cell"
        class="mono w-[24px] h-[24px] rounded-[var(--radius-md)] flex items-center justify-center text-micro transition-all"
        :class="cellClass(date)"
        @click="clickDay(date)"
      >{{ dayNum(date) }}</span>
    </div>

    <!-- Month total -->
    <div class="mt-sm text-center text-secondary font-semibold text-[var(--color-text-primary)]">
      <span class="mono">{{ monthTotalHours }}h</span>
      <span class="font-normal text-micro text-[var(--color-text-secondary)]"> / month</span>
    </div>
  </div>
</template>
