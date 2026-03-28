<div align="center">

# VibeAround

**在浏览器和聊天软件里使用真正的 coding agents。**

[English](README.md) | [简体中文](README_CN.md) | [Wiki](https://github.com/jazzenchen/VibeAround/wiki)

<p>
  <img src="Logo.png" width="120" alt="VibeAround" />
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Bun-1.3+-000?style=flat-square&logo=bun&logoColor=fff" alt="Bun" />
  <img src="https://img.shields.io/badge/Rust-1.78+-000?style=flat-square&logo=rust&logoColor=fff" alt="Rust" />
  <img src="https://img.shields.io/badge/Vite-6-646CFF?style=flat-square&logo=vite&logoColor=fff" alt="Vite" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=000" alt="React" />
  <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License: MIT" />
</p>

</div>

VibeAround 想做的事情很直接：把真正的 coding agents 带进你本来就在用的工具里。

它让你可以从桌面、浏览器、terminal，甚至 Telegram、飞书、微信这类聊天工具里访问 `Claude Code`、`Gemini CLI`、`Codex`、`OpenCode`，但又不会让产品看起来只是某一个 agent 的附属工具。

最酷的地方也在这里：

- 用的是真正的 coding agent，不是模拟助手
- 聊天软件不再只是通知入口，也可以变成 agent 的操作入口
- 同一套产品里同时拥有 terminal、Web chat 和 IM channel access
- channel 通过插件接入，可以按不同平台能力做配置与扩展
- 让 coding agent 从“某个窗口里的工具”变成“你日常工作流的一部分”

## 界面截图

| 桌面端 | 移动端 |
|--------|--------|
| <img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/screenshots/pc.webp" width="720" alt="VibeAround 桌面端 Web 控制台" /> | <img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/screenshots/mobile-claude.webp" width="200" alt="VibeAround 移动端 Web 控制台" /> |

## 为什么它会让人眼前一亮

很多 AI coding 产品只给你一个固定入口。

而 VibeAround 更想做的是一件非常酷、也非常实用的事：把真正的 coding agent 带进你每天都在用的工具里。

这意味着你可以想象这样的使用方式：

- 在浏览器 chat 里直接驱动 `Claude Code`
- 在手机上随时查看和操作 agent
- 用 Telegram、飞书、微信这类 IM 工具真正接入 coding agent
- 保留 terminal-heavy workflow，但不必把一切都锁死在 terminal 界面里

## 当前可以做什么

- 打开 Web dashboard 管理 terminals、tmux sessions 和 chat
- 启动或附加到持久化 PTY sessions
- 在 Web chat 中与支持的 coding agents 交互
- 通过 Telegram、飞书、微信等 IM channels 访问同一套 agent 系统
- 在 onboarding 中发现可用 channel plugins，并按插件能力完成接入
- 对支持的平台使用对应配置方式，例如 bot token、app credentials 或二维码登录
- 在 desktop app 中查看运行中的 agents、channels、tunnels 和 sessions
- 在 onboarding 中选择启用哪些 agents，以及默认 agent

## 产品入口

| 入口 | 作用 |
|---|---|
| Desktop app | 首次配置、运行状态可视化、tray actions、本地控制 |
| Web dashboard | 日常主工作区，用于 terminals、tmux sessions 和 chat |
| IM channels | 通过插件接入的轻量远程入口，不同平台可有不同鉴权与消息能力 |

## Channel plugins

VibeAround 把聊天工具接入抽象成 channel plugin。

当前仓库里已经包含多种 channel 形态，例如：

- Telegram：基于 bot token 接入
- 飞书 / Lark：基于 app credentials 接入
- 微信桥接 channel：基于 provider base URL，并可由插件声明二维码登录能力

这意味着：

- 平台差异被保留，而不是被强行抹平
- onboarding 可以根据插件声明的配置模型展示不同输入项
- 某些 channel 支持 richer messaging 能力，某些 channel 则更偏向轻量接入
- 你可以把 channel 看成“远程操作入口”，而不是单纯通知机器人

## 当前支持的智能体

VibeAround 当前支持四个智能体，均已实现真实的 `StdioAcpProvider`：

- **Claude Code** — `npx @agentclientprotocol/claude-agent-acp`
- **Gemini CLI** — `gemini --experimental-acp`
- **OpenCode** — `opencode acp`
- **Codex** — `npx codex-acp`

所有智能体均通过 [ACP（Agent Client Protocol）](https://agentclientprotocol.com/) 经 stdio 通信，宿主端使用 [ACP Rust SDK](https://github.com/agentclientprotocol/rust-sdk)。启用哪些 agent、默认 agent 是什么，均在 onboarding 中配置，并写入 `~/.vibearound/settings.json`。

## 快速开始

```bash
cd src
bun install
bun run prebuild
bun run dev
```

启动后：

1. 打开 desktop app
2. 首次运行时完成 onboarding
3. 选择启用的 agents 和默认 agent
4. 如有需要，配置 tunnel 与 IM channels
5. 从托盘或 desktop UI 打开 Web dashboard
6. 通过 terminals、tmux sessions、Web chat 或已接入的 channel 开始工作

## 配置位置

运行时配置文件：

- `~/.vibearound/settings.json`

Channel plugin 构建产物：

- `~/.vibearound/plugins/<channel>/dist/main.js`

常见 channel 配置项可能包括：

- `bot_token`
- `app_id` / `app_secret`
- `base_url`
- `account_id`
- `verbose.show_thinking`
- `verbose.show_tool_use`

## 文档入口

README 只保留项目介绍与快速上手；更完整的技术说明与使用文档请查看 wiki。

建议优先阅读：

- [Wiki 首页](https://github.com/jazzenchen/VibeAround/wiki)
- [安装与运行指南](https://github.com/jazzenchen/VibeAround/wiki/Setup-Guide-CN)
- [产品入口说明](https://github.com/jazzenchen/VibeAround/wiki/Product-Surfaces-CN)
- [Channel Plugins](https://github.com/jazzenchen/VibeAround/wiki/Channel-Plugins-CN)
- [架构说明](https://github.com/jazzenchen/VibeAround/wiki/Architecture-CN)
- [配置模型](https://github.com/jazzenchen/VibeAround/wiki/Configuration-Model-CN)
- [支持的智能体](https://github.com/jazzenchen/VibeAround/wiki/Supported-Agents-CN)
- [运行语义](https://github.com/jazzenchen/VibeAround/wiki/Operational-Semantics-CN)
- [构建与打包](https://github.com/jazzenchen/VibeAround/wiki/Build-and-Packaging-CN)

## 项目状态

VibeAround 仍在持续演进中。当前版本已经可以使用，体验与文档也在继续完善。

本仓库公开的主要目的是透明、学习和分享。当前阶段暂不接受 Pull Request 和 feature request。

## 开源协议

本项目基于 [MIT License](LICENSE) 开源。
