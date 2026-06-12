import { invoke } from "@tauri-apps/api/core";

export function logError(context: string, error: unknown): void {
  const message = error instanceof Error ? `${error.name}: ${error.message}` : String(error);
  const entry = `[${context}] ${message}`;
  console.error(entry);
  invoke("log_error", { message: entry }).catch(() => {});
}

export function logInfo(context: string, message: string): void {
  const entry = `[${context}] ${message}`;
  console.log(entry);
  invoke("log_info", { message: entry }).catch(() => {});
}
