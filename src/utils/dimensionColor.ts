// Dimension key → left-bar color token.
//
// The CSS variables in tokens.css use abbreviated suffixes (--dim-bar-cat,
// --dim-bar-biz, --dim-bar-imp) that do NOT match the full dimension keys
// (category, business-line, importance-urgency). A naive `--dim-bar-${key}`
// interpolation therefore resolves to an undefined variable for every key
// except `goal`. This map is the single source of truth for the mapping so
// DimensionPopover and DimensionEditorModal can't drift apart.

const KEY_TO_BAR: Record<string, string> = {
  category: "--dim-bar-cat",
  "business-line": "--dim-bar-biz",
  "importance-urgency": "--dim-bar-imp",
  goal: "--dim-bar-goal",
};

const FALLBACK = "--dim-bar-cat";

/** The `--dim-bar-*` CSS variable name for a dimension key (with leading `--`). */
export function dimBarVar(key: string): string {
  return KEY_TO_BAR[key] ?? FALLBACK;
}

/** `var(--dim-bar-*)` reference for use in an inline `background` style. */
export function dimBarColor(key: string): string {
  return `var(${dimBarVar(key)})`;
}

/** Tailwind `bg-[var(--dim-bar-*)]` class for use in `:class`. */
export function dimBarClass(key: string): string {
  return `bg-[var(${dimBarVar(key)})]`;
}
