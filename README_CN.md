<div align="center">

# VibeAround

**在浏览器、桌面端和聊天应用中使用真正的 coding agents。**

[English](README.md) | [简体中文](README_CN.md) | [Wiki](https://github.com/jazzenchen/VibeAround/wiki)

<p>
  <img src="Logo.png" width="120" alt="VibeAround" />
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.82+-000?style=flat-square&logo=rust&logoColor=fff" alt="Rust" />
  <img src="https://img.shields.io/badge/Tauri-2.10-24C8DB?style=flat-square&logo=tauri&logoColor=fff" alt="Tauri" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=000" alt="React" />
  <img src="https://img.shields.io/badge/ACP-Rust_SDK-000?style=flat-square" alt="ACP" />
  <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License: MIT" />
</p>

</div>

VibeAround 将真正的 coding agents — Claude Code、Gemini CLI、Codex、OpenCode — 带入你日常使用的工具：桌面应用、浏览器、以及 Telegram、飞书、Discord、微信等即时通讯平台。

这不是一个套壳产品。它是一个统一的运行时，每个接入面都能原生地访问同一个 agent 系统，完整支持流式输出、工具调用和思考过程展示。

## 截图

| 桌面端 | 移动端 |
|---------|--------|
| <img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/screenshots/pc.webp" width="720" alt="VibeAround 网页控制台" /> | <img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/screenshots/mobile-claude.webp" width="200" alt="VibeAround 移动端" /> |

## 功能概览

- **网页控制台** — 终端、tmux 会话和 agent 对话，访问 `localhost:12358`
- **桌面应用** — 引导向导、服务监控、工作空间管理、托盘操作
- **IM 频道** — 在 Telegram、飞书、Discord 或微信中与 agent 对话
- **Agent 切换** — 在 Claude Code、Gemini CLI、Codex 和 OpenCode 之间随时切换
- **多工作空间** — 管理项目文件夹、设置默认路径、通过桌面 UI 添加自定义目录
- **隧道访问** — 通过 Cloudflare Tunnel、Ngrok 或 Localtunnel 远程访问

## 支持的 Agents

所有 agent 通过 [ACP (Agent Client Protocol)](https://agentclientprotocol.com/) 经由 stdio 通信。

| Agent | 状态 |
|---|---|
| **Claude Code** | 可用 |
| **Gemini CLI** | 可用 |
| **OpenCode** | 可用 |
| **Codex** | 可用 |

## 频道插件

每个频道都是独立的 Node.js 插件，基于 [@vibearound/plugin-channel-sdk](https://www.npmjs.com/package/@vibearound/plugin-channel-sdk) 构建。

| 频道 | 认证方式 | 消息编辑 | 状态 |
|---|---|---|---|
| **Telegram** | Bot Token | 支持（流式编辑） | 可用 |
| **飞书 / Lark** | 应用凭证 | 支持（互动卡片） | 可用 |
| **Discord** | Bot Token | 支持（流式编辑） | 可用 |
| **微信** | 二维码登录 | 不支持（仅发送） | 可用 |
| **WhatsApp** | 配对码 | 不支持（仅发送） | 被 [Baileys 上游问题](https://github.com/WhiskeySockets/Baileys/issues/2422)阻塞 |

## 快速开始

```bash
cd src
bun install
bun run prebuild
bun run dev
```

1. 首次运行时桌面应用会打开引导向导
2. 选择 agents，配置频道和隧道
3. 网页控制台地址：`http://127.0.0.1:12358`
4. 通过终端、对话或已连接的频道开始工作

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
              │  │ Agent 工厂  │  │   ← 启动 Claude/Gemini/Codex/OpenCode
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
  "enabled_agents": ["claude", "gemini", "opencode", "codex"],
  "workspaces": ["/path/to/your/project"],
  "default_workspace": "",
  "channels": {
    "telegram": { "bot_token": "..." },
    "feishu": { "app_id": "...", "app_secret": "..." },
    "discord": { "bot_token": "..." }
  },
  "tunnel": {
    "provider": "cloudflare",
    "cloudflare": { "tunnel_token": "...", "hostname": "..." }
  }
}
```

## 已知问题

- **WhatsApp 插件** — Baileys v7 设备链接功能上游损坏；插件代码已就绪，等待修复
- **隧道认证** — 通过隧道暴露的控制台没有身份验证层
- **插件发现** — 频道插件目前是打包的；尚不支持动态下载安装
- **没有发行包** — 目前需要从源码构建
- **工作空间切换** — 工作空间设置已保存，但 `/workspaces` 聊天命令尚未实现
- **会话持久化** — agent 会话仅存在于内存中；重启后丢失
- **系统命令** — 目前 slash 命令支持有限（`/help`）；更多命令正在规划中

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

VibeAround 正在积极迭代。当前版本已可用于日常工作。暂不接受 PR 和功能请求。

## 许可证

[MIT](LICENSE)
