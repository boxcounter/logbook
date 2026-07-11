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
