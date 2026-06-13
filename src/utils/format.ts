/** Format minutes to human-readable: 90 → "1h 30m", 45 → "45m", 120 → "2h" */
export function formatDuration(minutes: number): string {
  if (minutes >= 60) {
    const h = Math.floor(minutes / 60);
    const m = minutes % 60;
    return m > 0 ? `${h}h ${m}m` : `${h}h`;
  }
  return `${minutes}m`;
}

const DURATION_RE = /(\d+(?:\.\d+)?)\s*(h|m)?/gi;

/** Parse duration from text. Accumulates as float, rounds once at end (matches Rust). */
export function parseDurationFromText(text: string): number | null {
  let total = 0;
  let matched = false;
  const re = new RegExp(DURATION_RE.source, "gi");
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    const value = parseFloat(m[1]);
    const unit = (m[2] || "m").toLowerCase();
    total += unit === "h" ? value * 60 : value;
    matched = true;
  }
  return matched ? Math.round(total) : null;
}

/** Remove duration patterns from text and clean up orphaned brackets/parentheses. */
export function stripDurations(text: string): string {
  let cleaned = text.replace(DURATION_RE, "");
  cleaned = cleaned
    .replace(/[([（]\s*[)\]）]/g, "")
    .replace(/\s*[,;，；]\s*$/, "")
    .replace(/\s+/g, " ")
    .trim();
  return cleaned || text.trim();
}

/** Parse delta input: "+45" → adds to current, "-30" → subtracts, "150" → absolute.
 *  Also evaluates arithmetic expressions: "5+60" → 65, "(30+20)*3" → 150. */
export function resolveDelta(input: string, currentMinutes: number): number {
  const trimmed = input.trim();

  // Delta mode: +N or -N prefix (adds/subtracts from current)
  if (/^[+-]/.test(trimmed)) {
    const delta = parseFloat(trimmed.substring(1)) || 0;
    const result = trimmed.startsWith("-") ? currentMinutes - delta : currentMinutes + delta;
    return Math.max(0, Math.round(result));
  }

  // Expression mode: detect arithmetic operators, evaluate safely
  if (/[+\-*/]/.test(trimmed)) {
    try {
      // Sanitize: only allow digits, operators, parens, decimal point, whitespace
      const sanitized = trimmed.replace(/[^0-9+\-*/().%\s]/g, "");
      if (sanitized === trimmed && sanitized.length > 0) {
        const result = new Function(`return (${sanitized})`)();
        if (typeof result === "number" && isFinite(result)) {
          return Math.max(0, Math.round(result));
        }
      }
    } catch {
      // Expression evaluation failed, fall through to absolute parsing
    }
  }

  // Absolute mode: plain number
  const absolute = parseFloat(trimmed);
  return isNaN(absolute) ? currentMinutes : Math.max(0, Math.round(absolute));
}
