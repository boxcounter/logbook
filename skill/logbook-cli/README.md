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
