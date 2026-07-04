export function formatDate(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

export function parseDate(dateStr: string): Date {
  return new Date(dateStr + "T00:00:00");
}

/** Return all dates (YYYY-MM-DD) in the month containing dateStr. */
export function datesInMonth(dateStr: string): string[] {
  const d = parseDate(dateStr);
  const year = d.getFullYear();
  const month = d.getMonth();
  const lastDay = new Date(year, month + 1, 0).getDate();
  const dates: string[] = [];
  for (let day = 1; day <= lastDay; day++) {
    dates.push(`${year}-${String(month + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`);
  }
  return dates;
}

/** Return the year and month from a YYYY-MM-DD date string. */
export function yearMonthFromDate(dateStr: string): { year: number; month: number } {
  const parts = dateStr.split("-");
  return { year: parseInt(parts[0], 10), month: parseInt(parts[1], 10) };
}

/** Return the date n days from dateStr (n may be negative), as YYYY-MM-DD. */
export function addDays(dateStr: string, n: number): string {
  const d = parseDate(dateStr);
  d.setDate(d.getDate() + n);
  return formatDate(d);
}

/**
 * Decide whether a calendar-day rollover should advance the viewed date.
 *
 * The view follows "today" only when the user is currently parked on what was
 * today (currentDate === lastKnownToday) and the app is ready. If they have
 * navigated to another day, a midnight crossing must NOT yank them to the new
 * today. Pure so it can be driven from both the focus handler and a timer.
 *
 * @returns `rollover: true` with the new date when the view should advance.
 */
export function rolloverDecision(
  currentDate: string,
  lastKnownToday: string,
  newToday: string,
  statusReady: boolean,
): { rollover: boolean; date: string } {
  if (newToday === lastKnownToday) return { rollover: false, date: currentDate };
  if (statusReady && currentDate === lastKnownToday) {
    return { rollover: true, date: newToday };
  }
  return { rollover: false, date: currentDate };
}
