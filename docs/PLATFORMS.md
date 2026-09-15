# Platforms

兼容性说明：`zcode`、`atomcode`、`dsh` 完美兼容；`codebuddy`、`claude`、`openclaude` 不完全兼容。

以下列表与 `default_platforms()` 内置默认配置一致（`~/.xskill/settings.json` 初始化时生成）。

| 配置 key | 显示名称 | 路径 | 项目级路径 | Skills 目录 | Agents 文件 | 来源 | Agents 兼容 | 默认启用 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| antigravity | Antigravity | `.gemini` | | `skills` | `GEMINI.md` | `AGENTS.md` | ✓ | ✓ |
| claude | Claude Code | `.claude` | | `skills` | `CLAUDE.md` | `AGENTS.md` | ✗ | ✓ |
| codebuddy | CodeBuddy | `.codebuddy` | | `skills` | `CODEBUDDY.md` | `AGENTS.md` | ✗ | ✓ |
| codex | Codex | `.codex` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| commandcode | Command Code | `.commandcode` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| dsh | DeepSeek Harness | `.dsh` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| omp | Oh My Pi | `.omp/agent` | `.omp` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| opencode | OpenCode | `.opencode` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| pi | Pi | `.pi/agent` | `.pi` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| qoder | Qoder | `.qoder` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| qoder-cn | Qoder CN | `.qoder-cn` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| workbuddy | WorkBuddy | `.workbuddy` | | `skills` | `CODEBUDDY.md` | `AGENTS.md` | ✗ | ✓ |
| zcode | ZCode | `.zcode` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| atomcode | AtomCode | `.atomcode` | | `skills` | `ATOMCODE.md` | `AGENTS.md` | ✓ | ✗ |
| cline | Cline | `.cline` | | `skills` | `CLAUDE.md` | `AGENTS.md` | ✓ | ✗ |
| factory | Factory | `.factory` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| jcode | JCode | `.jcode` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| kilo | Kilo Code | `.kilocode` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| kiro | Kiro | `.kiro` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✗ | ✗ |
| langcli | LangCLI | `.langcli` | | `skills` | `LANGCLI.md` | `AGENTS.md` | ✗ | ✗ |
| openclaude | OpenClaude | `.openclaude` | | `skills` | `CLAUDE.md` | `AGENTS.md` | ✗ | ✗ |
| openinterpreter | Open Interpreter | `.openinterpreter` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| grok | Grok Build CLI | `.grok` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| qwen | Qwen | `.qwen` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| zoo | Zoo Code | `.roo` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| mimocode | MiMo Code | `.config/mimocode` | `.mimocode` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| agentty | Agentty | `.agentty` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| hermes | Hermes Agent | `.hermes` | | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |

说明：

- **默认启用**（12 个）的渠道会出现在 `xskill platforms`、`find` 的交互选择与 `link --agent '*'` 等批量操作中；未启用渠道可通过 `~/.xskill/settings.json` 中对应条目的 `"enabled": true` 启用（默认 `false`），显式指定渠道名（如 `xskill link claude <skill>`）不受影响。
- **显示名称**：`name` 字段用于 `platforms`/`find`/`list` 等展示型输出，缺失时回退到配置 key。
- **路径**：`path` 为全局模式（`-g`）下的配置目录（`~/<path>/skills`）；**项目级路径**（`local_path`）仅当全局与项目级路径不同时填写，缺省时回退到 `path`。例如 `omp` 全局为 `~/.omp/agent/skills`，项目级为 `.omp/skills`。
- **Agents 兼容**：`agents_compat` 为 `true` 的渠道直接读取规范目录，add/remove/link/restore 跳过 symlink 操作；find TUI 中显示为已选中（`SELECTED`）。
- **内置渠道**：上表全部渠道均为内置（`builtin: true`），受保护——用户配置仅可覆盖 `name` 与 `enabled`，`path`、`local_path`、`skills` 等其余字段在配置加载时强制恢复内置默认值；自定义渠道（key 不在上表）不受限。
- `antigravity`（原 Gemini CLI，Google 已更名为 Antigravity CLI，配置目录沿用 `~/.gemini/`）与 `gemini` 为同一平台，内置列表仅保留 `antigravity`。
- `zoo`（Zoo Code）接手已停服的 Roo Code，配置目录沿用 `~/.roo/`。
- `qoder-cn`（Qoder 中国版）与 `qoder`（国际版）为同一产品的不同发行版，配置目录分别为 `~/.qoder-cn/` 与 `~/.qoder/`，互不共享。
- `mimocode`（MiMo Code）全局配置目录为 `~/.config/mimocode/skills`，项目级为 `.mimocode/skills`（[官方文档](https://mimo.xiaomi.com/zh/mimocode/skills)）。
- `agentty`（Agentty）全局配置目录为 `~/.agentty/skills`，项目级为 `.agentty/skills`，两者路径相同，无需 `local_path`；兼容 `.agents/` 规范目录（[官方文档](https://github.com/1ay1/agentty/blob/master/docs/website/skills.md)）。
- `hermes`（Hermes Agent）全局配置目录为 `~/.hermes/skills`，项目级为 `.hermes/skills`，两者路径相同，无需 `local_path`；兼容 `.agents/` 规范目录。
