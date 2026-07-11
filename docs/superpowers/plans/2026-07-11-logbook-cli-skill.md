# logbook-cli Agent Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a companion Agent Skill for logbook-cli, plus a README addition pointing to it, so AI agents can correctly operate the user's work time data.

**Architecture:** Single-file SKILL.md (slax-reader pattern) under `skill/logbook-cli/`, with a sibling `README.md` for self-contained install instructions. Repo root `README.md` gets a minimal CLI section linking to the skill.

**Tech Stack:** Markdown (skill content), no code dependencies.

## Global Constraints

- Skill frontmatter `name` must match directory name (`logbook-cli`).
- Skill description must be "pushy" with explicit "Trigger when..." clause (per skill-creator guidance).
- SKILL.md target under 500 lines.
- Imperative voice throughout. Explain *why* where non-obvious.
- No coverage of `migrate` command (out of scope — one-time internal tool).
- No coverage of `--root-path` override (personal-assistant scenario, default from root_path.txt).
- No coverage of build/install/bundle ID internals (development concerns, not usage).
- `dimensions set` IS covered (user confirmed).
- `commitments set` and `dimensions set` flagged as needing explicit user confirmation in Security rules.

---

### Task 1: Create `skill/logbook-cli/SKILL.md`

**Files:**
- Create: `skill/logbook-cli/SKILL.md`

**Interfaces:**
- Produces: the Skill file that Task 2's README links to and Task 3's README addition points to.

**Context for implementer:**
- GUI process detection: `pgrep -fi "Logbook.app/Contents/MacOS"` reliably matches the running GUI (verified: process is `/Applications/Logbook.app/Contents/MacOS/logbook`). This check is only needed before write commands.
- Two distinct `--json` flags exist: the global one (placed before subcommand, controls output format) and a local one on `dimensions list/set` (placed after subcommand, controls list output format / set input format). This is the #1 footgun.
- Write commands read from stdin, not args. Forgetting to pipe input causes hang.
- `duration` is a string (e.g. `"90"`, `"1h30m"`), parsed server-side by `parse_duration`.
- Read-only commands (skip instance lock, run alongside GUI): `entries list`, `commitments list`, `commitments progress`, `dimensions list`.
- Write commands (acquire instance lock, fail if GUI running): `entries add`, `entries update`, `entries delete`, `commitments set`, `dimensions set`.
- Write commands run `integrity::check()` first; corrupted data dir → immediate failure.
- Diagnostics (`Using data root:`, warnings, errors) go to stderr; data goes to stdout — safe to pipe.
- `allocation` in commitments is in **hours**, not minutes.
- `commitments set` propagates goal/role renames to existing entries and protects goals referenced by entries — it changes historical data.
- `dimensions list` human output is YAML (not the text format other commands use), and month variant prepends a `# source:` comment line.
- `entries list` accepts a single `--date` only — no range query. To query multiple days, run multiple times.

- [ ] **Step 1: Create the directory and file**

```bash
mkdir -p skill/logbook-cli
```

- [ ] **Step 2: Write `skill/logbook-cli/SKILL.md`**

Write this exact content:

```markdown
---
name: logbook-cli
description: "Use logbook-cli to read and write work time tracking data — list, add, update, delete entries, check commitments progress, view and edit dimensions. Trigger when the user asks to log time, check hours worked, see time breakdown by role/goal, or manage their Logbook dimensions."
---

# Logbook CLI

`logbook-cli` reads and writes Logbook work time data from the command line.

## Before using commands

**1. Check the binary is on PATH.** Run `which logbook-cli`. If it fails, ask the user to install it: open the Logbook app → menu bar → **Install Command Line Tool…**, then ensure `~/.local/bin` is on their `PATH`.

**2. Check the GUI is not running before any write command.** Write commands (`entries add/update/delete`, `commitments set`, `dimensions set`) acquire an instance lock that the GUI holds — they fail immediately if the GUI is open. Before running a write command, check:

```bash
pgrep -fi "Logbook.app/Contents/MacOS" > /dev/null && echo "GUI_RUNNING" || echo "OK"
```

If the output is `GUI_RUNNING`, ask the user to close the Logbook app before proceeding. Do not attempt the write command and wait for it to fail — check first.

Read commands (`entries list`, `commitments list`, `commitments progress`, `dimensions list`) skip the lock and run fine alongside the GUI.

## Commands

| Task | Command |
|------|---------|
| List entries for a date | `logbook-cli entries list --date 2026-07-11` |
| List entries as JSON | `logbook-cli --json entries list --date 2026-07-11` |
| List today's entries | `logbook-cli entries list --date $(date +%Y-%m-%d)` |
| Add an entry | `echo '<json>' \| logbook-cli entries add --date 2026-07-11` |
| Update an entry | `echo '<json>' \| logbook-cli entries update --date 2026-07-11 --entry-id <uuid>` |
| Delete an entry | `logbook-cli entries delete --date 2026-07-11 --entry-id <uuid>` |
| Check commitments progress | `logbook-cli commitments progress --year 2026 --month 7` |
| List commitments | `logbook-cli commitments list --year 2026 --month 7` |
| Set commitments | `echo '<yaml-or-json>' \| logbook-cli commitments set --year 2026 --month 7` |
| List dimensions (month) | `logbook-cli dimensions list --year 2026 --month 7` |
| List template dimensions | `logbook-cli dimensions list --template` |
| Set dimensions (month, YAML) | `echo '<yaml>' \| logbook-cli dimensions set --year 2026 --month 7` |
| Set template dimensions | `echo '<yaml>' \| logbook-cli dimensions set --template` |

`--year` and `--month` take plain integers (e.g. `--year 2026 --month 7`, not zero-padded).

## Entry rules

- **`duration` is a string, not a number.** It is parsed server-side by `parse_duration` — pass `"90"` for 90 minutes, or `"1h30m"` for 1 hour 30 minutes. Passing a bare number is rejected.
- **Write commands read JSON from stdin**, not from CLI arguments. Forgetting to pipe input causes the command to hang waiting for stdin. Always use `echo '<json>' | logbook-cli ...`.
- **`entries add` stdin shape:**

```json
{"item": "Refactor API", "duration": "90", "dimensions": {"role": "Dev"}}
```

  `dimensions` is optional, defaults to `{}`.

- **`entries update` stdin shape** — every field optional:

```json
{"item": "Updated description", "duration": "120", "dimensions": {"role": "Ops"}}
```

  Provide only the fields you want to change.

- Write commands run an integrity check on the data directory before writing. If the data is corrupted, the command fails immediately — this is the backend's authoritative validation, do not attempt to work around it.

## Commitments rules

- **`commitments set` accepts JSON or YAML** on stdin (JSON is tried first, then YAML).
- **`allocation` is in hours, not minutes.** `"allocation": 40` means 40 hours.
- **`commitments set` propagates renames to historical entries.** If you rename a goal or role, existing entries that referenced the old name are updated. Goals referenced by entries are protected from removal. This means `set` is not a config-only operation — it changes historical data.
- **stdin shape:**

```json
[{"role": "Dev", "allocation": 40, "goals": ["v2 launch", "infra"]}]
```

  Or YAML:

```yaml
- role: Dev
  allocation: 40
  goals:
    - v2 launch
    - infra
```

## Dimensions rules

- **Two different `--json` flags exist — do not confuse them:**
  - **Global `--json`** (placed *before* the subcommand): controls output format across all commands. E.g. `logbook-cli --json entries list --date ...`.
  - **Local `--json`** (placed *after* `dimensions list/set`): for `dimensions list` it means output JSON instead of YAML; for `dimensions set` it means input is JSON instead of YAML. E.g. `logbook-cli dimensions list --year 2026 --month 7 --json`.
- **`dimensions list` human output is YAML** (not the text format other commands use), and the month variant prepends a `# source: YYYY/MM/dimensions.yaml` comment line.
- **`dimensions set` reads YAML from stdin by default;** pass `--json` to feed JSON instead.
- **Dimension shape:**

```yaml
- name: "Role"
  key: "role"
  source: "static"
  values: ["Dev", "Ops"]
  required: false
  deleted: false
```

  `source` defaults to `"static"`. A goal-backed dimension uses `source: "commitments:<role-key>:goals"`.

## Listing and querying

- **`entries list` takes a single `--date`** — there is no range query. To cover multiple days, run the command for each date.
- **`commitments progress` human format:** `Role: X (NN% — Y.Yh / Zh)`. Add global `--json` for structured data.
- **Diagnostics go to stderr, data goes to stdout.** `Using data root:`, warnings, and errors are on stderr; only data output is on stdout — safe to pipe (e.g. `logbook-cli --json entries list ... | jq`).

## Security rules

- **Never guess `entry-id`.** Always run `entries list --date ...` first to get the real id before `update` or `delete`.
- **Never fabricate `duration`.** If the user did not state a duration, ask.
- **Confirm with the user before write operations** — show the exact content you are about to add or modify.
- **`commitments set` and `dimensions set` require explicit user confirmation.** They have wider blast radius than a single entry: `commitments set` propagates renames to historical entries; `dimensions set` changes the data structure for the month.
- **Deletion requires explicit confirmation.** `entries delete` is irreversible.
```

- [ ] **Step 3: Verify line count is under 500**

```bash
wc -l skill/logbook-cli/SKILL.md
```

Expected: under 500 lines.

- [ ] **Step 4: Commit**

```bash
git add skill/logbook-cli/SKILL.md
git commit -m "feat(skill): add logbook-cli agent skill"
```

---

### Task 2: Create `skill/logbook-cli/README.md`

**Files:**
- Create: `skill/logbook-cli/README.md`

**Interfaces:**
- Consumes: Task 1's `SKILL.md` (same directory, referenced by relative path).
- Produces: the install guide that Task 3's repo README links to.

- [ ] **Step 1: Write `skill/logbook-cli/README.md`**

Write this exact content:

```markdown
# logbook-cli Agent Skill

Companion skill for AI agents (ZCode, Claude Code, etc.) to operate Logbook time data via `logbook-cli`. The skill teaches the agent the correct command syntax, stdin input shapes, and the gotchas specific to logbook-cli (GUI lock, two `--json` flags, duration as string).

## Prerequisites

1. **logbook-cli installed.** Open the Logbook app → menu bar → **Install Command Line Tool…**
2. **`~/.local/bin` on your `PATH`.** The installer copies the binary there. Verify with `which logbook-cli`.

## Install

Copy this directory to your agent's skill discovery path:

```bash
cp -r skill/logbook-cli ~/.agents/skills/
```

For ZCode, use `.zcode/skills/` instead if you prefer workspace-level discovery (consult your agent's skill discovery priority).

Verify the skill is discovered by asking your agent something like "list my Logbook entries for today" — it should invoke `logbook-cli entries list`.
```

- [ ] **Step 2: Commit**

```bash
git add skill/logbook-cli/README.md
git commit -m "docs(skill): add logbook-cli skill install guide"
```

---

### Task 3: Add CLI section to repo `README.md`

**Files:**
- Modify: `README.md` (insert between the `## Development` section and the `## Documents` section)

**Interfaces:**
- Consumes: Task 1 and Task 2's `skill/logbook-cli/` directory (linked via relative path).

- [ ] **Step 1: Read current README.md**

```bash
cat README.md
```

Confirm the structure: `# Logbook` → tech line → design link → `## Development` → `## Documents`.

- [ ] **Step 2: Insert the `## CLI` section**

Insert this block between the `## Development` section (ending with the `pnpm tauri dev` code block) and the `## Documents` section:

```markdown
## CLI

Logbook ships with `logbook-cli`, a command-line tool for reading and
writing time data outside the GUI. Install it via the app menu:
**Logbook → Install Command Line Tool…**

An [Agent Skill](./skill/logbook-cli/) is available for AI agents
(ZCode, Claude Code, etc.) to operate your time data correctly.
```

- [ ] **Step 3: Verify the README renders correctly**

```bash
cat README.md
```

Expected: the `## CLI` section appears after `## Development` and before `## Documents`, the relative link `./skill/logbook-cli/` points to the directory created in Task 1.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: add CLI section pointing to agent skill"
```

---

### Task 4: Manual smoke test

**Files:** none (verification only)

This task validates that the SKILL.md content is accurate against the real CLI. No code changes.

- [ ] **Step 1: Verify the binary is available**

```bash
which logbook-cli
```

Expected: a path like `/Users/<user>/.local/bin/logbook-cli`.

- [ ] **Step 2: Verify GUI detection command works**

With the Logbook GUI running:

```bash
pgrep -fi "Logbook.app/Contents/MacOS" > /dev/null && echo "GUI_RUNNING" || echo "OK"
```

Expected: `GUI_RUNNING`.

If the GUI is not running, start it (`open -a Logbook` or via the app), then re-run to confirm `GUI_RUNNING`.

- [ ] **Step 3: Verify a read command works alongside GUI**

```bash
logbook-cli entries list --date $(date +%Y-%m-%d)
```

Expected: entries for today (or "no entries" message), runs without lock error despite GUI being open.

- [ ] **Step 4: Verify JSON output flag placement**

```bash
logbook-cli --json entries list --date $(date +%Y-%m-%d) | head -c 1
```

Expected: `{` (first character of JSON object). Confirms the global `--json` goes before the subcommand.

- [ ] **Step 5: Verify dimensions list local --json**

```bash
logbook-cli dimensions list --template
logbook-cli dimensions list --template --json | head -c 1
```

Expected: first command prints YAML; second command prints `{` (JSON). Confirms local `--json` flag.

- [ ] **Step 6: Record any discrepancies**

If any command does not behave as the SKILL.md documents, fix the SKILL.md before merging. Commit any fixes:

```bash
git add skill/logbook-cli/SKILL.md
git commit -m "fix(skill): correct <description of what was wrong>"
```
