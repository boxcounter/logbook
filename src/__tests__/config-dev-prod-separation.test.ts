import { describe, it, expect } from "vitest";

/**
 * Regression guard for dev/prod config separation.
 *
 * `tauri.conf.json` (base config, used by `tauri dev`) must use the dev
 * identifier so dev builds use an isolated data directory and never touch
 * production data. `tauri.conf.prod.json` (merged via --config during
 * `pnpm tauri:prod`) overrides it with the production identifier.
 *
 * Accidentally writing the prod identifier into the base config causes
 * `tauri dev` to read/write the production data directory — a silent data
 * corruption risk. This guard prevents that.
 */
const baseConfig = import.meta.glob("../../src-tauri/tauri.conf.json", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const prodConfig = import.meta.glob("../../src-tauri/tauri.conf.prod.json", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

describe("Tauri config dev/prod separation guard", () => {
  it("base config (tauri dev) uses dev identifier", () => {
    const raw = Object.values(baseConfig)[0] ?? "";
    expect(raw, "tauri.conf.json should be readable").not.toBe("");
    const config = JSON.parse(raw);
    expect(config.identifier, "tauri.conf.json must use dev identifier to isolate dev data").toBe(
      "com.boxcounter.logbook.dev",
    );
  });

  it("prod config (tauri:prod) uses production identifier", () => {
    const raw = Object.values(prodConfig)[0] ?? "";
    expect(raw, "tauri.conf.prod.json should be readable").not.toBe("");
    const config = JSON.parse(raw);
    expect(config.identifier, "tauri.conf.prod.json must use production identifier").toBe(
      "com.boxcounter.logbook",
    );
  });
});
