import { invoke } from "@tauri-apps/api/core";

export function logError(context: string, error: unknown): void {
  const message = error instanceof Error ? `${error.name}: ${error.message}` : String(error);
  const entry = `[${context}] ${message}`;
  console.error(entry);
  // Fire-and-forget to Rust; don't await to avoid blocking
  invoke("log_error", { message: entry }).catch(() => {
    // If even this fails, we're in trouble — but don't crash
  });
}
