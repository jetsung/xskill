# XSkill

🌐 English | [简体中文](README_zh-CN.md)

A skills management tool for discovering, installing, and managing reusable agent skill packs.

## Features

- **Multi-source support** — Install skills from Git repositories, GitHub, GitLab, and more
- **Platform management** — Install to multiple AI coding platforms (Claude, Codex, etc.)
- **Lock file tracking** — Reproducible installs with `.xskill-lock.json`
- **Batch operations** — Install/remove all skills with `--all` flag
- **Cache support** — Optional local cache for offline skill queries

## Installation

```bash
git clone https://github.com/jetsung/xskill.git
cd xskill
cargo install --path .
```

Pre-built binaries are available on [GitHub Releases](https://github.com/jetsung/xskill/releases).

## Quick Start

```bash
# Add a skill source
xskill sources add -n my-skills -u https://github.com/example/skills

# Query available skills
xskill query -f my-skills

# Install a skill
xskill add -f my-skills -s vue

# Install to global directory
xskill add -f my-skills -s vue -g

# List installed skills
xskill list

# Interactively find and install a skill (requires cache)
xskill find

# Find with a pre-filled filter
xskill find --skill git

# Find from a specific source
xskill find --from antfu

# Remove a skill
xskill remove -s vue
```

## Commands

| Command | Description |
|---------|-------------|
| `sources` | Manage configured sources (list/add/remove/edit) |
| `platforms` | List configured platforms |
| `add` | Install a skill |
| `remove` | Remove a skill |
| `update` | Update installed skills from lock file |
| `restore` | Restore skills from project lock file |
| `list` | List installed skills |
| `query` | Query skills from a source |
| `find` | Interactively find and install a skill (TUI) |
| `rec` | Manage recommended skills (list/add/remove) |
| `cache` | Manage skills cache |
| `config` | Manage configuration |
| `new` | Create a new skill project |

## Configuration

Config file: `~/.xskill/settings.json` (override with `XSKILL_CONFIG` env var).

```json
{
  "$schema": "https://xskill.gcli.cn/xskill.schema.json",
  "platforms": {
    "claude": { "path": ".claude", "skills": "skills", "agents": "CLAUDE.md", "agents_compat": false },
    "codex": { "path": ".codex", "skills": "skills", "agents": "AGENTS.md", "agents_compat": true }
  },
  "sources": [
    { "name": "antfu", "type": "git", "url": "https://github.com/antfu/skills" }
  ],
  "recommended": [{ "name": "antfu", "skills": ["vue"] }],
  "cache": { "enabled": true, "ttl": 600 },
  "registry": { "enabled": false, "url": "https://xskill.gcli.cn/skills.json" }
}
```

## Documentation

📖 **[Full Documentation](docs/guide.md)** — Complete guide covering all commands, configuration options, lock file format, and more.

## License

[Apache License 2.0](https://github.com/jetsung/xskill/blob/main/LICENSE)
