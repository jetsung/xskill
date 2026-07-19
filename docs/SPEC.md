# XSkill 规范文档

## 平台支持

* **支持平台**：Windows、macOS、Linux。
* **路径处理**：使用 `dirs` crate 获取平台相关的路径（如 home 目录），确保跨平台兼容。
* **配置文件**：使用 JSON 格式，路径为 `~/.xskill/settings.json`（全局唯一）。

---

## 输出规范

* **语言**：所有回显信息（包括帮助文档、错误信息、状态输出等）统一使用英文。
* **平台排序**：所有涉及平台列表的输出（`platforms`、`list`、`add`、`remove`、`find` 等），均按平台名称字母顺序（a-zA-Z0-9）排序，不受 `settings.json` 中的配置顺序影响。
* **列表输出**：部分子命令（如 `sources`、`platforms`）使用列对齐的表格形式，首行为表头（大写标签），数据行按列对齐。空值使用 `" - "` 替代。条件字段（如 VERSION）仅在至少一条数据包含该值时才显示该列。`list` 子命令显示标题（"Project Skills" 或 "Global Skills"），后接空行，然后以单行列对齐形式展示（skill 名称褐色、路径黑灰色、Agents 标签黑灰色、平台名默认白色），按路径排序，路径以 `~/` 开头。使用 `-a` 过滤时，未链接的 skill 显示 "not symlinked"（褐色）。`query` 子命令使用垂直键值对形式展示，标签使用 cyan bold，`Name` 值使用黄色显示。
  ```
  NAME   TYPE URL
  antfu  git  https://github.com/antfu/skills
  ```
  ```
  Name: vue
  Description: Vue.js skills
  Version: 1.0.0
  ```
* **配置文件注释**：配置文件中的注释可以使用中文（面向用户）。

---

## 时间戳规范

* **格式**：所有时间戳统一使用 ISO 8601 格式，精确到毫秒，以 `Z` 结尾：`YYYY-MM-DDTHH:MM:SS.sssZ`。
* **示例**：`2026-07-17T14:15:59.535Z`
* **生成方式**：`chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()`
* **适用范围**：锁文件（`installed_at`、条目 `updated_at`、顶层 `updated_at`）、缓存文件（`updated_at`）及所有涉及时间记录的字段。
* **禁止**：不得使用 `to_rfc3339()`（会产生 `+00:00` 后缀和纳秒精度，与规范不一致）。

---

## 配置

### 配置文件路径

| 路径 | 说明 |
|------|------|
| `~/.xskill/settings.json` | 全局配置（默认） |
| `XSKILL_CONFIG` 环境变量 | 若设置则指向替代配置文件（JSON 格式） |

无项目级配置，仅全局唯一配置文件。

### Platform 字段说明

| 字段 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `path` | 是 | — | 工具配置目录（相对路径、绝对路径或 `~/...`） |
| `skills` | 否 | — | skills 子目录名（相对于 path），为空则不安装 |
| `agents` | 否 | — | agents 配置文件名（相对于 path），为空则不安装 |
| `source` | 否 | `"AGENTS.md"` | 源文件名（固定 `.agents/` 目录下），`<path>/<agents>` 符号链接至 `.agents/<source>` |
| `agents_compat` | 否 | `false` | 是否兼容 `.agents/` 资源（可复用项目级 agents 配置） |

### Source 字段说明

| 字段 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `name` | 否 | — | 源名称（仅允许 `[a-zA-Z0-9_-]`）。留空或无效时自动使用 `url` 作为名称 |
| `type` | 否 | `"git"` | 源类型，可选 `git` 或 `api` |
| `url` | 是 | — | 源地址，必须以 `http://` 或 `https://` 开头 |

### 全局字段

| 字段 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `$schema` | 否 | `https://xskill.gcli.cn/xskill.schema.json` | JSON Schema URL，用于编辑器校验和自动补全。`config --init` 自动生成 |

### Cache 配置说明

| 字段 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `cache.enabled` | 否 | `false` | 是否启用 skills 列表缓存。启用后 `query` / `find` 优先读取本地缓存 |
| `cache.ttl` | 否 | `600` | 缓存有效期（秒），默认 600（10 分钟）。同时作用于主缓存（`skills.json`）和 URL 缓存（`source_<md5>.json`） |

### 注册中心配置说明

| 字段 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `registry.enabled` | 否 | `false` | 是否启用注册中心。启用后 `query` / `find` 会额外查询注册中心 |
| `registry.url` | 否 | `https://xskill.gcli.cn/skills.json` | 注册中心地址。支持裸域名（如 `https://xskill.gcli.cn`）、目录路径（如 `https://xskill.gcli.cn/`）或完整文件路径 |

URL 解析规则：
- 裸域名或以 `/` 结尾 → 自动补全 `/skills.json`
- 路径末尾含文件扩展名（如 `.json`）→ 原样使用
- 空值或无效协议 → 回退内置默认地址

去重规则（URL 归一化后比较，本地优先）：
- 注册中心源的 URL 与已配置源的 URL **相同** → 跳过该注册中心条目（以本地配置为准）。
- 注册中心源与已配置源**同名但 URL 不同** → 视为两个不同仓库，注册中心条目正常合并（包括与本地同名的 skill 也保留），以本地同名源为准。
  - `query` 子命令：注册中心条目的源名称置空（显示为 `-`）。
  - `find` 子命令：注册中心条目的源名称显示为该源的 URL（非注册中心 URL）。
- 无冲突 → 正常显示。
- **skill 级去重**：仅在 URL 相同时跳过整个源（见第一条）。URL 不同时，即使 skill 名称相同也保留两条（分别来自本地和注册中心）。
- 此规则同时适用于 `query` 和 `find` 子命令。

### 注册中心索引页面

注册中心提供一个单页面索引（`index.html`），用于浏览和搜索所有已收录的 skills。该页面部署在 GitHub Pages，与 `skills.json` 同域。

* **数据来源**：从同源 `skills.json`（[Registry Schema](../schemas/registry.schema.json)）异步加载。
* **页面标题**：`XSkill Registry`，附带指向项目仓库（`https://github.com/jetsung/xskill`）的 GitHub 图标链接。
* **统计栏**：显示 skills 总数、sources 总数、最后更新时间（使用浏览器本地时区和语言格式化）。

#### 搜索

* **输入框**：固定在页面顶部（sticky），支持实时搜索（150ms 防抖）。
* **匹配范围**：skill 名称、描述、source 名称/URL。
* **高亮**：匹配文字使用背景色高亮标记。
* **结果计数**：搜索时显示匹配数量。
* **键盘快捷键**：按 `/` 聚焦搜索框。

#### 数据展示

* **分组**：skills 按 source 分组展示，每组显示 source 名称（可点击跳转仓库）和 skill 数量标签。
* **source 名称回退**：`source` 为空时显示 `url`。
* **默认折叠**：所有 source 组默认折叠，点击 header 展开/收起。
* **搜索时展开**：搜索时匹配到的 source 组自动展开。
* **skill 条目**：显示名称、描述（溢出省略，悬停 tooltip 显示完整内容）、版本标签。
* **安装命令复制**：每个 skill 条目提供复制按钮，点击复制 `xskill add --from <URL> --skill <SKILL>`，屏幕中央显示已复制命令的提示浮层（2 秒后消失）。

#### 样式

* **响应式**：适配桌面和移动端。
* **主题**：自动跟随系统深色/浅色模式（`prefers-color-scheme`）。
* **零依赖**：所有 CSS/JS 内联，无外部 CDN 依赖。

### 配置文件示例

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
      "name": "antfu2",
      "type": "git",
      "url": "https://gitcode.com/gh_mirrors/skills11/skills.git"
    }
  ],
  "recommended": [
    {
      "name": "antfu2",
      "skills": ["antfu-design"]
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

---

## 安装模型：规范目录 + 软链接

skill 文件实际存放在**规范目录**（canonical directory），各平台目录通过**相对路径软链接**指向规范目录。

```
.agents/skills/my-skill/          ← 文件实际存放位置（规范目录）
.codebuddy/skills/my-skill/       → symlink → ../../.agents/skills/my-skill/
.gemini/skills/my-skill/          → symlink → ../../.agents/skills/my-skill/
```

### 全局 vs 本地路径映射

| 模式 | 规范目录 | 平台目录示例 |
|------|---------|-------------|
| `-g`（全局） | `~/.agents/skills/` | `~/.codebuddy/skills/`、`~/.gemini/skills/` |
| 本地（默认） | `./.agents/skills/` | `./.codebuddy/skills/`、`./.gemini/skills/` |

### 软链接创建规则

* **相对路径**：使用 `relative(platform_skills_dir, canonical_skill_dir)` 生成相对路径，便于目录移动。
* **幂等**：已存在且指向同一目标 → 跳过。
* **更新**：已存在但指向不同目标 → 删除重建。
* **自动创建父目录**：`mkdir -p` 确保平台 skills 子目录存在。
* **跨平台**：Windows 使用 junction，Unix 使用 symlink。

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
| `-a <具体平台>`（如 `-a codebuddy`） | **主动创建**平台目录并链接 |
| `-a '*'`（所有平台） | **跳过**，不创建目录也不链接 |

前置条件：`.agents/`、`.agents/skills/` 等父目录不存在时，`mkdir -p` 会自动递归创建。

### 规范目录清理

安装单个 skill 时，仅清理**该 skill 子目录**（如 `.agents/skills/my-skill/`），**不删除**整个 `.agents/skills/` 目录，已存在的其他 skill 不受影响。

---

## 锁文件

- **项目级路径**：`./.xskill-lock.json`
- **全局级路径**：`~/.agents/.xskill-lock.json`
- **职责**：记录已安装 skill 的安装来源、路径、哈希、时间戳。仅记录规范目录中的安装信息，平台目录的软链接不记录在锁文件中（可从规范目录反向推导）。

### 锁文件格式

```json
{
  "version": 1,
  "skills": {
    "skill-name": {
      "source": "antfu2",
      "source_type": "git",
      "source_url": "https://gitcode.com/gh_mirrors/skills11/skills.git",
      "skill_path": "skills/vue/SKILL.md",
      "skill_folder_hash": "5b3b9e205ee6d4e1256a18d8d61c6de8a75f9ed6",
      "installed_at": "2026-07-15T18:16:42.852Z",
      "updated_at": "2026-07-15T18:16:59.530Z"
    }
  },
  "updated_at": "2026-07-15T18:16:59.530Z"
}
```

---

## 通用参数解析规则

### 1. `--from, -f` 参数解析逻辑

当子命令使用 `-f` / `--from` 指定源仓库时，按以下优先级解析输入值：

1. **Git URL 判定**：若输入值以 `http://` 或 `https://` 开头，直接作为目标 Git 仓库地址使用。
2. **配置文件匹配**：若非 URL 格式，首先尝试将其作为"源名称"去配置文件中检索。若匹配成功，则使用其对应的配置 URL。
3. **GitHub 简写补全**：若未匹配到源名称，且值中包含斜杠 `/`（例如 `ORG/REPO`），则自动补全为 GitHub 仓库地址：`https://github.com/ORG/REPO.git`。
4. **异常处理**：若以上均不满足，则抛出"未找到有效源"的错误。

### 2. URL 归一化规则

所有涉及 URL 作为缓存键或源标识的场景，必须遵循以下归一化规则：

* **去除 `.git` 后缀**：`https://github.com/org/repo.git` 与 `https://github.com/org/repo` 视为同一 URL。
* **适用范围**：
  * `cache update --from <url>`：存储缓存时归一化 source 名称。
  * `query --from <url>`：查询缓存时归一化匹配。
  * `find --from <url>`：查询缓存时归一化匹配。
  * URL 缓存文件名生成（`source_<md5>.json`）：MD5 计算前归一化 URL。
* **目的**：确保同一仓库无论是否带 `.git` 后缀，都使用同一缓存文件，避免数据重复。

### 3. `--skill, -s` 参数解析逻辑

当子命令使用 `-s` / `--skill` 指定具体技能时：

* 支持传入具体名称（如 `deploy`）定位单个 skill。**精确匹配** skill 名称，不支持模糊或子串匹配。
* 支持传入通配符 `*`（如 `-s '*'`）代表**所有 skills**。
* **约束**：当 `-s '*'` 与 `-a '*'` 同时使用时（包括 `-A` 快捷键），**必须**通过 `-f` 指定源。因为所有平台的所有 skills 范围过大，需要限定来源以避免误操作。此约束仅适用于 `add` 命令；`remove` 命令不受此限制。

### 4. `--agent, -a` 参数验证规则

当子命令使用 `-a` / `--agent` 指定目标平台时：

* 支持传入具体平台名称（如 `codebuddy`）定位单个平台。
* 支持传入通配符 `*`（如 `-a '*'`）代表**所有已配置平台**。
* **验证**：当值为具体名称（非 `*`）时，必须存在于配置文件的 `platforms` 中。不存在则报错，格式如下：

  ```
  Invalid agents: <输入值>
  Valid agents: <平台1>, <平台2>, <平台3>, ...
  ```

  * `Invalid agents: <输入值>` 整行使用褐色（yellow）显示。
  * `Valid agents: <平台1>, <平台2>, <平台3>, ...` 整行使用黑灰色（bright black）显示。

* **适用范围**：`add`、`remove`、`list`、`restore` 等所有使用 `-a` 的子命令。

---

## 子命令实现规范

### `sources` — 列出配置源

* **行为**：读取并打印配置中所有的源，以表格形式输出。
* **输出格式**：
  ```
  NAME   TYPE URL
  antfu  git  https://github.com/antfu/skills
  ```
* **边界情况**：
  * sources 为空时输出 "No sources configured"。
  * name 为空时显示 ` - `。

### `sources add` — 添加源

* **行为**：通过 CLI 添加一个新源，写入 `~/.xskill/settings.json`。
* **参数**：
  * `-n, --name`：源名称（可选，仅允许 `[a-zA-Z0-9_-]`）。留空时 `name` 字段为空。
  * `-u, --url`：源地址（必填，合法 URI）。
  * `-t, --type`：源类型，可选 `git` 或 `api`，默认 `git`。
* **边界情况**：
  * URL 已存在时报错（若该源有名称则提示，无名称则仅提示 URL 已存在）。
  * `--name` 非空且已被占用时报错并提示已有源列表。
  * 名称或 URL 格式不合法时报错。

### `sources remove` — 移除源

* **行为**：通过 CLI 移除指定源，更新 `~/.xskill/settings.json`。
* **参数**：
  * `-n, --name`：要移除的源名称（可选）。
  * `-u, --url`：要移除的源地址（可选）。
* **匹配规则**：
  * 仅指定 `--name`：按名称匹配。
  * 仅指定 `--url`：按 URL 匹配。
  * 同时指定：名称和 URL 都匹配才删除。
  * 均未指定时报错。
* **边界情况**：
  * 无匹配源时提示 "No matching source found."。

### `sources edit` — 编辑源

* **行为**：通过 CLI 修改已有源的名称，更新 `~/.xskill/settings.json`。`url` 和 `type` 不可变更，仅允许通过增删条目管理。
* **参数**：
  * `-n, --name`：要编辑的源名称（可选）。
  * `-u, --url`：要编辑的源地址（可选）。
  * `-N, --new-name`：新名称（必填，传空字符串清空名称）。
* **匹配规则**：
  * 仅指定 `--name`：按名称匹配。
  * 仅指定 `--url`：按 URL 匹配。
  * 同时指定：名称和 URL 都匹配才更新。
  * 均未指定时报错。
* **边界情况**：
  * 无匹配源时提示 "No matching source found."。
  * `--new-name` 非空且已被其他源占用时报错。

### `cache` — 管理缓存

* **行为**：管理 skills 列表缓存，使用子命令方式调用。
* **子命令**：

#### `cache update` — 更新缓存

* **行为**：从远程源获取 skills 列表，解析 SKILL.md frontmatter，保存至 `~/.xskill/cache/skills.json`。
* **参数**：
  * `-f, --from <source>`：指定源（名称或 URL），仅更新该源的数据。不指定则更新所有源。URL 作为源时遵循 [URL 归一化规则]。
* **URL 缓存**：
  * 当 `--from` 为 URL 且不在 `sources` 配置中时，克隆仓库并将结果保存至 `~/.xskill/cache/source_<md5(url)>.json`。
  * URL 在计算 MD5 前会去除 `.git` 后缀，确保 `https://example.com/repo.git` 和 `https://example.com/repo` 使用同一缓存文件。
* **边界情况**：
  * `--from` 指定的源名称不在 `sources` 配置中时报错，格式为：
    ```
    Source '<name>' not found.
    Available: <source1>, <source2>, ...
    ```
  * 无配置源时输出 "No sources configured"。
  * 某个源获取失败时输出错误信息，继续处理其他源。

#### `cache clear` — 清空缓存

* **行为**：清空 `skills.json` 中的 `sources` 数组（保留文件，更新 `updated_at` 时间戳），同时删除 `~/.xskill/cache/` 目录下所有 `source_<md5>.json` URL 缓存文件。
* **参数**：
  * `-f, --from <source>`：指定源（名称或 URL），仅移除该源条目。不指定则移除所有源。
* **边界情况**：
  * `--from` 指定的源不存在时提示 "Source '<name>' not found in cache"。
  * 缓存文件不存在时输出 "No cache to clear"。
  * 清空后输出已移除的源数量和 URL 缓存文件数量。

* **缓存结构**：
  ```json
  {
    "updated_at": "2026-07-17T12:00:00.000Z",
    "sources": [
      {
        "source": "antfu",
        "url": "https://github.com/antfu/skills",
        "skills": [
          {"name": "vue", "path": "skills/vue", "description": "...", "version": "1.0.0"}
        ]
      }
    ]
  }
  ```

### `platforms` — 列出配置平台

* **行为**：以表格形式打印配置中的平台。
* **参数**：
  * `-a, --all`：显示各平台的详细信息（路径、关联的 agent 指导文件等）。
* **输出格式（简单）**：
  ```
  NAME    PATH
  claude  .claude
  ```
* **输出格式（详细）**：
  ```
  NAME    PATH    SKILLS  AGENTS      SOURCE      COMPAT
  claude  .claude skills  CLAUDE.md   AGENTS.md
  codex   .codex  skills  AGENTS.md   AGENTS.md   ✓
  ```
* **边界情况**：platforms 为空时输出 "No platforms configured"。空字段显示 ` - `。

### `add` — 安装技能

* **行为**：将 skill 安装到规范目录，并按需创建到各平台目录的软链接。安装后写入锁文件。
* **参数**：
  * `-f, --from`：指定源（遵循 [通用参数 -from] 规则）。
  * `-s, --skill`：指定 skill（遵循 [通用参数 -skill] 规则）。支持 `*` 安装所有 skills。使用 `-A` 时可省略（自动设为 `*`）。
  * `-g, --global`：全局安装。
  * `-a, --agent`：指定目标平台（支持 `'*'` 代表所有平台）。当值为具体名称时，必须存在于配置文件的 `platforms` 中（遵循 [通用参数 -a 验证规则]）。
  * `-A, --all`：一键全装快捷键，等同于 `--skill '*' --agent '*'`。
* **安装流程**（每个 skill × agent 组合独立处理）：

  | 命令 | 安装到规范目录 | 创建 symlink |
  |------|--------------|-------------|
  | `add -s s1` | `.agents/skills/s1` | 无 |
  | `add -s s1 -g` | `~/.agents/skills/s1` | 无 |
  | `add -s s1 -a codebuddy` | `.agents/skills/s1` | `.codebuddy/skills/s1` → 规范目录（平台目录不存在时自动创建） |
  | `add -s s1 -a codebuddy -g` | `~/.agents/skills/s1` | `~/.codebuddy/skills/s1` → 规范目录（同上） |
  | `add -s s1 -a '*'` | `.agents/skills/s1` | 各已存在平台目录（跳过不存在的） |
  | `add -s '*' -a '*' -f source1` | 所有 skills（来自 source1） | 各已存在平台目录 |
  | `add -s '*' -a '*'`（无 `-f`） | 报错：必须指定 `-f` | — |
  | `add -A`（等同 `-s '*' -a '*'`） | 报错：必须指定 `-f` | — |

* **单个安装流程**：
  1. 清理规范目录中的 skill 子目录（`rm -rf` + `mkdir -p`）。
  2. 写入 skill 文件到规范目录。
  3. 若指定 `-a`：创建软链接（相对路径）从平台 skills 目录指向规范目录。
  4. 若软链接失败 → 回退为文件复制。
* **源信息显示**：安装前输出源信息，格式为 `Source: <source-name> (<source-url>)`（标签 cyan bold）。
* **多源同名技能选择**：当未指定 `-f` 且多个源（含注册中心）包含同名技能时：
  * **交互终端**：弹出 skim 单选 TUI，三列对齐显示：`[registry]`（第一列，仅注册中心条目显示）、`source_name`（第二列）、`url`（第三列）。注册中心 source_name 为空或与本地源冲突时显示 `-`。TUI 配置与 `find` 子命令一致（`bg:236` 高亮、`exact` 模式）。
  * **非交互终端**：报错并列出所有匹配源（含 URL），提示使用 `-f <source>` 指定。
  * **搜索范围**：本地配置源（`sources`）→ 缓存 → 注册中心（`registry`），按 URL 归一化去重。
* **前置条件**：
  * `-a <name>` 时，该平台必须存在于配置文件的 `platforms` 中（遵循 [通用参数 -a 验证规则]）。
  * `-a <name>` 时，平台目录不要求预先存在，不存在则自动创建。
  * `-a '*'` 时，仅链接平台根目录已存在的平台。
* **输出样式**：
  * 标签（`Name`、`Description`、`Version`）使用 cyan bold 显示。
  * `Name` 值使用黄色（褐色）显示。
  * `Description` 为空时不显示该行。
  * `Version` 为空时不显示该行。
  * `Installed`（green）后接规范目录路径。`Symlinked`（green）后接平台目录路径（不显示箭头和目标，因 `Installed` 行已展示规范目录）。`Source`（cyan bold）后接源名和 URL，格式为 `Source: <name> (<url>)`。

### `remove` — 移除技能

* **行为**：移除已安装的 skill（包括规范目录和平台软链接）并同步更新锁文件。
* **参数**：
  * `-s, --skill`：指定要移除的 skill（遵循 [通用参数 -skill] 规则）。使用 `-A` 时可省略（自动设为 `*`）。
  * `-g, --global`：全局移除。
  * `-a, --agent`：指定目标平台（支持 `'*'` 代表所有平台）。当值为具体名称时，必须存在于配置文件的 `platforms` 中（遵循 [通用参数 -a 验证规则]）。
  * `-A, --all`：等同于 `--skill '*' --agent '*'`。
* **移除流程**：

  | 命令 | 删除规范目录 | 删除 symlink |
  |------|------------|-------------|
  | `remove -s s1` | `.agents/skills/s1` | 各平台 symlink（悬空文件夹清理） |
  | `remove -s s1 -g` | `~/.agents/skills/s1` | 各平台 symlink（悬空文件夹清理） |
  | `remove -s s1 -a codebuddy` | — | 仅删除 `.codebuddy/skills/s1` symlink |
  | `remove -s s1 -a '*'` | — | 删除各平台 symlink（保留规范目录） |
  | `remove -s '*'` | `.agents/skills/` 下所有 skill | 各平台 symlink（悬空文件夹清理） |
  | `remove -s '*' -a '*'` | 删除规范目录 | 删除各平台 symlink |

* **关键规则**：
  * `remove` **不需要** `-f` 参数（与 add 不同）。
  * 不指定 `-a` 时：移除规范目录 + 清理各平台中指向该 skill 的 symlink（悬空文件夹），避免规范目录删除后留下断裂链接。
  * `-a '*'` 时：仅移除各平台 symlink，**保留规范目录**（skill 文件仍可用于其他平台）。
  * `-a <具体平台>` 时：仅移除该平台的 symlink，保留规范目录。
  * 删除 symlink 时使用 `fs::remove_file`（非 `remove_dir_all`），避免误删规范目录内容。
  * 删除规范目录时仅 `rm -rf` 单个 skill 子目录。
  * 更新锁文件：移除对应条目。
  * `-a <name>` 时，该平台必须存在于配置文件的 `platforms` 中（遵循 [通用参数 -a 验证规则]）。
* **边界情况**：
  * 指定的 skill 未安装（规范目录和各平台均不存在）时，输出 `Nothing to remove: skill '<name>' not installed`（黄色显示）。
  * `-s '*'` 批量移除但无任何已安装 skill 时，输出 `Nothing to remove: no skills installed`（黄色显示）。

### `update` — 更新已安装技能

* **行为**：基于锁文件记录的源信息重新拉取并安装，保留原始 `installed_at` 时间戳。更新时按 `source_url` 分组，同一仓库仅克隆一次（`clone_for_listing`），从共享克隆中复制文件到目标目录，避免重复 git clone。
* **参数**：
  * `-s, --skill`：指定更新单个或所有 (`'*'`) 技能（遵循 [通用参数 -skill] 规则）。
  * `-g, --global`：更新全局定义的 skills。
* **输出格式**：
  ```
  Updating skills...

  Source: https://github.com/antfu/skills.git
    Updating: vue
      Name: Vue.js Skills
      Description: Vue.js skills collection
      Version: 1.0.0
      Updated: .agents/skills/vue

    Updating: react
      Name: React Skills
      Description: React skills collection
      Updated: .agents/skills/react

  Update complete: 2 succeeded, 0 failed
  ```
* **输出样式**：
  * 标签（`Source`、`Updating`、`Name`、`Description`、`Version`、`Updated`）使用 cyan 显示。
  * `Name` 值使用黄色（褐色）显示。
* **锁文件更新**：更新成功后更新锁文件：
  * `skill_folder_hash`：从远程仓库获取最新 git tree hash。
  * `installed_at`：保留原始安装时间戳。
  * `updated_at`（条目级）：更新为当前时间戳。
  * `updated_at`（顶层）：更新为当前时间戳。
* **边界情况**：
  * 锁文件不存在或无 skill 条目时输出 "No skills to install"（全局）或 "No skills to install"（项目级）。
  * 单个 skill 更新失败时记录错误，继续处理其余 skill，最终汇总报告。
  * 仓库克隆失败时，该仓库下所有 skill 计入失败数。

### `restore` — 从锁文件恢复技能

* **行为**：读取当前目录下的 `.xskill-lock.json`，按照锁文件中记录的源信息安装所有 skill 到指定目标目录。适用于新环境搭建或项目克隆后快速恢复已安装的 skills。恢复时按 `source_url` 分组，同一仓库仅克隆一次（`clone_for_listing`），从共享克隆中文件复制到目标目录，避免重复 git clone。
* **参数**：
  * `-g, --global`：安装到全局目录 `~/.agents/skills/`（默认安装到项目级 `.agents/skills/`）。
  * `-a, --agent`：指定目标平台（支持 `'*'` 代表所有平台）。当值为具体名称时，必须存在于配置文件的 `platforms` 中（遵循 [通用参数 -a 验证规则]）。
  * `-D, --dry-run`：预览模式，仅列出将要恢复的 skill 信息（名称、来源 URL、目标路径），不执行实际安装。
* **输出格式**：按 `source_url` 分组输出，每组先显示源地址，再逐个显示 skill 信息。
  ```
  Source: <source-url>
    Name: <display-name>
    Description: <description>
    Version: <version>
    Installed: <target-path>

  Restore complete: <N> succeeded, <M> failed
  ```
* **输出样式**：
  * 标签（`Source`、`Name`、`Description`、`Version`）使用 cyan 显示，`Name` 标签额外加粗。
  * `Name` 值使用黄色（褐色）显示。
  * `Restore complete` 使用绿色，失败数使用红色。
* **dry-run 输出格式**：按 skill 名称分组，同一 skill 的多个目标缩进显示。
  ```
  Skills to restore:

  NAME              SOURCE                                      TARGET
  git-commit        https://github.com/antfu/skills.git         .agents/skills/git-commit
                                                                .claude/skills/git-commit
  react             https://github.com/antfu/skills.git         .agents/skills/react
  ```
  * `Skills to restore:` 使用默认色。
  * 多目标时（`-a '*'` 或默认多平台）：表头行蓝色，每组首行默认色，续行 TARGET 使用黑灰色。
  * 单目标时（`-a <name>` 指定具体平台）：不使用颜色。
* **锁文件更新**：安装成功后更新目标锁文件（默认项目级 `.xskill-lock.json`，`-g` 时全局级 `~/.agents/.xskill-lock.json`，不存在则创建）：
  * `skill_folder_hash`：从远程仓库获取最新 git tree hash。
  * `installed_at`：若目标锁文件中已存在同名 skill，保留其 `installed_at`；否则使用源锁文件中的值。
  * `updated_at`（条目级）：更新为当前时间戳。
  * `updated_at`（顶层）：更新为当前时间戳。
  * 项目级与全局级锁文件遵循相同的更新逻辑。
* **边界情况**：
  * 锁文件不存在或无 skill 条目时输出 "No skills to restore"。
  * 单个 skill 安装失败时记录错误，继续处理其余 skill，最终汇总报告。
  * `--global` 和 `--agent` 均未指定时默认安装到项目级 `.agents/skills/`。

### `list` — 查看已安装列表

* **行为**：扫描规范目录和各平台目录，按 skill 名称去重后以单行列对齐形式展示。列表按路径排序。
* **参数**：
  * `-g, --global`：列出全局安装的 skills。
  * `-a, --agent`：按平台名称进行过滤筛选。当值为具体名称时，该平台必须存在于配置文件的 `platforms` 中（遵循 [通用参数 -a 验证规则]）。
* **输出格式**：
  ```
  Project Skills

  skill-name    ~/.agents/skills/skill-name    Agents: platform1, platform2
  vue           ~/.agents/skills/vue           Agents: codebuddy, gemini
  react         ~/.agents/skills/react         Agents: codebuddy
  ```
* **标题**：
  * 项目级显示 "Project Skills"，全局级显示 "Global Skills"。
  * 标题后有空行分隔。
* **输出列**：
  * **第 1 列**：skill 名称（左对齐，褐色/yellow 显示）。
  * **第 2 列**：路径（以 `~/` 开头，黑灰色/bright black 显示）。
  * **第 3 列**：`Agents:` 前缀（黑灰色/bright black）+ 逗号分隔的平台名列表（默认白色显示）。
* **排序**：按路径字母顺序排序。
* **扫描逻辑**：
  * 始终扫描规范目录和**所有**平台目录，以收集完整的 agent 列表。
  * 按 skill 名称去重（symlink 指向同一目录，内容相同）。
  * 跳过断裂的 symlink。
  * 路径显示使用 `~/` 前缀代替 home 目录绝对路径。
* **平台过滤逻辑**（`-a <agent>`）：
  * 始终显示 skill 关联的**所有**平台（不仅是过滤的平台）。
  * 若 skill 在规范目录中但未链接到指定平台，显示 "Agents: not symlinked"（`Agents:` 黑灰色，`not symlinked` 褐色）。
  * 若 skill 仅存在于平台目录（非 symlink 到规范目录），显示实际平台路径而非规范目录路径。
* **边界情况**：
  * 无已安装 skill 时输出 "No skills installed"（黑灰色/bright black）。

### `rec` — 管理推荐 skills

* **行为**：管理配置中的推荐 skills 源，支持列表、添加和移除操作。
* **子命令**：

#### `rec list` — 列出推荐源

* **行为**：以表格形式打印配置中所有推荐源，交叉验证 `sources` 配置。
* **输出格式**：
  ```
  SOURCE  NAME   URL                              SKILLS
  true    antfu  https://github.com/antfu/skills  vue, react
  false   foo    invalid                          bar
  ```
* **列顺序**：`SOURCE → NAME → URL → SKILLS`。
* **列说明**：
  * `SOURCE`：验证状态，`true` 表示该推荐源的 name 存在于 `sources` 配置中且 URL 一致，`false` 表示不匹配或 name 不存在于 sources。
  * `URL`：始终显示，按以下规则解析。
* **URL 解析规则**：
  * 若 `name` 存在于 `sources` 且 `url` 为空 → 从 source 获取 URL，`SOURCE` 显示 `true`。
  * 若 `name` 存在于 `sources` 且 `url` 与 source 一致 → 正常显示 URL，`SOURCE` 显示 `true`。
  * 若 `name` 存在于 `sources` 但 `url` 与 source 不一致 → URL 显示 `invalid`（红色），`SOURCE` 显示 `false`。
  * 若 `name` 不存在于 `sources` 且 `url` 有值 → 显示原始 `url`，`SOURCE` 显示 `false`。
  * 若 `name` 不存在于 `sources` 且 `url` 为空 → URL 显示 `invalid`（红色），`SOURCE` 显示 `false`。
* **边界情况**：推荐源为空时输出 "No recommended skills configured"。空字段显示 ` - `。

#### `rec add` — 添加推荐技能

* **行为**：向推荐源添加技能，写入 `~/.xskill/settings.json`。若推荐源已存在则追加技能（去重）。
* **参数**：
  * `-n, --name`：源名称（可选，必须存在于 sources 中）。
  * `-u, --url`：源地址（可选）。
  * `-s, --skills`：逗号分隔的技能名称列表（必填）。
* **参数组合逻辑**：
  * 仅 `-n` 和 `-s`：验证 `-n` 存在于 sources 中，保存 `name` + `skills`。
  * `-n`、`-u` 和 `-s`：
    * 若 `-n` 存在于 sources 中且 `url` 与 `-u` 匹配：仅保存 `name` + `skills`（无需 url）。
    * 若 `-n` 存在于 sources 中但 `url` 与 `-u` 不匹配：报错。
    * 若 `-n` 不存在于 sources 中：使用 `url` + `skills` 保存（name 为 url 值）。
  * 仅 `-u` 和 `-s`：使用 `url` + `skills` 保存。
* **追加逻辑**：若推荐源已存在且已有技能 `a`，本次使用 `-s b,c`，则最终值为 `a,b,c`。
* **边界情况**：
  * 未指定 `-n` 或 `-u` 时报错。
  * `-n` 不存在于 sources 中且未指定 `-u` 时报错。
  * 技能列表为空时报错。
  * 所有技能已存在时输出提示信息。

#### `rec remove` — 移除推荐源或特定技能

* **行为**：移除推荐源或从推荐源中移除特定技能。
* **参数**：
  * `-n, --name`：源名称（可选，用于标识条目，或与 `-u`/`-s` 配合使用）。
  * `-u, --url`：源地址（可选，当同时指定 `-n` 和 `-u` 时，优先以 `-u` 为准）。
  * `-s, --skills`：逗号分隔的技能名称列表（可选，移除特定技能而非整个条目）。
* **优先级逻辑**：
  * 同时指定 `-n` 和 `-u`：优先按 `-u` 查找，若未找到则回退到 `-n`。
  * 仅指定 `-n`：删除对应名称的整条数据。
  * 指定 `-n` 和 `-s`：删除该名称下对应的技能。
  * 指定 `-u` 和 `-s`：删除该 URL 下对应的技能。
* **边界情况**：
  * 未指定 `-n` 或 `-u` 时报错。
  * 指定的源不存在时报错并提示可用推荐源列表。
  * 指定的技能不存在时输出提示信息。

### `query` — 查询/列出技能

* **行为**：查询指定的 skill。若 `registry.enabled` 为 `true` 且未指定 `-f`，还会额外查询注册中心。
* **参数**：
  * `-f, --from`：指定源（遵循 [通用参数 -from] 规则）。URL 作为源时遵循 [URL 归一化规则]。
  * `-s, --skill`：指定 skill（必填，遵循 [通用参数 -skill] 规则）。不支持 `*`，必须为具体 skill 名称。
* **缓存策略**：
  * 若 `cache.enabled` 为 `true`：从 `~/.xskill/cache/skills.json` 读取，根据 `cache.ttl` 检查 `updated_at` 是否过期。
    * 未过期且 sources 非空（或无配置源）：直接使用缓存。
    * 过期、不存在、或 sources 为空但有配置源：重新克隆所有源，并自动回写 `skills.json`。
  * 若 `cache.enabled` 为 `false`（默认）：从远程源获取数据查询。
  * 若 `registry.enabled` 为 `true` 且未指定 `-f`：本地数据按上述缓存策略加载后，与注册中心数据合并（注册中心始终实时拉取）。
  * 注册中心返回 JSON 格式（与缓存结构兼容，每条源含 `source` 和 `url` 字段）。
  * **去重**：遵循 [注册中心去重规则]。
  * 获取失败时输出警告信息，继续处理。
* **输出格式**：
  ```
  Source: antfu
  Name: vue
  Description: Vue.js skills
  Version: 1.0.0
  Path: skills/vue/SKILL.md

  Source: -
  Registry: https://gitcode.com/gh_mirrors/skills11/skills.git
  Name: react
  Description: React skills
  Path: skills/react/SKILL.md
  ```
* **输出样式**：
  * 标签（`Source`、`Registry`、`Name`、`Description`、`Version`、`Path`）使用 cyan bold 显示。
  * `Name` 值使用黄色（褐色）显示。
  * `Source` 为空时显示 `-`。
  * `Description` 为空或 `"无"` 时不显示该行。
  * `Version` 为空时不显示该行。
  * 各 skill 之间以空行分隔。
* **源显示规则**：
  * 若指定 `-f`：优先显示配置中的源名称，否则显示传入的值。
  * 若未指定 `-f`：显示配置中的源名称。
  * 注册中心的技能：显示原始源名称（为空时显示 `-`），并额外显示 `Registry` 行（该 source 条目的 `url`）。
* **边界情况**：
  * `-s` 为空时报错：`Skill name cannot be empty`。
  * `-s` 为 `*` 时报错：`Skill name cannot be '*', use a specific skill name`。
  * 无 skills 且存在已配置源（`sources` 非空或 `registry.enabled` 为 `true`）且 `cache.enabled` 为 `true` 时，提示 `Hint: run 'xskill cache update' to refresh skills cache`（单独一行，cyan 显示）。

### `find` — 交互式查找并安装技能

* **行为**：通过两步 TUI 交互式查找 skill 和目标平台后安装。安装范围通过 `-g` 参数控制。
* **参数**：
  * `-f, --from`：指定源（遵循 [通用参数 -from] 规则），仅显示该源的 skills。支持 URL 方式，若 URL 不在 sources 中则自动拉取并缓存至 `~/.xskill/cache/source_<md5(url)>.json`，缓存有效期由 `cache.ttl` 控制（默认 10 分钟）。URL 作为源时遵循 [URL 归一化规则]。
  * `-s, --skill`：初始过滤查询。
  * `-g, --global`：全局安装（`~/.agents`），不指定则安装到项目级（`.agents`）。
* **TUI 显示规范**：
  * **布局**：搜索框在底部，列表紧贴搜索框向上排列（`Default` 布局）。
  * **搜索框提示**：`Search skills: `。
  * **快捷键提示**：列表下方显示 `up/down navigate | enter select | esc cancel`，上下各空一行。
  * **列表项样式**：使用 `ratatui` 渲染带颜色的 `Line`。非选中行 skill 名称使用默认颜色，选中行 skill 名称使用蓝色（`Blue`）。source 标签 `[source]` 始终使用暗灰色（`DarkGray`）显示。来自注册中心的条目，在 `[source]` 前额外显示 `[registry]` 标签，格式为 `name [registry] [source]`。`[registry]` 标签颜色：非选中行为暗灰色（`DarkGray`），选中行为绿色（`Green`）。
  * **选中行高亮**：当前选中行使用深色背景（`bg:236`）高亮显示（`highlight_line`），匹配文字使用更亮的颜色（`fg:151:bg:236`）。
  * **匹配计数**：显示在列表下方（快捷键提示上方），格式为 `当前/总数`（如 `2/2`）。
  * **已知问题**：skim 库的列表行号从 0 开始（0-based），而非从 1 开始（1-based）。这是 skim 自身行为（skim 5.2.0），非本项目可控。
* **交互流程**：
  1. **选择 Skill**：启动 TUI 子串搜索（exact 模式），显示格式为 `name [source]`（非选中行 name 默认色，选中行 name 蓝色，source 暗灰色），来自注册中心的条目显示为 `name [registry] [source]`（选中时 `[registry]` 绿色）。输入进行子串匹配，选中行有深色背景高亮。
  2. **选择目标平台**：TUI 多选，列表首项为 `Default`（不可选中，因 skim Default 布局倒序显示，视觉上在底部），后续为配置中所有 platforms。选中行文字使用蓝色（`Blue`），深色背景高亮。顶部提示 `TAB: select/deselect`，上下各空一行。TAB 选中平台后按 Enter 确认；未选中任何平台直接按 Enter（光标在 `Default` 处），则不创建任何平台符号链接。
  3. **安装**：skill 始终安装到规范目录（`.agents/skills/<name>` 或 `-g` 时 `~/.agents/skills/<name>`），输出 `Installed: <path>`。然后为第 2 步选中的平台创建相对符号链接，输出 `Symlinked: <platform1>, <platform2>, ...`。符号链接失败时回退为文件复制。安装完成后更新锁文件。失败的平台在最后统一报告（如有）。
  * 任意步骤按 Esc 或 Ctrl-C 取消，输出 "Cancelled."
* **子串匹配**：使用 skim exact 模式，仅匹配包含连续子串的项。输入 `git-commit` 时只显示包含 `git-commit` 子串的项，不会匹配 `gtm-board-and-investor-communication` 等非子串项。
* **依赖**：`skim`（TUI 模糊搜索）、`ratatui`（列表项颜色渲染，`Line`/`Span`/`Color`）、`colored`（安装结果消息颜色）。
* **缓存策略**：
  * 若 `cache.enabled` 为 `true`：从 `~/.xskill/cache/skills.json` 读取，根据 `cache.ttl` 检查 `updated_at` 是否过期。
    * 未过期且 sources 非空（或无配置源）：直接使用缓存。
    * 过期、不存在、或 sources 为空但有配置源：重新克隆所有源，并自动回写 `skills.json`（后续调用直接命中新鲜缓存）。
  * 若 `--from` 为 URL 且不在 sources 中：从 `~/.xskill/cache/source_<md5>.json` 读取（`cache.ttl` 秒内有效），不存在或过期则自动拉取并回写。
  * 若 `registry.enabled` 为 `true`：本地数据按上述缓存策略加载后，与注册中心数据合并（注册中心始终实时拉取）。
* **边界情况**：
  * 缓存为空时输出 "No skills found in cache."。
  * `--from` 指定的源不在缓存中时报错。
  * 非交互终端时报错 "find requires an interactive terminal."

### `config` — 管理配置

* **行为**：管理 `~/.xskill/settings.json` 配置文件。无参数时输出用法提示。
* **参数**：
  * `-i, --init`：初始化配置文件，生成含默认值的完整配置（含默认平台、缓存、注册中心配置）。若配置文件已存在则提示，不覆盖。
  * `-e, --edit`：在编辑器中打开配置文件（使用 `$EDITOR` 环境变量，默认 `vi`）。
  * `-g, --get <key>`：读取单个配置值，使用点号路径（如 `cache.enabled`、`sources`）。
  * `-s, --set <key=value>`：设置单个配置值，使用点号路径（如 `cache.enabled=true`）。
* **边界情况**：
  * 无参数时输出 `Usage: xskill config --init | --edit | --get <key> | --set <key=value>`。
  * `--init` 时若配置文件已存在，输出提示信息并退出。
  * `--get` 路径不存在时报错。
  * `--set` 值类型不匹配时报错（如将字符串赋给布尔字段）。
  * `$EDITOR` 未设置时回退到 `vi`。

### `new` — 创建 Skill 项目

* **行为**：在当前目录下创建标准 skill 项目结构，生成模板文件。
* **参数**：
  * `-n, --name <name>`：skill 名称（必填），用作目录名和 SKILL.md 的 name 字段。
  * `-d, --description <desc>`：skill 描述（可选），写入 SKILL.md 的 description 字段。
  * `-t, --template <template>`：模板类型（可选），预设不同 skill 类型的模板内容。
* **生成文件**：
  * `<name>/SKILL.md`：含 YAML frontmatter（name、description、metadata.version）。
* **边界情况**：
  * 目录已存在时报错并提示。
  * 名称为空时报错。
