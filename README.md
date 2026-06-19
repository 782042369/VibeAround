<div align="center">

<img src="Logo.png" alt="VibeWbz logo" width="96" />

# VibeWbz

Desktop environment configurator for Claude Code, Codex CLI, Claude Desktop, and Codex Desktop.

[简体中文](README_CN.md)

</div>

## What It Does

VibeWbz is now a small desktop-only environment setup tool for macOS and Windows.

- Detect whether Claude Code CLI, Codex CLI, Claude Desktop, and Codex Desktop are installed.
- Install missing CLI prerequisites and CLI tools.
- Open the official download page when a desktop app is missing.
- Do not write Claude/Codex config, generate profiles, or launch tools.
- After setup, guide the user to install CCS and create a token in My Gateway.
- Guide users to join the community group for $5 trial credit.

This build intentionally keeps only environment setup and follow-up guidance.

## Supported Targets

| Target | macOS | Windows |
|---|---:|---:|
| Claude Code CLI | Yes | Yes |
| Codex CLI | Yes | Yes |
| Claude Desktop | Yes | Yes |
| Codex Desktop | Yes | Yes |

## Use

1. Open VibeWbz Desktop.
2. Select Claude Code, Codex CLI, Claude Desktop, or Codex Desktop.
3. Install missing CLI environment items.
4. Install missing desktop apps from the download links.
5. Follow the CCS and gateway-token guide.

VibeWbz does not write Claude/Codex config and does not launch the CLI or desktop apps.

Codex Desktop downloads are opened from `https://codexapp.agentsmirror.com/latest/` for the current macOS or Windows architecture.

## Development

```bash
bun install
bun run --cwd src/desktop-ui build
cargo test --manifest-path src/Cargo.toml
```

Current scope is intentionally narrow: environment detection, installation, and CCS / gateway-token guidance for the four supported targets.

## Thanks

- [jazzenchen/VibeAround](https://github.com/jazzenchen/VibeAround)
- [Codex App Agents Mirror](https://codexapp.agentsmirror.com)
