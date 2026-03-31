<div align="center">

# VibeAround

**Use real coding agents from your browser, desktop, and chat apps.**

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

VibeAround brings real coding agents — Claude Code, Gemini CLI, Codex, and OpenCode — into the tools you already use: desktop, browser, and messaging apps like Telegram, Feishu, Discord, and WeChat.

It's not a wrapper. It's a unified runtime where every surface gets native-feeling access to the same agent system, with real support for streaming, tool use, and thinking display.

## Screenshots

| Desktop | Mobile |
|---------|--------|
| <img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/screenshots/pc.webp" width="720" alt="VibeAround web dashboard on desktop" /> | <img src="https://pub-806a1b8456464ce7a6c110f84946697e.r2.dev/screenshots/mobile-claude.webp" width="200" alt="VibeAround web dashboard on mobile" /> |

## What you can do

- **Web dashboard** — terminals, tmux sessions, and agent chat at `localhost:12358`
- **Desktop app** — onboarding wizard, service monitoring, workspace management, tray actions
- **IM channels** — talk to agents from Telegram, Feishu, Discord, or WeChat
- **Agent switching** — switch between Claude Code, Gemini CLI, Codex, and OpenCode per session
- **Multi-workspace** — manage project folders, set defaults, add custom paths via desktop UI
- **Tunnel access** — expose your dashboard via Cloudflare Tunnel, Ngrok, or Localtunnel

## Supported agents

All agents communicate via [ACP (Agent Client Protocol)](https://agentclientprotocol.com/) over stdio.

| Agent | Status |
|---|---|
| **Claude Code** | Working |
| **Gemini CLI** | Working |
| **OpenCode** | Working |
| **Codex** | Working |

## Channel plugins

Each channel is a standalone Node.js plugin built with [@vibearound/plugin-channel-sdk](https://www.npmjs.com/package/@vibearound/plugin-channel-sdk).

| Channel | Auth | Message Editing | Status |
|---|---|---|---|
| **Telegram** | Bot token | Yes (streaming edits) | Working |
| **Feishu / Lark** | App credentials | Yes (interactive cards) | Working |
| **Discord** | Bot token | Yes (streaming edits) | Working |
| **WeChat** | QR code login | No (send-only) | Working |
| **WhatsApp** | Pairing code | No (send-only) | Blocked by [Baileys upstream issue](https://github.com/WhiskeySockets/Baileys/issues/2422) |

## Quick start

```bash
cd src
bun install
bun run prebuild
bun run dev
```

1. Desktop app opens with onboarding wizard on first run
2. Choose agents, configure channels and tunnel
3. Web dashboard available at `http://127.0.0.1:12358`
4. Start working through terminals, chat, or connected channels

## Architecture

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  Desktop    │  │    Web      │  │  IM Channel │
│  (Tauri)    │  │  Dashboard  │  │  Plugins    │
└──────┬──────┘  └──────┬──────┘  └──────┬──────┘
       │                │                │
       └────────────────┼────────────────┘
                        │
              ┌─────────┴─────────┐
              │   Rust Runtime    │
              │  ┌─────────────┐  │
              │  │  ACP Hub    │  │   ← routes prompts to agents
              │  │  (per-route │  │
              │  │   ACPPod)   │  │
              │  └──────┬──────┘  │
              │         │         │
              │  ┌──────┴──────┐  │
              │  │Agent Factory│  │   ← spawns Claude/Gemini/Codex/OpenCode
              │  └─────────────┘  │
              │                   │
              │  ┌─────────────┐  │
              │  │ PTY Manager │  │   ← terminal sessions + tmux
              │  └─────────────┘  │
              └───────────────────┘
```

## Configuration

All config lives in `~/.vibearound/settings.json`:

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

## Known issues

- **WhatsApp plugin** — Baileys v7 device linking broken upstream; plugin code ready, awaiting fix
- **Tunnel auth** — no authentication layer for tunnel-exposed dashboards
- **Plugin discovery** — channel plugins are bundled; no dynamic download/install yet
- **No release binaries** — must build from source currently
- **Workspace switching** — workspace settings saved but `/workspaces` chat command not yet implemented
- **Session persistence** — agent sessions are in-memory; lost on restart
- **System commands** — limited slash command support (`/help`); more planned

## Plugin SDK

Build your own channel plugin with the SDK:

```bash
npm install @vibearound/plugin-channel-sdk
```

See the [SDK README](https://github.com/jazzenchen/vibearound-plugin-channel-sdk) for the full guide.

## Documentation

- [Wiki Home](https://github.com/jazzenchen/VibeAround/wiki)
- [Setup Guide](https://github.com/jazzenchen/VibeAround/wiki/Setup-Guide)
- [Channel Plugins](https://github.com/jazzenchen/VibeAround/wiki/Channel-Plugins)
- [Architecture](https://github.com/jazzenchen/VibeAround/wiki/Architecture)
- [Configuration](https://github.com/jazzenchen/VibeAround/wiki/Configuration-Model)
- [FAQ & Troubleshooting](https://github.com/jazzenchen/VibeAround/wiki/FAQ-and-Troubleshooting)

## Project status

VibeAround is actively evolving. The current product is usable for daily work. Pull requests and feature requests are not being accepted at this time.

## License

[MIT](LICENSE)
