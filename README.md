# XSkill

🌐 English | [简体中文](README_zh-CN.md)

A skills management tool for discovering, installing, and managing reusable agent skill packs.

## Features

- **Multi-source support** — Install skills from Git repositories, GitHub, GitLab, and more
- **Platform management** — Install to multiple AI coding platforms (Claude, Codex, etc.)
- **Lock file tracking** — Reproducible installs with `.xskill-lock.json`
- **Batch operations** — Install/remove all skills with `--all` flag
- **Cache support** — Optional local cache for offline skill queries
- **Interactive TUI** — Multi-select skill finder with `find` command

## Installation

### Quick install

**Linux / macOS:**

```bash
curl -fsSL https://xskill.gcli.cn/install.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://xskill.gcli.cn/install.ps1 | iex
```

### From crates.io

```bash
cargo install xskill
```

### From Git

```bash
cargo install --git https://github.com/jetsung/xskill.git xskill
```

### From source

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

# Interactively find and install skills (multi-select, requires cache)
xskill find

# Find with a pre-filled filter
xskill find --skill git

# Find from a specific source
xskill find --from antfu

# Find and install globally
xskill find -g

# Link existing skills to a platform
xskill link -s vue -a claude

# Remove a skill
xskill remove -s vue
```

## Commands

| Command | Description |
|---------|-------------|
| `sources` | Manage configured sources (list/add/remove/edit) |
| `platforms` | List configured platforms |
| `add` | Install a skill |
| `link` | Symlink existing skills to a platform |
| `remove` | Remove a skill |
| `update` | Update installed skills from lock file |
| `restore` | Restore skills from project lock file |
| `list` | List installed skills |
| `query` | Query skills from a source |
| `find` | Interactively find and install skills (multi-select TUI) |
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
