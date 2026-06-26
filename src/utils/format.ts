import { logError } from "./errorLog";

/** Format minutes to human-readable: 90 → "1h 30m", 45 → "45m", 120 → "2h" */
export function formatDuration(minutes: number): string {
  if (minutes >= 60) {
    const h = Math.floor(minutes / 60);
    const m = minutes % 60;
    return m > 0 ? `${h}h ${m}m` : `${h}h`;
  }
  return `${minutes}m`;
}

/** Format minutes to compact hour display: 90 → "1.5h", 30 → "0.5h", 0 → "0" */
export function formatDurationCompact(minutes: number): string {
  if (minutes === 0) return "0";
  const hours = Math.round(minutes / 60 * 10) / 10;
  const display = hours % 1 === 0 ? hours.toFixed(0) : String(hours);
  return `${display}h`;
}

/** Duration pattern: number + required unit (h or m). Plain numbers are not durations. */
const DURATION_RE = /(\d+(?:\.\d+)?)\s*(h|m)/gi;

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

// ── Safe addition/subtraction evaluator ────────────────────────────────────
// Regex-based evaluator for expressions like "52+60+15". No eval(),
// no new Function(), CSP-compatible. Only + and - are supported
// (the user's main use case: "30+55m", "20+1.5h").
// h/m unit preprocessing happens before this is called.

function evalAddSub(expr: string): number {
  let total = 0;
  const re = /([+-]?)\s*(\d+(?:\.\d+)?)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(expr)) !== null) {
    const sign = m[1] === "-" ? -1 : 1;
    total += sign * parseFloat(m[2]);
  }
  return total;
}

// ────────────────────────────────────────────────────────────────────────────

/** Parse delta input: "+45" → adds to current, "-30" → subtracts, "150" → absolute.
 *  Also evaluates addition/subtraction: "30+55m" → 85, "20+1.5h" → 110. */
export function resolveDelta(input: string, currentMinutes: number): number {
  const trimmed = input.trim();

  // Delta mode: +N or -N prefix (adds/subtracts from current). Supports
  // optional h/m unit suffix: "+1h" → +60 min, "+30m" → +30 min.
  if (/^[+-]/.test(trimmed)) {
    const raw = trimmed.substring(1);
    const m = raw.match(/^(\d+(?:\.\d+)?)\s*(h|m)?$/i);
    const value = m ? parseFloat(m[1]) : parseFloat(raw) || 0;
    const unit = m?.[2]?.toLowerCase() || "m";
    const delta = unit === "h" ? value * 60 : value;
    const result = trimmed.startsWith("-") ? currentMinutes - delta : currentMinutes + delta;
    return Math.max(0, Math.round(result));
  }

  // Expression mode: detect + or - operator, evaluate as addition/subtraction.
  // Pre-process duration units: "30+55m" → "30+55", "20+1.5h" → "20+90".
  if (/[+\-]/.test(trimmed)) {
    try {
      let expr = trimmed.replace(
        /(\d+(?:\.\d+)?)\s*h/gi,
        (_, n) => String(parseFloat(n) * 60)
      ).replace(
        /(\d+(?:\.\d+)?)\s*m/gi,
        (_, n) => String(parseFloat(n))
      );
      // Sanitize: only allow digits, +, -, decimal point, whitespace
      const sanitized = expr.replace(/[^0-9+\-.\s]/g, "");
      if (sanitized.length > 0) {
        const result = evalAddSub(sanitized);
        if (typeof result === "number" && isFinite(result)) {
          return Math.max(0, Math.round(result));
        }
      }
    } catch (e) {
      // Expression evaluation failed, fall through to absolute parsing
      logError("resolveDelta", e);
    }
  }

  // Absolute mode: plain number
  const absolute = parseFloat(trimmed);
  return isNaN(absolute) ? currentMinutes : Math.max(0, Math.round(absolute));
}
