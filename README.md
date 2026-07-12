# Logbook

Personal work time tracking app. Replaces Apple Numbers for daily work logging, dimension-based categorization, and monthly statistics.

**Tech**: Tauri 2.x + Vue 3 + TypeScript

**Design**: [Vault → `1_Projects/Logbook/README.md`](obsidian://open?vault=Everything&file=1_Projects/Logbook/README.md)

## Development

```
pnpm install
pnpm tauri dev
```

## CLI

Logbook ships with `logbook-cli`, a command-line tool for reading and
writing time data outside the GUI. Install it via the app menu:
**Logbook → Install Command Line Tool…**

An [Agent Skill](./skill/logbook-cli/) is available for AI agents
(ZCode, Claude Code, etc.) to operate your time data correctly.

## Documents

- [AGENTS.md](./AGENTS.md) — Project conventions, command dictionary, frontend architecture, data flow
