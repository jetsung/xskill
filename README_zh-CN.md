# XSkill

🌐 [English](README.md) | 简体中文

一个用于发现、安装和管理可复用 agent 技能包的技能管理工具。

## 特性

- **多源支持** — 从 Git 仓库、GitHub、GitLab 等安装技能
- **平台管理** — 一条命令安装到多个 AI 编程平台（Claude、Codex 等）
- **锁文件追踪** — 通过 `.xskill-lock.json` 实现可复现安装
- **批量操作** — 使用 `--all` 标志安装/移除所有技能
- **缓存支持** — 可选本地缓存，支持离线技能查询

## 安装

### 快速安装

**Linux / macOS：**

```bash
curl -fsSL https://xskill.gcli.cn/install.sh | bash
```

**Windows（PowerShell）：**

```powershell
irm https://xskill.gcli.cn/install.ps1 | iex
```

### 从 crates.io 安装

```bash
cargo install xskill
```

### 从 Git 安装

```bash
cargo install --git https://github.com/jetsung/xskill.git xskill
```

或从 AtomGit 安装（国内用户）：

```bash
cargo install --git https://atomgit.com/jetsung/xskill.git xskill
```

### 从源码安装

```bash
git clone https://github.com/jetsung/xskill.git
cd xskill
cargo install --path .
```

预编译二进制文件可在 [GitHub Releases](https://github.com/jetsung/xskill/releases) 获取。

## 快速开始

```bash
# 添加技能源
xskill sources add -n my-skills -u https://github.com/example/skills

# 查询可用技能
xskill query -f my-skills

# 安装技能
xskill add -f my-skills -s vue

# 安装到全局目录
xskill add -f my-skills -s vue -g

# 列出已安装技能
xskill list

# 交互式查找并安装技能（需要缓存）
xskill find

# 带预填充过滤条件的查找
xskill find --skill git

# 从指定源查找
xskill find --from antfu

# 移除技能
xskill remove -s vue
```

## 命令

| 命令 | 说明 |
|------|------|
| `sources` | 管理配置源（list/add/remove/edit） |
| `platforms` | 列出配置平台 |
| `add` | 安装技能 |
| `remove` | 移除技能 |
| `update` | 从锁文件更新已安装技能 |
| `restore` | 从项目锁文件恢复技能 |
| `list` | 列出已安装技能 |
| `query` | 从源查询技能 |
| `find` | 交互式查找并安装技能（TUI） |
| `rec` | 管理推荐技能（list/add/remove） |
| `cache` | 管理技能缓存 |
| `config` | 管理配置 |
| `new` | 创建新技能项目 |

## 配置

配置文件：`~/.xskill/settings.json`（可通过 `XSKILL_CONFIG` 环境变量覆盖）。

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

## 文档

📖 **[完整文档](docs/guide_zh-CN.md)** — 涵盖所有命令、配置选项、锁文件格式等的完整指南。

## 许可证

[Apache License 2.0](https://github.com/jetsung/xskill/blob/main/LICENSE)
