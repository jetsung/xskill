# Platforms

兼容性说明：`zcode`、`atomcode` 完美兼容；`codebuddy`、`claude`、`openclaude` 不完全兼容。

以下列表与 `default_platforms()` 内置默认配置一致（`~/.xskill/settings.json` 初始化时生成）。

| 配置 key | 显示名称 | 路径 | Skills 目录 | Agents 文件 | 来源 | Agents 兼容 | 默认启用 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| antigravity | Antigravity | `.gemini` | `skills` | `GEMINI.md` | `AGENTS.md` | ✓ | ✓ |
| claude | Claude Code | `.claude` | `skills` | `CLAUDE.md` | `AGENTS.md` | ✗ | ✓ |
| codebuddy | CodeBuddy | `.codebuddy` | `skills` | `CODEBUDDY.md` | `AGENTS.md` | ✗ | ✓ |
| codex | Codex | `.codex` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| commandcode | Command Code | `.commandcode` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| omp | Oh My Pi | `.omp/agent` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| opencode | OpenCode | `.opencode` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| pi | Pi | `.pi/agent` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| qoder | Qoder | `.qoder` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| zcode | ZCode | `.zcode` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✓ |
| atomcode | AtomCode | `.atomcode` | `skills` | `ATOMCODE.md` | `AGENTS.md` | ✓ | ✗ |
| cline | Cline | `.cline` | `skills` | `CLAUDE.md` | `AGENTS.md` | ✓ | ✗ |
| factory | Factory | `.factory` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| jcode | JCode | `.jcode` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| kilo | Kilo Code | `.kilocode` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| kiro | Kiro | `.kiro` | `skills` | `AGENTS.md` | `AGENTS.md` | ✗ | ✗ |
| langcli | LangCLI | `.langcli` | `skills` | `LANGCLI.md` | `AGENTS.md` | ✗ | ✗ |
| openclaude | OpenClaude | `.openclaude` | `skills` | `CLAUDE.md` | `AGENTS.md` | ✗ | ✗ |
| openinterpreter | Open Interpreter | `.openinterpreter` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| qwen | Qwen | `.qwen` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |
| zoo | Zoo Code | `.roo` | `skills` | `AGENTS.md` | `AGENTS.md` | ✓ | ✗ |

说明：

- **默认启用**（9 个）的渠道会出现在 `xskill platforms`、`find` 的交互选择与 `link --agent '*'` 等批量操作中；未启用渠道可通过 `~/.xskill/settings.json` 中对应条目的 `"enabled": true` 启用（默认 `false`），显式指定渠道名（如 `xskill link claude <skill>`）不受影响。
- **显示名称**：`name` 字段用于 `platforms`/`find`/`list` 等展示型输出，缺失时回退到配置 key。
- **Agents 兼容**：`agents_compat` 为 `true` 的渠道直接读取规范目录，add/remove/link/restore 跳过 symlink 操作；find TUI 中显示为已选中（`SELECTED`）。
- `antigravity`（原 Gemini CLI，Google 已更名为 Antigravity CLI，配置目录沿用 `~/.gemini/`）与 `gemini` 为同一平台，内置列表仅保留 `antigravity`。
- `zoo`（Zoo Code）接手已停服的 Roo Code，配置目录沿用 `~/.roo/`。
