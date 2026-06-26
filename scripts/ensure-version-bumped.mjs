#!/usr/bin/env node
// Ensure the current version hasn't already been released.
// If a git tag v{version} exists, auto-bump patch before building.
//
// Called automatically by `pnpm tauri:prod` — no need to remember.

import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

function readJSON(filePath) {
  return JSON.parse(readFileSync(filePath, "utf-8"));
}

function writeJSON(filePath, obj) {
  writeFileSync(filePath, JSON.stringify(obj, null, 2) + "\n");
}

function bump(version) {
  const parts = version.split(".").map(Number);
  if (parts.length !== 3) {
    console.error(`Expected semver x.y.z, got: ${version}`);
    process.exit(1);
  }
  parts[2] += 1;
  return parts.join(".");
}

// 1. Read current version from the source of truth (tauri.conf.json)
const tauriConf = readJSON(resolve(ROOT, "src-tauri", "tauri.conf.json"));
const currentVersion = tauriConf.version;
console.log(`Current version: ${currentVersion}`);

// 2. Check if git tag v{version} already exists
let tagExists = false;
try {
  const output = execSync(`git tag -l "v${currentVersion}"`, {
    cwd: ROOT,
    encoding: "utf-8",
  });
  tagExists = output.trim() === `v${currentVersion}`;
} catch {
  console.warn("Warning: could not check git tags, skipping version check");
  process.exit(0);
}

if (!tagExists) {
  console.log(
    `Tag v${currentVersion} does not exist — version is fresh, proceeding.`,
  );
  process.exit(0);
}

// 3. Tag exists — auto-bump patch version
const newVersion = bump(currentVersion);
console.log(
  `Tag v${currentVersion} already exists → auto-bumping to ${newVersion}`,
);

// 4. Update all three version files
const pkg = readJSON(resolve(ROOT, "package.json"));
pkg.version = newVersion;
writeJSON(resolve(ROOT, "package.json"), pkg);

tauriConf.version = newVersion;
writeJSON(resolve(ROOT, "src-tauri", "tauri.conf.json"), tauriConf);

const cargoPath = resolve(ROOT, "src-tauri", "Cargo.toml");
let cargoToml = readFileSync(cargoPath, "utf-8");
cargoToml = cargoToml.replace(
  /^version\s*=\s*"[^"]*"/m,
  `version = "${newVersion}"`,
);
writeFileSync(cargoPath, cargoToml);

// 5. Git commit + tag (local only — push happens separately)
execSync(
  "git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml",
  { cwd: ROOT },
);
execSync(`git commit -m "chore: bump version to ${newVersion}"`, {
  cwd: ROOT,
});
execSync(`git tag v${newVersion}`, { cwd: ROOT });

console.log(
  `✓ Bumped to ${newVersion}, tagged v${newVersion} (local only — push with --follow-tags)`,
);
