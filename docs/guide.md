# XSkill

A skills management tool for discovering, installing, and managing reusable agent skill packs across multiple AI coding platforms.

## Features

- **Multi-source support**: Install skills from Git repositories, GitHub, GitLab, and any Git-compatible host
- **Platform management**: Install skills to multiple AI coding platforms (Claude, Codex, etc.) with a single command
- **Lock file tracking**: Track installed skills with `.xskill-lock.json` for reproducible installs
- **Global configuration**: Single global config at `~/.xskill/settings.json` with `XSKILL_CONFIG` override
- **Recursive search**: Automatically discover skills in nested directory structures within repositories
- **Batch operations**: Install or remove all skills with `--all` flag or `*` wildcard
- **Cache support**: Optional local cache for faster skill queries without network access
- **Interactive TUI**: Fuzzy-find and install skills interactively with `find` command
- **Cross-platform**: Works on Windows, macOS, and Linux

## Installation

### Quick install

**Linux / macOS:**

```bash
curl -fsSL https://xskill.gcli.cn/install.sh | bash
```

The script auto-detects OS and architecture, downloads the appropriate pre-built binary from GitHub Releases, and installs it to `~/.local/bin` (or `/usr/local/bin` for root).

**Windows (PowerShell):**

```powershell
irm https://xskill.gcli.cn/install.ps1 | iex
```

Auto-detects architecture, downloads from GitHub Releases, and installs to `%USERPROFILE%\.local\bin` (or `%ProgramFiles%\xskill\bin` for admin). The script will prompt you to add it to PATH if needed.

### From npm

```bash
npm install -g @jetsung/xskill
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

### Requirements

- Rust 1.70+
- Git (must be available in PATH)

### Pre-built binaries

Pre-built binaries for Linux, macOS, and Windows are available on the [GitHub Releases](https://github.com/jetsung/xskill/releases) page.

## Quick Start

### 1. Configure a source

Add a skill repository as a source:

```bash
xskill sources add -n my-skills -u https://github.com/example/skills
```

### 2. Query available skills

List skills from a source:

```bash
xskill query -f my-skills
```

Query a specific skill:

```bash
xskill query -f my-skills -s vue
```

### 3. Install a skill

Install to the project-level `.agents/` directory:

```bash
xskill add -f my-skills -s vue
```

Install to the global `~/.agents/` directory:

```bash
xskill add -f my-skills -s vue -g
```

### 4. List installed skills

```bash
xskill list
```

### 5. Remove a skill

```bash
xskill remove -s vue
```

## Commands

### `sources` — Manage configured sources

List, add, remove, or edit skill sources in the configuration.

#### `sources list`

List all configured sources:

```bash
xskill sources
# or explicitly:
xskill sources list
```

Output format:
```
NAME   TYPE URL
antfu  git  https://github.com/antfu/skills
```

#### `sources add`

Add a new source:

```bash
xskill sources add -n <name> -u <url> [-t git|api]
```

Options:
- `-n, --name` — Source name (optional, alphanumeric with `-` and `_`; when empty, the URL is used as the name)
- `-u, --url` — Source URL (required, must start with `http://` or `https://`)
- `-t, --type` — Source type: `git` or `api` (default: `git`)

#### `sources remove`

Remove a source by name and/or URL:

```bash
xskill sources remove -n <name>
xskill sources remove -u <url>
xskill sources remove -n <name> -u <url>
```

Options:
- `-n, --name` — Source name to remove (optional)
- `-u, --url` — Source URL to remove (optional)

At least one of `--name` or `--url` is required. When both are provided, both must match for the source to be removed.

#### `sources edit`

Rename an existing source (only the name can be changed; `url` and `type` are immutable):

```bash
xskill sources edit -n <name> -N <new-name>
```

Options:
- `-n, --name` — Current source name (or use `-u` to match by URL)
- `-u, --url` — Current source URL (alternative identifier)
- `-N, --new-name` — New name (required, pass empty string to clear the name)

### `platforms` — List configured platforms

List all configured AI coding platforms (sorted alphabetically):

```bash
xskill platforms
```

Show detailed platform information:

```bash
xskill platforms -a
```

Detailed output includes path, skills directory, agents file, source file, and agents compatibility (`COMPAT`) for each platform. Platforms with `agents_compat: true` display `✓` in the COMPAT column, indicating they can reuse project-level `.agents/` resources.

### `add` — Install a skill

Install a skill from a source to a target directory.

```bash
xskill add [OPTIONS] --from <SOURCE> --skill <SKILL>
```

Options:
- `-f, --from <SOURCE>` — Source name, `ORG/REPO`, or Git URL
- `-s, --skill <SKILL>` — Skill name (use `'*'` for all skills)
- `-g, --global` — Install to global `~/.agents/` directory
- `-a, --agent <AGENT>` — Target platform (use `'*'` for all platforms)
- `-A, --all` — Shorthand for `--skill '*' --agent '*'` (requires `--from`)

#### Install targets

| Flag | Target |
|------|--------|
| (none) | Project-level `.agents/skills/` |
| `-g` | Global `~/.agents/skills/` |
| `-a <platform>` | Platform-specific directory (e.g., `.claude/skills/`) |
| `-a '*'` | All configured platforms |

#### Source info display

Before installation, the source information is displayed: `Source: <source-name> (<source-url>)` (label in cyan bold).

#### Multi-source name collision

When `-f` is not specified and multiple sources (including registry) contain identically named skills:
- **Interactive terminal**: a skim single-select TUI is shown with three aligned columns: `[registry]` / `-` (first column — `[registry]` for registry entries, `-` for local sources), `source_name`, `url`. Registry entries show `-` as source name when empty or conflicting with a local source.
- **Non-interactive terminal**: an error is shown listing all matching sources (with URLs), suggesting `xskill add -f <source> -s <skill>`.

#### Output style

Labels (`Name`, `Description`, `Version`) are displayed in cyan bold. `Name` values are shown in yellow. Empty `Description` or `Version` lines are hidden.

- `Installed` (green): canonical directory path.
- `Symlinked` (green): platform directory path (no arrow or target shown, as `Installed` already displays the canonical path).
- `Source` (cyan bold): `Source: <name> (<url>)`.

#### Examples

```bash
# Install to project
xskill add -f antfu -s vue

# Install to global
xskill add -f antfu -s vue -g

# Install to a specific platform
xskill add -f antfu -s vue -a claude

# Install to all platforms
xskill add -f antfu -s vue -a '*'

# Install all skills from a source
xskill add -f antfu -s '*'

# Install everything everywhere
xskill add -f antfu -A
```

### `link` — Symlink existing skills to a platform

Symlink skills that already exist in the canonical directory to a specified platform directory. Unlike `add`, `link` does not download or install any skills from remote sources — it only operates on locally existing skills.

```bash
xskill link [OPTIONS] --skill <SKILL> --agent <AGENT>
```

Options:
- `-s, --skill <SKILL>` — Skill name (use `'*'` for all skills)
- `-a, --agent <AGENT>` — Target platform (use `'*'` for all platforms)
- `-g, --global` — Operate on global `~/.agents/skills/` directory
- `-A, --all` — Shorthand for `--skill '*' --agent '*'`

#### Link behavior

| Flag | Behavior |
|------|----------|
| `-s s1 -a codebuddy` | Symlink `.agents/skills/s1` to `.codebuddy/skills/s1` (auto-creates platform dir) |
| `-s s1 -a codebuddy -g` | Symlink `~/.agents/skills/s1` to `~/.codebuddy/skills/s1` |
| `-s s1 -a '*'` | Link s1 to all existing platform directories |
| `-s '*' -a claude` | Link all existing skills to claude platform |
| `-s '*' -a '*'` | Link all existing skills to all existing platforms |
| `-A` | Same as `-s '*' -a '*'` |

#### Key rules

- `link` does **not** require a `-f` parameter (no remote source involved).
- **No lock file update**: skills were already installed and recorded by `add`; `link` only creates symlinks.
- Skills must already exist in the canonical directory (with `SKILL.md`), otherwise an error is shown.
- `-s '*'` scans all subdirectories containing `SKILL.md` in the canonical directory.
- Symlinks use relative paths, following the same rules as `add`.
- Falls back to file copy if symlink creation fails.

#### agents_compat

Platforms with `agents_compat: true` are skipped (they read the canonical directory directly). Single platform: `Skipped: <name> (agents_compat)` (dimmed); `-a '*'`: silently skipped with summary.

#### Examples

```bash
# Link a single skill to a specific platform
xskill link -s vue -a claude

# Link all existing skills to a platform
xskill link -s '*' -a claude

# Link a skill to all platforms
xskill link -s vue -a '*'

# Link everything to everything
xskill link -A

# Global mode
xskill link -s vue -a claude -g
```

### `remove` — Remove a skill

Remove an installed skill and update the lock file.

```bash
xskill remove [OPTIONS] --skill <SKILL>
```

Options:
- `-s, --skill <SKILL>` — Skill name (use `'*'` for all skills)
- `-g, --global` — Remove from global directory
- `-a, --agent <AGENT>` — Target platform (use `'*'` for all platforms)
- `-A, --all` — Shorthand for `--skill '*' --agent '*'`

### `update` — Update installed skills

Re-install skills from lock file records, preserving the original `installed_at` timestamps.

```bash
xskill update [OPTIONS]
```

Options:
- `-g, --global` — Update global skills only
- `-s, --skill <SKILL>` — Skill name (use `'*'` for all skills)

Output style: labels (`Source`, `Updating`, `Name`, `Description`, `Version`, `Updated`) in cyan, `Name` value in yellow.

### `restore` — Restore skills from lock file

Read `.xskill-lock.json` from the current directory and install all recorded skills. Useful for setting up a new environment or restoring skills after cloning a project. Skills are grouped by `source_url` so each repository is cloned only once, avoiding redundant git operations.

```bash
xskill restore [OPTIONS]
```

Options:
- `-g, --global` — Install to global `~/.agents/skills/` directory (default: project-level `.agents/skills/`)
- `-a, --agent <AGENT>` — Target platform (use `'*'` for all platforms)
- `-D, --dry-run` — Preview mode: list skills to restore without installing

#### Install targets

| Flag | Target |
|------|--------|
| (none) | Project-level `.agents/skills/` |
| `-g` | Global `~/.agents/skills/` |
| `-a <platform>` | Platform-specific directory (e.g., `.claude/skills/`) |
| `-a '*'` | All configured platforms |

#### Examples

```bash
# Restore all skills to project
xskill restore

# Restore to global directory
xskill restore --global

# Restore to a specific platform
xskill restore --agent claude

# Preview what would be restored
xskill restore --dry-run
```

Output format:
```
Restoring: vue
  Source: https://github.com/antfu/skills.git
  Target: .agents/skills/vue
  Name: Vue
  Description: Vue.js skill pack

Restore complete: 3 succeeded, 0 failed
```

Dry-run output (grouped by skill name to avoid redundancy):
```
Skills to restore:

NAME   SOURCE                                      TARGET
vue    https://github.com/antfu/skills.git         .claude/skills/vue
                                                    .codex/skills/vue
react  https://github.com/antfu/skills.git         .claude/skills/react
                                                    .codex/skills/react
```

Color rules: multi-target (`-a '*'` or multiple platforms) — table header in blue, continuation TARGET entries in dimmed gray. Single target (`-a <name>`) — no color.

### `list` — List installed skills

Display installed skills with aligned columns.

```bash
xskill list [OPTIONS]
```

Options:
- `-g, --global` — List global skills
- `-a, --agent <AGENT>` — Filter by platform name

Output format:
```
Project Skills

vue     ~/.agents/skills/vue     Agents: codebuddy, gemini
react   ~/.agents/skills/react   Agents: codebuddy
```

- Skill names are displayed in yellow, paths in dimmed gray (`~/` prefix replaces home directory).
- `Agents:` prefix in dimmed gray, platform names in default white.
- With `-a <agent>`, skills not linked to that platform show `Agents: not symlinked` (`Agents:` dimmed gray, `not symlinked` yellow).
- Sorted by path alphabetically.

### `query` — Query skills from a source

Query or list skills from a configured or remote source.

```bash
xskill query [OPTIONS]
```

Options:
- `-f, --from <SOURCE>` — Source name, `ORG/REPO`, or Git URL
- `-s, --skill <SKILL>` — Specific skill name (required; wildcard `*` is not supported)

When `cache.enabled` is `true`, queries read from the local cache instead of fetching from the remote source. When `registry.enabled` is `true` and `--from` is not specified, the registry is also queried alongside configured sources.

Output style: labels (`Source`, `Registry`, `Name`, `Description`, `Version`, `Path`) in cyan bold, `Name` value in yellow. Empty `Source` shows `-`. Empty `Description` or `Version` lines are hidden. Each skill block is separated by a blank line.

When no skills are found and configured sources exist with `cache.enabled` true, a hint is displayed: `Hint: run 'xskill cache update' to refresh skills cache` (cyan).

### `find` — Interactively find and install skills

Launch a multi-step interactive TUI to search, configure, and install skills. Supports multi-select for batch installation, with source grouping to clone each repo only once.

```bash
xskill find [OPTIONS]
```

Options:
- `-f, --from <SOURCE>` — Filter skills by source name or URL
- `-s, --skill <QUERY>` — Pre-fill the filter query
- `-g, --global` — Install to global `~/.agents/` directory (default: project-level `.agents/`)

#### How it works

1. **Skill selection** — Multi-select substring search (exact mode) from cached skills. Display format: `name [source]` (registry entries show `name [registry] [source]`). Non-selected names use default color, selected names use blue. Source tags are always dark gray; `[registry]` tags turn green when selected. Search box at bottom, list arranged upward. Keyboard hints: `TAB: multi-select | enter confirm | esc cancel`. Press TAB to toggle multiple skills, Enter to confirm. If nothing is toggled, the cursor item is used.
2. **Platform selection** — Multi-select target platforms. First item is `Default` (disabled, means no platform symlinks). `agents_compat` platforms are excluded from the list and shown as `SELECTED: <platform1>, <platform2>, ...` in the header. Remaining items are non-compat configured platforms. Press TAB to select/deselect, Enter to confirm. Selected rows use blue text with dark background highlight.
3. **Install** — Skills are grouped by source URL; each repo is cloned once. For each skill, the correct path is extracted from `CachedSkill.path` (supports nested paths like `skills/engineering/grill/SKILL.md`). The skill is installed to the canonical directory (`.agents/skills/<name>` or `~/.agents/skills/<name>` with `-g`), then relative symlinks are created for each selected platform. Reports `Installed:`, `Symlinked:`, and any `Failed:` platforms. Each skill's output is separated by a blank line. Registry skills are cloned directly from their URL, independent of local `sources` configuration.

Press Esc or Ctrl-C at any step to cancel.

**Known issue:** skim library list row numbers are 0-based (skim 5.2.0 behavior), not 1-based.

#### Examples

```bash
# Open the interactive finder
xskill find

# Pre-filter for skills matching "git"
xskill find --skill git

# Find only from a specific source
xskill find --from antfu

# Find from a URL (auto-cached for 10 minutes)
xskill find --from https://github.com/example/skills
```

**Note:** Requires a populated cache. Run `xskill cache update` first if you haven't already. When using a URL with `--from`, the skill list is fetched and cached automatically.

### `rec` — Manage recommended skills

Manage recommended skills sources with list, add, and remove operations.

```bash
xskill rec <COMMAND>
```

#### `rec list`

List all recommended sources:

```bash
xskill rec list
```

Output format:
```
SOURCE  NAME   URL                                  SKILLS
true    antfu  https://github.com/antfu/skills       vue, react
false   foo    invalid                              bar
```

- `SOURCE` column: `true` if the name matches a configured source with consistent URL, `false` otherwise.
- `URL` shows `invalid` (red) when the name exists in sources but URL doesn't match, or when no URL can be resolved.

#### `rec add`

Add skills to a recommended source. If the entry already exists, new skills are appended (duplicates ignored).

```bash
xskill rec add [-n <name>] [-u <url>] -s <skills>
```

Options:
- `-n, --name` — Source name (must exist in sources if `--url` not provided)
- `-u, --url` — Source URL (when name exists in sources and url matches, only name is saved)
- `-s, --skills` — Comma-separated list of skill names (required)

Parameter combination logic:
- **Only `-n` and `-s`**: Validate `-n` exists in sources, save name + skills
- **`-n`, `-u`, and `-s`**:
  - If `-n` exists in sources AND url matches `-u`: save only name + skills (no url needed)
  - If `-n` exists in sources BUT url doesn't match: error
  - If `-n` doesn't exist in sources: save url + skills (name becomes url)
- **Only `-u` and `-s`**: Save url + skills

Append behavior: If entry "antfu" already has skills `vue`, running `rec add -n antfu -s react,angular` results in `vue,react,angular`.

#### `rec remove`

Remove a recommended source or specific skills:

```bash
xskill rec remove [-n <name>] [-u <url>] [-s <skills>]
```

Options:
- `-n, --name` — Source name (used to identify entry, or with `-u`/`-s` for specific removal)
- `-u, --url` — Source URL (when both `-n` and `-u` provided, `-u` takes priority)
- `-s, --skills` — Comma-separated list of skill names to remove (removes specific skills instead of entire entry)

Priority logic:
- When both `-n` and `-u` provided: prioritize `-u` (fallback to `-n` if url not found)
- When only `-n`: delete entire entry with that name
- When `-n` and `-s`: delete specific skills from entry with that name
- When `-u` and `-s`: delete specific skills from entry with that url

### `cache` — Manage skills cache

Manage the local skills cache for offline queries.

```bash
xskill cache <COMMAND>
```

#### `cache update`

Fetch skills list from remote sources and save to cache:

```bash
xskill cache update [-f <source>]
```

Options:
- `-f, --from <source>` — Update a specific source only (name or URL)

Per-source output: `<source_name>: <count> skills`. Summary: `Cache updated: N sources, M skills total`.

#### `cache clear`

Clear cached skills data:

```bash
xskill cache clear [-f <source>]
```

Options:
- `-f, --from <source>` — Clear a specific source only (name or URL)

### `config` — Manage configuration

View or modify the global configuration file.

```bash
xskill config [OPTIONS]
```

Options:
- `-i, --init` — Initialize config file with default values (default platforms, cache, registry)
- `-e, --edit` — Open config in `$EDITOR` (defaults to `vi`)
- `-g, --get <key>` — Get a config value by dot path (e.g., `cache.enabled`)
- `-s, --set <key=value>` — Set a config value by dot path (e.g., `cache.enabled=true`)

#### Examples

```bash
# Initialize config with defaults
xskill config --init

# Open config in editor
xskill config --edit

# Read a value
xskill config --get cache.enabled

# Set a value
xskill config --set cache.enabled=true
```

### `new` — Create a skill project

Create a new skill project with a template `SKILL.md`.

```bash
xskill new --name <name> [--description <desc>] [--template <template>]
```

Options:
- `-n, --name <name>` — Skill name (required, used as directory name)
- `-d, --description <desc>` — Skill description
- `-t, --template <template>` — Template type

## Configuration

### Config file location

| Path | Description |
|------|-------------|
| `~/.xskill/settings.json` | Global config (default) |
| `XSKILL_CONFIG` env var | Override config path (must point to a JSON file) |

There is no project-level configuration. Only one global config file is used.

### Configuration structure

```json
{
  "$schema": "https://xskill.gcli.cn/xskill.schema.json",
  "platforms": { ... },
  "sources": [ ... ],
  "recommended": [ ... ],
  "cache": { ... },
  "registry": { ... }
}
```

### Full example

```json
{
  "$schema": "https://xskill.gcli.cn/xskill.schema.json",
  "platforms": {
    "claude": {
      "path": ".claude",
      "skills": "skills",
      "agents": "CLAUDE.md",
      "agents_compat": false
    },
    "codex": {
      "path": ".codex",
      "skills": "skills",
      "agents": "AGENTS.md",
      "agents_compat": true
    }
  },
  "sources": [
    {
      "name": "antfu",
      "type": "git",
      "url": "https://github.com/antfu/skills"
    },
    {
      "name": "mattpocock",
      "url": "https://github.com/mattpocock/skills"
    }
  ],
  "recommended": [
    {
      "name": "antfu",
      "skills": ["vue", "react"]
    }
  ],
  "cache": {
    "enabled": true,
    "ttl": 600
  },
  "registry": {
    "enabled": false,
    "url": "https://xskill.gcli.cn/skills.json"
  }
}
```

### Platforms

Each platform entry configures how skills are installed for a specific AI coding tool.

#### Platform fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `path` | Yes | — | Tool config directory (relative, absolute, or `~/...`) |
| `skills` | No | — | Skills subdirectory name relative to `path`. Omit to skip skill installation |
| `agents` | No | — | Agents config file name relative to `path`. Omit to skip agents installation |
| `source` | No | `"AGENTS.md"` | Source file name under the fixed `.agents/` directory |
| `agents_compat` | No | `false` | Whether this platform can reuse `.agents/` resources. When `true`, the platform reads directly from the canonical directory — add/remove/link/restore skip symlink operations (single platform: `Skipped` output; `-a '*'`: silent). find TUI lists and selects normally, silently skips symlink during install. list `-a` shows all canonical skills as linked. |

#### Symlink behavior

The `agents` file is symlinked to a source file under `.agents/`:

```
<path>/<agents>  →  .agents/<source>
```

For example, with `agents: "AGENTS.md"` and `source: "AGENTS.md"` (default):
```
.codex/AGENTS.md  →  .agents/AGENTS.md
```

With `agents: "AGENTS.md"` and `source: "CLAUDE.md"`:
```
.codex/AGENTS.md  →  .agents/CLAUDE.md
```

### Sources

Sources define where skills are fetched from.

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | No | — | Unique identifier (alphanumeric, `-`, `_`); when empty or invalid, the URL is used as the name |
| `type` | No | `"git"` | Source type: `git` or `api` |
| `url` | Yes | — | Repository URL (must start with `http://` or `https://`) |

### Recommended

Recommended skills are managed by the `rec` command for easy installation.

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Source name (must match a configured source) |
| `skills` | Yes | Array of skill names to recommend |

### Cache

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `cache.enabled` | No | `false` | Enable local skills cache for `query` and `find` commands |
| `cache.ttl` | No | `600` | Cache time-to-live in seconds (default: 10 minutes). Applies to both the main cache (`skills.json`) and URL cache (`source_<md5>.json`) |

When enabled, `xskill cache update` fetches skill metadata from all sources and stores it locally. Subsequent `query` and `find` commands check `cache.ttl` for staleness: if the cache is fresh, it is used directly; if stale or empty with configured sources, sources are re-cloned and the cache is automatically refreshed.

### Registry

The registry is an optional JSON API that provides a curated skill index. When enabled, `query` and `find` commands will also query the registry alongside configured sources.

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `registry.enabled` | No | `false` | Enable registry lookup |
| `registry.url` | No | `https://xskill.gcli.cn/skills.json` | Registry URL |

URL resolution rules:
- Bare domain or trailing `/` → auto-append `/skills.json`
- Path ending with a file extension (e.g. `.json`) → use as-is
- Empty or invalid protocol → fall back to built-in default

Deduplication (URL-normalized, local takes priority):
- If a registry source has the same **URL** as a configured source → skipped (local config wins).
- If a registry source has the same **name** but a **different URL** → kept as a separate source; registry entry's source name is blanked (shown as `-` in `query`, or the source URL in `find`).
- No conflict → displayed normally.
- **Skill-level dedup**: only skipped when URLs match. Different URLs with same skill name are kept as separate entries.

Examples:
```bash
# Enable registry
xskill config --set registry.enabled=true

# Use a custom registry URL (bare domain)
xskill config --set registry.url=https://example.com

# Use a custom registry URL (with path)
xskill config --set registry.url=https://example.com/api/v1/
```

### `--from` parameter resolution

The `-f` / `--from` parameter is resolved in this order:

1. **Git URL**: If the value starts with `http://` or `https://`, use it directly
2. **Config name**: Match against configured source names
3. **GitHub shorthand**: If the value contains `/` (e.g., `ORG/REPO`), expand to `https://github.com/ORG/REPO.git`
4. **Error**: If none of the above match, report "source not found"

### `--skill` parameter

The `-s` / `--skill` parameter accepts:
- A specific skill name (e.g., `vue`) — **exact match** only, no fuzzy or substring matching
- The wildcard `*` to match all skills

### `--agent` validation

When `-a` / `--agent` specifies a platform name (not `*`), it must exist in the configured `platforms`. Otherwise, the following error is displayed:

```
Invalid agents: <input>           (yellow)
Valid agents: platform1, platform2, ...  (bright black)
```

### URL normalization

All URL-related operations normalize URLs by stripping the `.git` suffix before comparison or caching. This applies to `cache update --from`, `query --from`, `find --from`, and URL cache file name generation (`source_<md5>.json`). For example, `https://github.com/org/repo.git` and `https://github.com/org/repo` are treated as the same URL.

## Installation Model: Canonical Directory + Symlinks

Skills are stored in a **canonical directory** (`.agents/skills/`), and each platform directory links to it via **relative symlinks**.

```
.agents/skills/my-skill/          ← actual files (canonical)
.codebuddy/skills/my-skill/       → symlink → ../../.agents/skills/my-skill/
.gemini/skills/my-skill/          → symlink → ../../.agents/skills/my-skill/
```

### Global vs local paths

| Mode | Canonical directory | Platform directory examples |
|------|-------------------|---------------------------|
| `-g` (global) | `~/.agents/skills/` | `~/.codebuddy/skills/`, `~/.gemini/skills/` |
| Local (default) | `./.agents/skills/` | `./.codebuddy/skills/`, `./.gemini/skills/` |

### Symlink rules

- **Relative paths**: `relative(platform_skills_dir, canonical_skill_dir)` for portability.
- **Idempotent**: existing link pointing to the same target → skipped.
- **Update**: existing link pointing to a different target → deleted and recreated.
- **Auto-create parents**: `mkdir -p` ensures platform skills subdirectories exist.
- **Cross-platform**: Windows uses junctions, Unix uses symlinks.

### Fallback mechanism

```
Preferred: symlink
  ↓ fails
Fallback: copy (file duplication)
```

If symlink creation fails, the target directory is cleaned and `copy_dir_recursive` is used instead.

### Platform directory behavior

| Scenario | When platform directory doesn't exist |
|----------|--------------------------------------|
| `-a <name>` (specific platform) | **Created** automatically, then linked |
| `-a '*'` (all platforms) | **Skipped** — no directory created, no link |

## Lock File

The lock file tracks installed skills for reproducibility.

### Locations

| Path | Scope |
|------|-------|
| `./.xskill-lock.json` | Project-level |
| `~/.agents/.xskill-lock.json` | Global |

### Format

```json
{
  "version": 1,
  "skills": {
    "vue": {
      "source": "antfu",
      "source_type": "git",
      "source_url": "https://github.com/antfu/skills.git",
      "skill_path": "skills/vue/SKILL.md",
      "skill_folder_hash": "abc123...",
      "installed_at": "2026-07-15T18:16:42.852Z",
      "updated_at": "2026-07-15T18:16:42.852Z"
    }
  },
  "updated_at": "2026-07-15T18:16:42.852Z"
}
```

### Entry Fields

| Field | Description |
|-------|-------------|
| `source` | Source name from configuration |
| `source_type` | Source type (`git`) |
| `source_url` | Full repository URL |
| `skill_path` | Relative path to `SKILL.md` within the repo |
| `skill_folder_hash` | Git tree hash of the skill folder for change detection |
| `installed_at` | ISO 8601 timestamp of first installation (`YYYY-MM-DDTHH:MM:SS.sssZ`) |
| `updated_at` | ISO 8601 timestamp of last update for this skill (`YYYY-MM-DDTHH:MM:SS.sssZ`) |

### Top-level Fields

| Field | Description |
|-------|-------------|
| `version` | Lock file format version (always `1`) |
| `updated_at` | ISO 8601 timestamp of last lock file modification (any skill add/update/remove) |

The `update` command uses lock file records to re-fetch skills while preserving the original `installed_at` timestamp.

The `restore` command reads from the project lock file and writes back to the same lock file scope (project-level by default, global with `-g`), updating `skill_folder_hash` and both `updated_at` fields while preserving `installed_at`.

## JSON Schemas

Two JSON Schemas are provided in the [`schemas/`](https://github.com/jetsung/xskill/tree/main/schemas) directory and hosted at `xskill.gcli.cn`.

### `xskill.schema.json` — Tool configuration

For `~/.xskill/settings.json`. Defines the full configuration structure.

**Top-level fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `$schema` | `string` | No | JSON Schema URL for editor validation. Auto-generated by `config --init` |
| `platforms` | `object<string, Platform>` | No | Platform configurations keyed by platform identifier (e.g. `"claude"`, `"codex"`) |
| `sources` | `Source[]` | No | Skill source repositories |
| `recommended` | `RecommendedSource[]` | No | Recommended skill sets grouped by source |
| `cache` | `CacheConfig` | No | Cache settings for skills list caching |
| `registry` | `RegistryConfig` | No | Registry settings for skill discovery |

**Platform** (`platforms.*`):

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | `string` | Yes | — | Tool config directory (relative, absolute, or `~/...` path). Minimum 1 character |
| `skills` | `string` | No | `""` | Skills subdirectory name relative to `path`. Empty string skips skill installation |
| `agents` | `string` | No | `""` | Agents config file name relative to `path`. Empty string skips agents installation |
| `source` | `string` | No | `"AGENTS.md"` | Source file name under the fixed `.agents/` directory. `<path>/<agents>` is symlinked to `.agents/<source>` |
| `agents_compat` | `boolean` | No | `false` | Whether this platform can reuse `.agents/` resources. When `true`, reads directly from canonical directory — symlink operations are skipped |

**Source** (`sources[]`):

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | `string` | No | `""` | Unique source identifier. Pattern: `^[a-zA-Z0-9_-]+$`. When empty or invalid, the `url` is used as the name |
| `type` | `string` | No | `"git"` | Source type. Enum: `"git"`, `"api"` |
| `url` | `string` | Yes | — | Source repository URL. Must be a valid URI starting with `http://` or `https://` |

**RecommendedSource** (`recommended[]`):

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | `string` | No | `""` | Source name referencing a `sources` entry, or custom label |
| `url` | `string` | No | `""` | Direct source URL (overrides name reference when name not found in sources) |
| `skills` | `string[]` | Yes | — | List of recommended skill names. Minimum 1 item |

**CacheConfig** (`cache`):

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `enabled` | `boolean` | No | `false` | Enable skills list caching. When enabled, `query` and `find` read from local cache |
| `ttl` | `integer` | No | `600` | Cache time-to-live in seconds (default 10 minutes). Applies to both main cache (`skills.json`) and URL cache (`source_<md5>.json`). Minimum: 0 |

**RegistryConfig** (`registry`):

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `enabled` | `boolean` | No | `false` | Enable registry lookup. When enabled, `query` and `find` also query the registry |
| `url` | `string` | No | `"https://xskill.gcli.cn/skills.json"` | Registry URL. Supports bare domain, directory path, or full file path |

### `registry.schema.json` — Registry index

For the registry API response (`skills.json`). Defines the skills index data structure.

**Top-level fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `updated_at` | `string` | Yes | ISO 8601 timestamp of last update (e.g. `2026-07-17T12:00:00.000Z`) |
| `sources` | `SourceEntry[]` | Yes | Skills grouped by source repository |

**SourceEntry** (`sources[]`):

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `source` | `string` | Yes | Source name (e.g. `org/repo`) |
| `url` | `string` | Yes | Source repository URL |
| `skills` | `SkillEntry[]` | Yes | Skills available from this source |

**SkillEntry** (`sources[].skills[]`):

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | `string` | Yes | — | Skill name |
| `path` | `string` | Yes | — | Path to `SKILL.md` relative to repository root |
| `description` | `string` | No | `""` | Skill description |
| `version` | `string` | No | `""` | Skill version |

### Editor integration

`xskill config --init` automatically adds a `$schema` field to `settings.json`:

```json
{
  "$schema": "https://xskill.gcli.cn/xskill.schema.json",
  ...
}
```

Most JSON editors (VSCode, Neovim with jsonls, etc.) will automatically load the schema from this URL and provide validation and autocompletion.

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Project Structure

```
xskill/
├── Cargo.toml
├── README.md
├── schemas/                # JSON Schema definitions
│   ├── xskill.schema.json    # settings.json schema
│   └── registry.schema.json  # registry index schema
├── docs/                   # Documentation source (mdbook input)
│   ├── SPEC.md             # Requirements specification
├── book/                   # mdbook output (generated)
│   ├── en/
│   └── zh/
├── crates/
│   └── generate-book/      # mdbook content generator
├── src/
│   ├── main.rs             # CLI entry point (clap derive)
│   ├── config.rs           # Configuration handling
│   ├── git.rs              # Git operations (clone, sparse checkout)
│   ├── lock.rs             # Lock file management
│   ├── skill_meta.rs       # SKILL.md frontmatter parsing
│   ├── cache.rs            # Cache data structures
│   ├── utils.rs            # Utility functions
│   └── commands/
│       ├── add.rs          # Install skills
│       ├── link.rs         # Symlink existing skills to platforms
│       ├── remove.rs       # Remove skills
│       ├── update.rs       # Update from lock file
│       ├── restore.rs      # Restore from lock file
│       ├── list.rs         # List installed skills
│       ├── find.rs         # Interactive TUI skill finder
│       ├── query.rs        # Query remote/cache skills
│       ├── sources.rs      # Manage sources (CRUD)
│       ├── platforms.rs    # List platforms
│       ├── rec.rs          # Manage recommended skills (list/add/remove)
│       ├── cache.rs        # Cache management
│       ├── config.rs       # Config management
│       └── new.rs          # Create skill project
```

## License

[Apache License 2.0](https://github.com/jetsung/xskill/blob/main/LICENSE)
