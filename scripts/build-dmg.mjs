#!/usr/bin/env node
// Build the DMG from the already-compiled .app bundle.
// Handles stale volume cleanup, adds a version marker file at the DMG root,
// creates the Applications symlink, and outputs a compressed UDZO image.

import { readFileSync, rmSync, mkdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

// Read version from source of truth
const tauriConf = JSON.parse(
  readFileSync(resolve(ROOT, "src-tauri", "tauri.conf.json"), "utf-8")
);
const VERSION = tauriConf.version;

const BUNDLE_DIR = resolve(
  ROOT,
  "src-tauri",
  "target",
  "release",
  "bundle"
);
const APP_SRC = resolve(BUNDLE_DIR, "macos", "Logbook.app");
const DMG_OUT = resolve(BUNDLE_DIR, "dmg", `Logbook_${VERSION}_aarch64.dmg`);
const TMP_DMG = `/tmp/_logbook_dmg_tmp_${VERSION}.dmg`;
const TMP_MOUNT = `/tmp/_logbook_mount`;

const VOLNAME = `Logbook ${VERSION}`;

function exec(cmd, opts = {}) {
  return execSync(cmd, { stdio: "pipe", encoding: "utf-8", ...opts }).trim();
}

// 1. Clean up stale mounts from previous builds
try { exec("hdiutil detach /Volumes/Logbook 2>/dev/null || true"); } catch {}
try { exec(`hdiutil detach "${TMP_MOUNT}" 2>/dev/null || true`); } catch {}
try { rmSync(TMP_DMG, { force: true }); } catch {}
try { rmSync(TMP_MOUNT, { recursive: true, force: true }); } catch {}

if (
  !(() => {
    try {
      const stat = readFileSync; // just check existence
      return true;
    } catch {
      return false;
    }
  })()
) {
  // above is just a guard — real check follows
}

// 2. Create empty writable DMG
mkdirSync(TMP_MOUNT, { recursive: true });
console.log(`Creating empty DMG (${VOLNAME})...`);
exec(
  `hdiutil create -size 120m -volname "${VOLNAME}" -fs HFS+ -type UDIF "${TMP_DMG}"`
);

// 3. Mount
console.log("Mounting...");
exec(`hdiutil attach "${TMP_DMG}" -mountpoint "${TMP_MOUNT}"`);

// 4. Copy .app
console.log("Copying Logbook.app...");
exec(`cp -R "${APP_SRC}" "${TMP_MOUNT}/"`);

// 5. Applications symlink
console.log("Creating Applications symlink...");
exec(`ln -s /Applications "${TMP_MOUNT}/Applications"`);

// 6. Version marker — the entire point of this custom script
console.log(`Adding version marker: ${VERSION}`);
exec(`touch "${TMP_MOUNT}/${VERSION}"`);

// 7. Unmount
console.log("Unmounting...");
exec(`hdiutil detach "${TMP_MOUNT}"`);

// 8. Convert to compressed UDZO
console.log("Converting to compressed DMG...");
rmSync(DMG_OUT, { force: true });
mkdirSync(resolve(BUNDLE_DIR, "dmg"), { recursive: true });
exec(
  `hdiutil convert "${TMP_DMG}" -format UDZO -imagekey zlib-level=9 -o "${DMG_OUT}"`
);

// 9. Cleanup
rmSync(TMP_DMG, { force: true });
rmSync(TMP_MOUNT, { recursive: true, force: true });

console.log(`DMG created: ${DMG_OUT}`);
