# XSkill

一个用于发现、安装和管理可复用 agent 技能包的技能管理工具，支持多种 AI 编程平台。

## 特性

- **多源支持**：从 Git 仓库、GitHub、GitLab 及任何 Git 兼容主机安装技能
- **平台管理**：一条命令将技能安装到多个 AI 编程平台（Claude、Codex 等）
- **锁文件追踪**：通过 `.xskill-lock.json` 追踪已安装技能，确保可复现安装
- **全局配置**：单一全局配置文件 `~/.xskill/settings.json`，支持 `XSKILL_CONFIG` 环境变量覆盖
- **递归搜索**：自动发现仓库中嵌套目录结构下的技能
- **批量操作**：使用 `--all` 标志或 `*` 通配符安装或移除所有技能
- **缓存支持**：可选的本地缓存，实现无网络访问的快速技能查询
- **交互式 TUI**：通过 `find` 命令模糊搜索并交互式安装技能
- **跨平台**：支持 Windows、macOS 和 Linux

## 安装

### 从源码构建

```bash
git clone https://github.com/jetsung/xskill.git
cd xskill
cargo build --release
cargo install --path .
```

### 环境要求

- Rust 1.70+
- Git（需在 PATH 中可用）

### 预编译二进制

[GitHub Releases](https://github.com/jetsung/xskill/releases) 页面提供 Linux、macOS 和 Windows 的预编译二进制文件。

## 快速开始

### 1. 配置源

添加技能仓库作为源：

```bash
xskill sources add -n my-skills -u https://github.com/example/skills
```

### 2. 查询可用技能

列出源中的技能：

```bash
xskill query -f my-skills
```

查询特定技能：

```bash
xskill query -f my-skills -s vue
```

### 3. 安装技能

安装到项目级 `.agents/` 目录：

```bash
xskill add -f my-skills -s vue
```

安装到全局 `~/.agents/` 目录：

```bash
xskill add -f my-skills -s vue -g
```

### 4. 列出已安装技能

```bash
xskill list
```

### 5. 移除技能

```bash
xskill remove -s vue
```

## 命令

### `sources` — 管理配置源

列出、添加、移除或编辑配置中的技能源。

#### `sources list`

列出所有已配置的源：

```bash
xskill sources
# 或显式调用：
xskill sources list
```

输出格式：
```
NAME   TYPE URL
antfu  git  https://github.com/antfu/skills
```

#### `sources add`

添加新源：

```bash
xskill sources add -n <name> -u <url> [-t git|api]
```

选项：
- `-n, --name` — 源名称（可选，仅允许字母数字、`-` 和 `_`；留空时自动使用 URL 作为名称）
- `-u, --url` — 源地址（必填，须以 `http://` 或 `https://` 开头）
- `-t, --type` — 源类型：`git` 或 `api`（默认：`git`）

#### `sources remove`

按名称和/或 URL 移除源：

```bash
xskill sources remove -n <name>
xskill sources remove -u <url>
xskill sources remove -n <name> -u <url>
```

选项：
- `-n, --name` — 要移除的源名称（可选）
- `-u, --url` — 要移除的源地址（可选）

至少需指定 `--name` 或 `--url` 之一。同时指定时两者都匹配才删除。

#### `sources edit`

重命名已有源（仅允许修改名称，`url` 和 `type` 不可变更）：

```bash
xskill sources edit -n <name> -N <new-name>
```

选项：
- `-n, --name` — 当前源名称（或使用 `-u` 按 URL 匹配）
- `-u, --url` — 当前源地址（替代标识符）
- `-N, --new-name` — 新名称（必填，传空字符串清空名称）

### `platforms` — 列出配置平台

列出所有已配置的 AI 编程平台：

```bash
xskill platforms
```

显示详细平台信息：

```bash
xskill platforms -a
```

详细输出包含每个平台的路径、技能目录、代理文件、源文件和 agents 兼容性（`COMPAT`）信息。`agents_compat: true` 的平台在 COMPAT 列显示 `✓`，表示可复用项目级 `.agents/` 资源。

### `add` — 安装技能

从源安装技能到目标目录。

```bash
xskill add [OPTIONS] --from <SOURCE> --skill <SKILL>
```

选项：
- `-f, --from <SOURCE>` — 源名称、`ORG/REPO` 或 Git URL
- `-s, --skill <SKILL>` — 技能名称（使用 `'*'` 表示所有技能）
- `-g, --global` — 安装到全局 `~/.agents/` 目录
- `-a, --agent <AGENT>` — 目标平台（使用 `'*'` 表示所有平台）
- `-A, --all` — `--skill '*' --agent '*'` 的简写（需配合 `--from`）

#### 安装目标

| 标志 | 目标 |
|------|------|
| （无） | 项目级 `.agents/skills/` |
| `-g` | 全局 `~/.agents/skills/` |
| `-a <platform>` | 平台特定目录（如 `.claude/skills/`） |
| `-a '*'` | 所有已配置平台 |

#### 源信息显示

安装前会输出源信息：`Source: <source-name> (<source-url>)`（标签 cyan bold）。

#### 多源同名技能选择

当未指定 `-f` 且多个源（含注册中心）包含同名技能时：
- **交互终端**：弹出 skim 单选 TUI，三列对齐显示：`[registry]` / `-`（第一列，注册中心条目显示 `[registry]`，本地源显示 `-`）、`source_name`、`url`。注册中心 source_name 为空或与本地源冲突时显示 `-`。
- **非交互终端**：报错并列出所有匹配源（含 URL），提示使用 `xskill add -f <source> -s <skill>`。

#### 输出样式

标签（`Name`、`Description`、`Version`）使用 cyan bold 显示。`Name` 值使用黄色显示。`Description` 或 `Version` 为空时不显示该行。

- `Installed`（green）：规范目录路径。
- `Symlinked`（green）：平台目录路径（不显示箭头和目标，因 `Installed` 行已展示规范目录）。
- `Source`（cyan bold）：`Source: <name> (<url>)`。

#### 示例

```bash
# 安装到项目
xskill add -f antfu -s vue

# 安装到全局
xskill add -f antfu -s vue -g

# 安装到特定平台
xskill add -f antfu -s vue -a claude

# 安装到所有平台
xskill add -f antfu -s vue -a '*'

# 安装源中所有技能
xskill add -f antfu -s '*'

# 安装到所有位置
xskill add -f antfu -A
```

### `remove` — 移除技能

移除已安装的技能并更新锁文件。

```bash
xskill remove [OPTIONS] --skill <SKILL>
```

选项：
- `-s, --skill <SKILL>` — 技能名称（使用 `'*'` 表示所有技能）
- `-g, --global` — 从全局目录移除
- `-a, --agent <AGENT>` — 目标平台（使用 `'*'` 表示所有平台）
- `-A, --all` — `--skill '*' --agent '*'` 的简写

### `update` — 更新已安装技能

根据锁文件记录重新安装技能，保留原始 `installed_at` 时间戳。

```bash
xskill update [OPTIONS]
```

选项：
- `-g, --global` — 仅更新全局技能
- `-s, --skill <SKILL>` — 技能名称（使用 `'*'` 表示所有技能）

输出样式：标签（`Source`、`Updating`、`Name`、`Description`、`Version`、`Updated`）使用 cyan 显示，`Name` 值使用黄色。

### `restore` — 从锁文件恢复技能

读取当前目录下的 `.xskill-lock.json`，安装所有已记录的技能。适用于新环境搭建或克隆项目后快速恢复技能。恢复时按 `source_url` 分组，同一仓库仅克隆一次，避免重复 git 操作。

```bash
xskill restore [OPTIONS]
```

选项：
- `-g, --global` — 安装到全局 `~/.agents/skills/` 目录（默认：项目级 `.agents/skills/`）
- `-a, --agent <AGENT>` — 目标平台（使用 `'*'` 表示所有平台）
- `-D, --dry-run` — 预览模式：列出将要恢复的技能，不执行安装

#### 安装目标

| 标志 | 目标 |
|------|------|
| （无） | 项目级 `.agents/skills/` |
| `-g` | 全局 `~/.agents/skills/` |
| `-a <platform>` | 平台特定目录（如 `.claude/skills/`） |
| `-a '*'` | 所有已配置平台 |

#### 示例

```bash
# 恢复所有技能到项目
xskill restore

# 恢复到全局目录
xskill restore --global

# 恢复到特定平台
xskill restore --agent claude

# 预览将要恢复的内容
xskill restore --dry-run
```

输出格式：
```
Restoring: vue
  Source: https://github.com/antfu/skills.git
  Target: .agents/skills/vue
  Name: Vue
  Description: Vue.js 技能包

Restore complete: 3 succeeded, 0 failed
```

Dry-run 输出（按技能名称分组，避免重复）：
```
Skills to restore:

NAME   SOURCE                                      TARGET
vue    https://github.com/antfu/skills.git         .claude/skills/vue
                                                    .codex/skills/vue
react  https://github.com/antfu/skills.git         .claude/skills/react
                                                    .codex/skills/react
```

颜色规则：多目标（`-a '*'` 或多个平台）时表头蓝色、续行 TARGET 黑灰色。单目标（`-a <name>`）时无颜色。

### `list` — 列出已安装技能

以对齐列格式显示已安装的技能。

```bash
xskill list [OPTIONS]
```

选项：
- `-g, --global` — 列出全局技能
- `-a, --agent <AGENT>` — 按平台名称筛选

输出格式：
```
Project Skills

vue     ~/.agents/skills/vue     Agents: codebuddy, gemini
react   ~/.agents/skills/react   Agents: codebuddy
```

- 技能名称显示为黄色，路径显示为暗灰色（使用 `~/` 前缀替代 home 目录）。
- `Agents:` 前缀为黑灰色，平台名为默认白色。
- 使用 `-a <agent>` 过滤时，未链接到指定平台的技能显示 `Agents: not symlinked`（`Agents:` 黑灰色，`not symlinked` 黄色）。
- 按路径字母顺序排序。

### `query` — 查询源中的技能

从已配置或远程源查询或列出技能。

```bash
xskill query [OPTIONS]
```

选项：
- `-f, --from <SOURCE>` — 源名称、`ORG/REPO` 或 Git URL
- `-s, --skill <SKILL>` — 特定技能名称（必填，不支持通配符 `*`）

当 `cache.enabled` 为 `true` 时，查询从本地缓存读取，而非从远程源获取。当 `registry.enabled` 为 `true` 且未指定 `--from` 时，还会额外查询注册中心。

输出样式：标签（`Source`、`Registry`、`Name`、`Description`、`Version`、`Path`）使用 cyan bold 显示，`Name` 值使用黄色。`Source` 为空时显示 `-`。`Description` 或 `Version` 为空时不显示该行。各技能之间以空行分隔。

当未找到技能但已配置源且 `cache.enabled` 为 `true` 时，显示提示：`Hint: run 'xskill cache update' to refresh skills cache`（cyan）。

### `find` — 交互式查找并安装技能

启动多步交互式 TUI，依次查找技能、选择安装范围和目标平台后安装。

```bash
xskill find [OPTIONS]
```

选项：
- `-f, --from <SOURCE>` — 按源名称或 URL 过滤技能
- `-s, --skill <QUERY>` — 预填充过滤查询
- `-g, --global` — 安装到全局 `~/.agents/` 目录（默认：项目级 `.agents/`）

#### 工作流程

1. **选择技能** — 从缓存中子串搜索（exact 模式）。显示格式：`name [source]`（注册中心条目显示 `name [registry] [source]`）。非选中行技能名称使用默认色，选中行使用蓝色。source 标签始终暗灰色；`[registry]` 标签选中时变为绿色。搜索框在底部，列表向上排列。快捷键提示：`up/down navigate | enter select | esc cancel`。匹配计数格式为 `当前/总数`（如 `2/2`）。
2. **选择目标平台** — TUI 多选。首项为 `Default`（不可选中，表示不创建平台符号链接）。后续为配置中所有平台。按 TAB 选择/取消选择，Enter 确认。选中行使用蓝色文字和深色背景高亮。
3. **安装** — 将技能安装到规范目录（`.agents/skills/<name>` 或 `-g` 时 `~/.agents/skills/<name>`），然后为每个选中的平台创建相对符号链接。输出 `Installed:`、`Symlinked:` 和失败的平台（如有）。

任意步骤按 Esc 或 Ctrl-C 取消。

**已知问题**：skim 库的列表行号从 0 开始（skim 5.2.0 行为），非本项目可控。

#### 示例

```bash
# 打开交互式查找器
xskill find

# 预过滤匹配 "git" 的技能
xskill find --skill git

# 从指定源查找
xskill find --from antfu

# 从 URL 查找（自动缓存 10 分钟）
xskill find --from https://github.com/example/skills
```

**注意：** 需要已填充的缓存。如尚未更新缓存，请先运行 `xskill cache update`。使用 URL 方式的 `--from` 时，技能列表会自动拉取并缓存。

### `rec` — 管理推荐技能

管理推荐技能源，支持列表、添加和移除操作。

```bash
xskill rec <COMMAND>
```

#### `rec list`

列出所有推荐源：

```bash
xskill rec list
```

输出格式：
```
SOURCE  NAME   URL                                  SKILLS
true    antfu  https://github.com/antfu/skills       vue, react
false   foo    invalid                              bar
```

- `SOURCE` 列：`true` 表示该推荐源的名称存在于 `sources` 配置中且 URL 一致，`false` 表示不匹配。
- `URL` 在名称存在于 sources 但 URL 不匹配时显示 `invalid`（红色）。

#### `rec add`

向推荐源添加技能。若条目已存在，新技能将被追加（自动去重）。

```bash
xskill rec add [-n <name>] [-u <url>] -s <skills>
```

选项：
- `-n, --name` — 源名称（若未提供 `--url`，则必须存在于 sources 中）
- `-u, --url` — 源地址（当 name 存在于 sources 中且 url 匹配时，仅保存 name）
- `-s, --skills` — 逗号分隔的技能名称列表（必填）

参数组合逻辑：
- **仅 `-n` 和 `-s`**：验证 `-n` 存在于 sources 中，保存 name + skills
- **`-n`、`-u` 和 `-s`**：
  - 若 `-n` 存在于 sources 中且 url 与 `-u` 匹配：仅保存 name + skills（无需 url）
  - 若 `-n` 存在于 sources 中但 url 与 `-u` 不匹配：报错
  - 若 `-n` 不存在于 sources 中：使用 url + skills 保存（name 为 url 值）
- **仅 `-u` 和 `-s`**：使用 url + skills 保存

追加行为：若条目 "antfu" 已有技能 `vue`，执行 `rec add -n antfu -s react,angular` 后结果为 `vue,react,angular`。

#### `rec remove`

移除推荐源或特定技能：

```bash
xskill rec remove [-n <name>] [-u <url>] [-s <skills>]
```

选项：
- `-n, --name` — 源名称（用于标识条目，或与 `-u`/`-s` 配合使用）
- `-u, --url` — 源地址（当同时指定 `-n` 和 `-u` 时，优先以 `-u` 为准）
- `-s, --skills` — 逗号分隔的技能名称列表（移除特定技能而非整个条目）

优先级逻辑：
- 同时指定 `-n` 和 `-u`：优先按 `-u` 查找，若未找到则回退到 `-n`
- 仅指定 `-n`：删除对应名称的整条数据
- 指定 `-n` 和 `-s`：删除该名称下对应的技能
- 指定 `-u` 和 `-s`：删除该 URL 下对应的技能

### `cache` — 管理技能缓存

管理本地技能缓存，支持离线查询。

```bash
xskill cache <COMMAND>
```

#### `cache update`

从远程源获取技能列表并保存到缓存：

```bash
xskill cache update [-f <source>]
```

选项：
- `-f, --from <source>` — 仅更新特定源（名称或 URL）

每个源的输出：`<源名称>: <数量> skills`。汇总：`Cache updated: N sources, M skills total`。

#### `cache clear`

清除缓存的技能数据：

```bash
xskill cache clear [-f <source>]
```

选项：
- `-f, --from <source>` — 仅清除特定源（名称或 URL）

### `config` — 管理配置

查看或修改全局配置文件。

```bash
xskill config [OPTIONS]
```

选项：
- `-i, --init` — 初始化配置文件，生成含默认值的完整配置（默认平台、缓存、注册中心）
- `-e, --edit` — 在 `$EDITOR` 中打开配置（默认 `vi`）
- `-g, --get <key>` — 通过点号路径获取配置值（如 `cache.enabled`）
- `-s, --set <key=value>` — 通过点号路径设置配置值（如 `cache.enabled=true`）

#### 示例

```bash
# 初始化配置文件
xskill config --init

# 在编辑器中打开配置
xskill config --edit

# 读取值
xskill config --get cache.enabled

# 设置值
xskill config --set cache.enabled=true
```

### `new` — 创建技能项目

使用模板创建新的技能项目。

```bash
xskill new --name <name> [--description <desc>] [--template <template>]
```

选项：
- `-n, --name <name>` — 技能名称（必填，用作目录名）
- `-d, --description <desc>` — 技能描述
- `-t, --template <template>` — 模板类型

## 配置

### 配置文件位置

| 路径 | 说明 |
|------|------|
| `~/.xskill/settings.json` | 全局配置（默认） |
| `XSKILL_CONFIG` 环境变量 | 覆盖配置路径（需指向 JSON 文件） |

没有项目级配置，仅使用一个全局配置文件。

### 配置结构

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

### 完整示例

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

### 平台

每个平台条目配置特定 AI 编程工具的技能安装方式。

#### 平台字段

| 字段 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `path` | 是 | — | 工具配置目录（相对路径、绝对路径或 `~/...`） |
| `skills` | 否 | — | 技能子目录名（相对于 `path`），省略则跳过技能安装 |
| `agents` | 否 | — | 代理配置文件名（相对于 `path`），省略则跳过代理安装 |
| `source` | 否 | `"AGENTS.md"` | 固定 `.agents/` 目录下的源文件名 |
| `agents_compat` | 否 | `false` | 是否兼容 `.agents/` 资源（可复用项目级 agents 配置） |

#### 符号链接行为

`agents` 文件通过符号链接指向 `.agents/` 下的源文件：

```
<path>/<agents>  →  .agents/<source>
```

例如，`agents: "AGENTS.md"` 且 `source: "AGENTS.md"`（默认）：
```
.codex/AGENTS.md  →  .agents/AGENTS.md
```

`agents: "AGENTS.md"` 且 `source: "CLAUDE.md"`：
```
.codex/AGENTS.md  →  .agents/CLAUDE.md
```

### 源

源定义技能的获取位置。

| 字段 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `name` | 否 | — | 唯一标识符（字母数字、`-`、`_`）；留空或无效时自动使用 URL 作为名称 |
| `type` | 否 | `"git"` | 源类型：`git` 或 `api` |
| `url` | 是 | — | 仓库 URL（须以 `http://` 或 `https://` 开头） |

### 推荐

推荐技能由 `rec` 命令管理，方便安装。

| 字段 | 必填 | 说明 |
|------|------|------|
| `name` | 是 | 源名称（必须匹配已配置的源） |
| `skills` | 是 | 推荐技能名称数组 |

### 缓存

| 字段 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `cache.enabled` | 否 | `false` | 为 `query` 命令启用本地技能缓存 |
| `cache.ttl` | 否 | `600` | 缓存有效期（秒），默认 600（10 分钟）。同时作用于主缓存（`skills.json`）和 URL 缓存（`source_<md5>.json`） |

启用后，`xskill cache update` 从所有源获取技能元数据并存储在本地。后续 `query` 命令从缓存读取，而非克隆仓库。

### 注册中心

注册中心是一个可选的 JSON API，提供精选的技能索引。启用后，`query` 和 `find` 命令会在查询配置源的同时额外查询注册中心。

| 字段 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `registry.enabled` | 否 | `false` | 是否启用注册中心查询 |
| `registry.url` | 否 | `https://xskill.gcli.cn/skills.json` | 注册中心地址 |

URL 解析规则：
- 裸域名或以 `/` 结尾 → 自动补全 `/skills.json`
- 路径末尾含文件扩展名（如 `.json`）→ 原样使用
- 空值或无效协议 → 回退内置默认地址

去重规则（URL 归一化后比较，本地优先）：
- 注册中心源的 URL 与已配置源的 URL **相同** → 跳过（以本地配置为准）。
- 注册中心源与已配置源**同名但 URL 不同** → 视为两个不同仓库，注册中心条目正常合并。`query` 中注册中心条目的源名称置空（显示为 `-`），`find` 中显示为该源的 URL。
- 无冲突 → 正常显示。
- **Skill 级去重**：仅在 URL 相同时跳过整个源。URL 不同时，即使 skill 名称相同也保留两条。

示例：
```bash
# 启用注册中心
xskill config --set registry.enabled=true

# 使用自定义注册中心地址（裸域名）
xskill config --set registry.url=https://example.com

# 使用自定义注册中心地址（带路径）
xskill config --set registry.url=https://example.com/api/v1/
```

### `--from` 参数解析

`-f` / `--from` 参数按以下顺序解析：

1. **Git URL**：若值以 `http://` 或 `https://` 开头，直接使用
2. **配置名称**：匹配已配置的源名称
3. **GitHub 简写**：若值包含 `/`（如 `ORG/REPO`），展开为 `https://github.com/ORG/REPO.git`
4. **错误**：若以上均不匹配，报告"源未找到"

### `--skill` 参数

`-s` / `--skill` 参数接受：
- 特定技能名称（如 `vue`）— **精确匹配**，不支持模糊或子串匹配
- 通配符 `*` 匹配所有技能

### `--agent` 验证规则

当 `-a` / `--agent` 指定具体平台名称（非 `*`）时，该平台必须存在于配置的 `platforms` 中。否则显示以下错误：

```
Invalid agents: <输入值>                    （黄色）
Valid agents: platform1, platform2, ...     （黑灰色）
```

### URL 归一化

所有 URL 相关操作在比较或缓存前会去除 `.git` 后缀。适用于 `cache update --from`、`query --from`、`find --from` 及 URL 缓存文件名生成（`source_<md5>.json`）。例如，`https://github.com/org/repo.git` 和 `https://github.com/org/repo` 视为同一 URL。

## 安装模型：规范目录 + 软链接

技能文件存放在**规范目录**（`.agents/skills/`），各平台目录通过**相对路径软链接**指向规范目录。

```
.agents/skills/my-skill/          ← 文件实际存放位置（规范目录）
.codebuddy/skills/my-skill/       → symlink → ../../.agents/skills/my-skill/
.gemini/skills/my-skill/          → symlink → ../../.agents/skills/my-skill/
```

### 全局 vs 本地路径

| 模式 | 规范目录 | 平台目录示例 |
|------|---------|-------------|
| `-g`（全局） | `~/.agents/skills/` | `~/.codebuddy/skills/`、`~/.gemini/skills/` |
| 本地（默认） | `./.agents/skills/` | `./.codebuddy/skills/`、`./.gemini/skills/` |

### 软链接规则

- **相对路径**：使用 `relative(platform_skills_dir, canonical_skill_dir)` 生成相对路径，便于目录移动。
- **幂等**：已存在且指向同一目标 → 跳过。
- **更新**：已存在但指向不同目标 → 删除重建。
- **自动创建父目录**：`mkdir -p` 确保平台 skills 子目录存在。
- **跨平台**：Windows 使用 junction，Unix 使用 symlink。

### 回退机制

```
优先：symlink（默认）
  ↓ 失败
回退：copy（文件复制）
```

symlink 创建失败时，清理目标目录后回退为 `copy_dir_recursive` 文件复制。

### 平台目录不存在时的行为

| 场景 | 平台目录不存在时 |
|------|-----------------|
| `-a <具体平台>` | **主动创建**平台目录并链接 |
| `-a '*'`（所有平台） | **跳过**，不创建目录也不链接 |

## 锁文件

锁文件追踪已安装技能，确保可复现性。

### 位置

| 路径 | 范围 |
|------|------|
| `./.xskill-lock.json` | 项目级 |
| `~/.agents/.xskill-lock.json` | 全局 |

### 格式

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

### 条目字段

| 字段 | 说明 |
|------|------|
| `source` | 配置中的源名称 |
| `source_type` | 源类型（`git`） |
| `source_url` | 完整仓库 URL |
| `skill_path` | 仓库中 `SKILL.md` 的相对路径 |
| `skill_folder_hash` | 技能目录的 Git 树哈希，用于变更检测 |
| `installed_at` | 首次安装的 ISO 8601 时间戳（`YYYY-MM-DDTHH:MM:SS.sssZ`） |
| `updated_at` | 该技能最后更新的 ISO 8601 时间戳（`YYYY-MM-DDTHH:MM:SS.sssZ`） |

### 顶层字段

| 字段 | 说明 |
|------|------|
| `version` | 锁文件格式版本（固定为 `1`） |
| `updated_at` | 锁文件最后修改的 ISO 8601 时间戳（增删改任意 skill 时更新） |

`update` 命令使用锁文件记录重新获取技能，同时保留原始 `installed_at` 时间戳。

`restore` 命令从项目锁文件读取，并写回同级锁文件（默认项目级，`-g` 时全局级），更新 `skill_folder_hash` 和两个 `updated_at` 字段，保留原始 `installed_at`。

## JSON Schema

[`schemas/`](https://github.com/jetsung/xskill/tree/main/schemas) 目录提供两个 JSON Schema，同时托管在 `xskill.gcli.cn`。

### `xskill.schema.json` — 工具配置

用于 `~/.xskill/settings.json`，定义完整配置结构。

**顶层字段：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `$schema` | `string` | 否 | JSON Schema URL，用于编辑器校验。`config --init` 自动生成 |
| `platforms` | `object<string, Platform>` | 否 | 平台配置，以平台标识符为键（如 `"claude"`、`"codex"`） |
| `sources` | `Source[]` | 否 | 技能源仓库 |
| `recommended` | `RecommendedSource[]` | 否 | 按源分组的推荐技能集 |
| `cache` | `CacheConfig` | 否 | 缓存配置 |
| `registry` | `RegistryConfig` | 否 | 注册中心配置 |

**Platform**（`platforms.*`）：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `path` | `string` | 是 | — | 工具配置目录（相对路径、绝对路径或 `~/...`）。最少 1 字符 |
| `skills` | `string` | 否 | `""` | skills 子目录名（相对于 `path`）。为空则跳过技能安装 |
| `agents` | `string` | 否 | `""` | agents 配置文件名（相对于 `path`）。为空则跳过 agents 安装 |
| `source` | `string` | 否 | `"AGENTS.md"` | 固定 `.agents/` 目录下的源文件名。`<path>/<agents>` 符号链接至 `.agents/<source>` |
| `agents_compat` | `boolean` | 否 | `false` | 是否兼容 `.agents/` 资源（可复用项目级 agents 配置） |

**Source**（`sources[]`）：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `name` | `string` | 否 | `""` | 唯一标识符。正则：`^[a-zA-Z0-9_-]+$`。留空或无效时自动使用 `url` 作为名称 |
| `type` | `string` | 否 | `"git"` | 源类型。枚举：`"git"`、`"api"` |
| `url` | `string` | 是 | — | 源仓库 URL。须以 `http://` 或 `https://` 开头 |

**RecommendedSource**（`recommended[]`）：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `name` | `string` | 否 | `""` | 源名称，引用 `sources` 中的条目，或自定义标签 |
| `url` | `string` | 否 | `""` | 直接源 URL（当 `name` 在 sources 中未找到时作为回退） |
| `skills` | `string[]` | 是 | — | 推荐技能名称列表。至少 1 项 |

**CacheConfig**（`cache`）：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `enabled` | `boolean` | 否 | `false` | 启用技能列表缓存。启用后 `query` 和 `find` 从本地缓存读取 |
| `ttl` | `integer` | 否 | `600` | 缓存有效期（秒），默认 10 分钟。同时作用于主缓存（`skills.json`）和 URL 缓存（`source_<md5>.json`）。最小值：0 |

**RegistryConfig**（`registry`）：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `enabled` | `boolean` | 否 | `false` | 启用注册中心查询。启用后 `query` 和 `find` 会额外查询注册中心 |
| `url` | `string` | 否 | `"https://xskill.gcli.cn/skills.json"` | 注册中心地址。支持裸域名、目录路径或完整文件路径 |

### `registry.schema.json` — 注册中心索引

用于注册中心 API 响应（`skills.json`），定义技能索引数据结构。

**顶层字段：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `updated_at` | `string` | 是 | ISO 8601 最后更新时间戳（如 `2026-07-17T12:00:00.000Z`） |
| `sources` | `SourceEntry[]` | 是 | 按源仓库分组的技能 |

**SourceEntry**（`sources[]`）：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `source` | `string` | 是 | 源名称（如 `org/repo`） |
| `url` | `string` | 是 | 源仓库 URL |
| `skills` | `SkillEntry[]` | 是 | 该源下可用的技能 |

**SkillEntry**（`sources[].skills[]`）：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `name` | `string` | 是 | — | 技能名称 |
| `path` | `string` | 是 | — | `SKILL.md` 相对于仓库根目录的路径 |
| `description` | `string` | 否 | `""` | 技能描述 |
| `version` | `string` | 否 | `""` | 技能版本 |

### 编辑器集成

`xskill config --init` 会自动在 `settings.json` 中添加 `$schema` 字段：

```json
{
  "$schema": "https://xskill.gcli.cn/xskill.schema.json",
  ...
}
```

大多数 JSON 编辑器（VSCode、Neovim + jsonls 等）会自动从该 URL 加载 schema，提供校验和自动补全。

## 开发

### 构建

```bash
cargo build
```

### 测试

```bash
cargo test
```

### 项目结构

```
xskill/
├── Cargo.toml
├── README.md
├── schemas/                # JSON Schema 定义
│   ├── xskill.schema.json    # settings.json schema
│   └── registry.schema.json  # 注册中心索引 schema
├── docs/                   # 文档源（mdbook 输入）
│   ├── SPEC.md             # 需求规范
├── book/                   # mdbook 输出（生成）
│   ├── en/
│   └── zh/
├── crates/
│   └── generate-book/      # mdbook 内容生成器
├── src/
│   ├── main.rs             # CLI 入口（clap derive）
│   ├── config.rs           # 配置处理
│   ├── git.rs              # Git 操作（克隆、稀疏检出）
│   ├── lock.rs             # 锁文件管理
│   ├── skill_meta.rs       # SKILL.md frontmatter 解析
│   ├── cache.rs            # 缓存数据结构
│   ├── utils.rs            # 工具函数
│   └── commands/
│       ├── add.rs          # 安装技能
│       ├── remove.rs       # 移除技能
│       ├── update.rs       # 从锁文件更新
│       ├── restore.rs      # 从锁文件恢复
│       ├── list.rs         # 列出已安装技能
│       ├── find.rs         # 交互式 TUI 技能查找器
│       ├── query.rs        # 查询远程/缓存技能
│       ├── sources.rs      # 管理源（CRUD）
│       ├── platforms.rs    # 列出平台
│       ├── rec.rs          # 管理推荐技能（list/add/remove）
│       ├── cache.rs        # 缓存管理
│       ├── config.rs       # 配置管理
│       └── new.rs          # 创建技能项目
└── openspec/               # 变更追踪
```

## 许可证

[Apache License 2.0](https://github.com/jetsung/xskill/blob/main/LICENSE)
