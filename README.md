<div align="center">

# VibeAround

**Use real coding agents from your browser and chat apps.**

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

VibeAround does something simple: it brings real coding agents into the tools you already use.

It gives you access to `Claude Code`, `Gemini CLI`, `Codex`, and `OpenCode` from desktop, browser, terminals, and chat surfaces such as Telegram, Feishu, and WeChat — without making the product feel like a wrapper around just one agent.

- use real coding agents, not a fake assistant
- turn chat apps into actual entry points for coding agents
- keep terminals, web chat, and IM channel access in one product
- plug channels in with platform-specific capabilities and configuration models
- make coding agents feel like part of your everyday workflow, not just a tool trapped in one window

## Screenshots

| Desktop | Mobile |
|---------|--------|
| <img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/screenshots/pc.webp" width="720" alt="VibeAround web dashboard on desktop" /> | <img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/screenshots/mobile-claude.webp" width="200" alt="VibeAround web dashboard on mobile" /> |

## Why VibeAround

Most AI coding products give you a single surface.

VibeAround is trying to do something much cooler: make real coding agents accessible from the tools you actually use every day.

That means you can imagine workflows like:

- driving `Claude Code` from a browser chat
- checking in on work from your phone
- using Telegram, Feishu, or WeChat as a real entry point to coding agents
- keeping terminal-heavy workflows available without forcing everything through the terminal UI itself

## What you can do today

- Open a web dashboard for terminals, tmux sessions, and chat
- Launch or attach to persistent PTY sessions
- Talk to supported coding agents from the web chat surface
- Reach the same agent system through IM channels such as Telegram, Feishu, and WeChat
- Discover available channel plugins during onboarding and configure them according to plugin capabilities
- Use platform-appropriate connection flows such as bot tokens, app credentials, or QR login
- Inspect running agents, channels, tunnels, and sessions from the desktop app
- Choose enabled agents and the default agent during onboarding

## Product surfaces

| Surface | Purpose |
|---|---|
| Desktop app | Onboarding, runtime visibility, tray actions, and local control |
| Web dashboard | Main daily workspace for terminals, tmux sessions, and chat |
| IM channels | Lightweight remote access through plugins, with platform-specific auth and messaging capabilities |

## Channel plugins

VibeAround models chat integrations as channel plugins.

The repository already includes multiple channel styles, for example:

- Telegram: token-based bot integration
- Feishu / Lark: app credential based integration
- WeChat bridge channel: provider base URL based integration, with optional QR login support declared by the plugin

This keeps the model practical:

- platform differences are preserved instead of flattened away
- onboarding can render different forms based on each plugin's declared config schema
- some channels can expose richer messaging features while others stay intentionally lightweight
- channels act as remote operating surfaces, not just notification bots

## Supported agents

VibeAround currently supports:

- Claude Code
- Gemini CLI
- OpenCode
- Codex

Agent enablement and the default agent are configured during onboarding and stored in `~/.vibearound/settings.json`.

## Quick start

```bash
cd src
bun install
bun run prebuild
bun run dev
```

After startup:

1. open the desktop app
2. complete onboarding on first run
3. choose enabled agents and the default agent
4. configure a tunnel and IM channels if needed
5. open the web dashboard from the tray or desktop UI
6. start working through terminals, tmux sessions, web chat, or connected channels

## Configuration

Runtime configuration:

- `~/.vibearound/settings.json`

Channel plugin bundles:

- `~/.vibearound/plugins/<channel>/dist/main.js`

Common channel settings may include:

- `bot_token`
- `app_id` / `app_secret`
- `base_url`
- `account_id`
- `verbose.show_thinking`
- `verbose.show_tool_use`

## Documentation

This README stays focused on product overview and fast onboarding. The wiki contains the technical and usage documentation.

Recommended starting points:

- [Wiki Home](https://github.com/jazzenchen/VibeAround/wiki)
- [Setup Guide](https://github.com/jazzenchen/VibeAround/wiki/Setup-Guide)
- [Product Surfaces](https://github.com/jazzenchen/VibeAround/wiki/Product-Surfaces)
- [Channel Plugins](https://github.com/jazzenchen/VibeAround/wiki/Channel-Plugins)
- [Architecture](https://github.com/jazzenchen/VibeAround/wiki/Architecture)
- [Configuration Model](https://github.com/jazzenchen/VibeAround/wiki/Configuration-Model)
- [Supported Agents](https://github.com/jazzenchen/VibeAround/wiki/Supported-Agents)
- [Operational Semantics](https://github.com/jazzenchen/VibeAround/wiki/Operational-Semantics)
- [Build and Packaging](https://github.com/jazzenchen/VibeAround/wiki/Build-and-Packaging)

## Project status

VibeAround is actively evolving. The current product is already usable, while the experience and documentation continue to improve.

The repository is public for transparency and learning. Pull requests and feature requests are not being accepted at this time.

## License

This project is licensed under the [MIT License](LICENSE).
