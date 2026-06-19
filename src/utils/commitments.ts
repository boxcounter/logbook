import type { CommitmentProgress } from "../types";

/** Logged minutes for a goal, matched by its original (load-time) name.
 *  Returns 0 for unsaved goals (origName null) or names with no progress. */
export function goalLoggedMinutes(progress: CommitmentProgress[], origName: string | null): number {
  if (!origName) return 0;
  for (const p of progress) {
    const g = p.goals.find(x => x.name === origName);
    if (g) return g.spent_minutes;
  }
  return 0;
}
