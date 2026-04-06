<div align="center">

<img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/documents/v0.1/banner.webp" width="100%" alt="VibeAround — AI 编程代理统一运行时" />

# VibeAround

**AI 编程代理统一运行时 — 终端、浏览器、手机、聊天应用，随时随地。**

[English](README.md) | [简体中文](README_CN.md) | [Wiki](https://github.com/jazzenchen/VibeAround/wiki)

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.82+-000?style=flat-square&logo=rust&logoColor=fff" alt="Rust" />
  <img src="https://img.shields.io/badge/Tauri-2.10-24C8DB?style=flat-square&logo=tauri&logoColor=fff" alt="Tauri" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=000" alt="React" />
  <img src="https://img.shields.io/badge/ACP-Rust_SDK-000?style=flat-square" alt="ACP" />
  <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License: MIT" />
</p>

</div>

VibeAround 是 AI 编程代理的统一运行时。它将真正的编程代理（Claude Code、Gemini CLI、Codex CLI、Cursor CLI、Kiro CLI、Qwen Code、OpenCode）接入你日常使用的每个界面：桌面应用、浏览器、Telegram、飞书、Discord、Slack、微信。不是套壳 — 是一个完整的运行时，支持流式输出、工具调用和思考过程展示。

在 Mac 上用 Claude Code 开始一个任务，移交到手机上的 Telegram 继续对话（完整上下文保留），回到桌前再移交回终端。

## 核心功能

- **网页终端** — 浏览器内完整 PTY 终端，集成 tmux，shell 会话与 agent 对话并行运行
- **会话接力** — 将任意 agent 的编程会话一键移交到任意 IM 频道，手机上继续对话
- **Agent 切换** — 在任何频道中 `/switch claude`、`/switch codex`、`/switch cursor` 随时切换
- **网页控制台** — 终端、tmux、agent 对话，访问 `localhost:12358`
- **IM 频道** — Telegram、飞书、Discord、Slack、微信 — 每个都是独立插件
- **桌面应用** — 引导向导（含安装进度）、服务监控、工作空间管理、系统托盘
- **多工作空间** — 管理项目目录、设置默认、切换上下文
- **隧道访问** — 通过 Cloudflare Tunnel、Ngrok 或 Localtunnel 远程访问

## 支持的 Agents

所有 agent 通过 [ACP (Agent Client Protocol)](https://agentclientprotocol.com/) 经由 stdio 通信。基于 npm 的 agent 首次使用时自动安装。CLI 类 agent（Cursor、Kiro、Qwen、OpenCode）需用户自行安装。

| Agent | ACP | 会话接力 |
|---|---|---|
| **Claude Code** | 可用 | 已支持 |
| **Gemini CLI** | 可用 | 已支持 |
| **Codex CLI** | 可用 | 已支持 |
| **Cursor CLI** | 可用 | 已支持 |
| **Kiro CLI** | 可用 | 已支持 |
| **Qwen Code** | 可用 | 已支持 |
| **OpenCode** | 可用 | 不支持 |

## 频道插件

每个频道都是独立的 Node.js 插件，基于 [@vibearound/plugin-channel-sdk](https://www.npmjs.com/package/@vibearound/plugin-channel-sdk) 构建。

| 频道 | 认证方式 | 私聊 | 文件/图片 | 流式输出 | 斜杠命令 | 状态 |
|---|---|---|---|---|---|---|
| **Telegram** | Bot Token | 支持 | 支持 | 支持 | `/command` | 可用 |
| **飞书 / Lark** | 应用凭证 | 支持 | 支持 | 支持（卡片） | `/command` | 可用 |
| **Discord** | Bot Token | 支持 | 支持 | 支持 | `/command` | 可用 |
| **Slack** | Bot + App Token | 支持 | 支持 | 支持 | `/va`、`/vibearound` | 可用 |
| **微信** | 二维码登录 | 支持 | 不支持 | 不支持 | `/command` | 可用 |

## 命令

### 系统命令

| 命令 | 说明 |
|---|---|
| `/help` | 显示可用命令 |
| `/new` | 重置会话（新对话） |
| `/switch <agent>` | 切换 agent（claude、gemini、codex、cursor、kiro、qwen-code、opencode） |
| `/profile <name>` | 切换 profile |
| `/close` | 关闭对话 |
| `/pickup <code>` | 恢复一个编程 agent 会话 |
| `/handover` | 将会话导出到编程 agent CLI |

### Agent 命令

| 命令 | 说明 |
|---|---|
| `/agent <command>` | 向 agent 发送斜杠命令（如 `/agent status`） |

### Slack 专用

在 Slack 中，`/` 前缀会被客户端拦截。请使用 `/va` 或 `/vibearound` 代替：

| Slack 命令 | 等同于 |
|---|---|
| `/va help` | `/help` |
| `/va switch claude` | `/switch claude` |
| `/va agent status` | `/agent status` |
| `/va new` | `/new` |

## 环境要求

| 工具 | 版本 | 安装 |
|------|------|------|
| **Rust** | 1.82+ | [rustup.rs](https://rustup.rs/) |
| **Node.js** | 20+ | [nodejs.org](https://nodejs.org/) |
| **Bun** | 1.1+ | [bun.sh](https://bun.sh/) |
| **npm** | 10+ | 随 Node.js 一起安装 |

仅支持 macOS。需要 Xcode 命令行工具：

```bash
xcode-select --install
```

## 快速开始

```bash
cd src
bun install
bun run prebuild
bun run dev
```

1. 首次运行时桌面应用会打开引导向导
2. 选择 agents，配置频道和隧道
3. 网页控制台：`http://127.0.0.1:12358`
4. 通过终端、对话或 IM 频道开始编程

## 会话接力

将编程会话移交到任意已连接的 IM 频道 — 支持 Claude Code、Gemini CLI、Codex CLI、Cursor CLI、Kiro CLI 和 Qwen Code：

```
你 (终端)    > /handover
Agent       > 移交就绪，已复制到剪贴板：
               /pickup V5RX
               在任何已连接 VibeAround 的 IM 中粘贴。
               此代码 2 分钟内有效。
```

在 Telegram、飞书、Discord、Slack 或微信中粘贴 `/pickup` 命令 — 完整上下文继续对话。完成后再次 `/handover`，将会话移交回终端。

## 架构

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│   桌面端    │  │  网页控制台  │  │  IM 频道    │
│  (Tauri)    │  │  Dashboard  │  │   插件      │
└──────┬──────┘  └──────┬──────┘  └──────┬──────┘
       │                │                │
       └────────────────┼────────────────┘
                        │
              ┌─────────┴─────────┐
              │   Rust 运行时     │
              │  ┌─────────────┐  │
              │  │  ACP Hub    │  │   ← 将 prompt 路由到 agent
              │  │ (按路由分配  │  │
              │  │   ACPPod)   │  │
              │  └──────┬──────┘  │
              │         │         │
              │  ┌──────┴──────┐  │
              │  │ Agent 工厂  │  │   ← 启动 Claude/Gemini/Codex/Cursor/Kiro/Qwen/OpenCode
              │  └─────────────┘  │
              │                   │
              │  ┌─────────────┐  │
              │  │ PTY 管理器  │  │   ← 终端会话 + tmux
              │  └─────────────┘  │
              └───────────────────┘
```

## 配置

所有配置位于 `~/.vibearound/settings.json`：

```json
{
  "default_agent": "claude",
  "enabled_agents": ["claude", "gemini", "opencode", "codex", "cursor", "kiro", "qwen-code"],
  "workspaces": ["/path/to/your/project"],
  "channels": {
    "telegram": { "bot_token": "..." },
    "feishu": { "app_id": "...", "app_secret": "..." },
    "discord": { "bot_token": "..." },
    "slack": { "bot_token": "xoxb-...", "app_token": "xapp-..." }
  },
  "tunnel": {
    "provider": "cloudflare",
    "cloudflare": { "tunnel_token": "...", "hostname": "..." }
  }
}
```

## 插件 SDK

使用 SDK 构建自己的频道插件：

```bash
npm install @vibearound/plugin-channel-sdk
```

详见 [SDK README](https://github.com/jazzenchen/vibearound-plugin-channel-sdk)。

## 文档

- [Wiki 首页](https://github.com/jazzenchen/VibeAround/wiki)
- [安装指南](https://github.com/jazzenchen/VibeAround/wiki/Setup-Guide)
- [频道插件](https://github.com/jazzenchen/VibeAround/wiki/Channel-Plugins)
- [架构](https://github.com/jazzenchen/VibeAround/wiki/Architecture)
- [配置模型](https://github.com/jazzenchen/VibeAround/wiki/Configuration-Model)
- [FAQ 和故障排除](https://github.com/jazzenchen/VibeAround/wiki/FAQ-and-Troubleshooting)

## 项目状态

VibeAround 正在积极迭代，当前版本已可用于日常工作。

## 路线图

### 更多 IM 频道

| 频道 | 状态 |
|---|---|
| LINE | 开发中 |
| Microsoft Teams | 开发中 |
| 钉钉 | 计划中 |
| QQ | 计划中 |

### 工作空间管理

- 多项目工作空间切换与持久化
- 按工作空间配置 agent 和频道
- 工作空间级别的会话历史与上下文

## 许可证

[MIT](LICENSE)
