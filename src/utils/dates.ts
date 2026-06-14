export function formatDate(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

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
