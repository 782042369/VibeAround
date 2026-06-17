<div align="center">

<img src="Logo.png" alt="VibeWbz logo" width="96" />

# VibeWbz

Desktop launcher for Claude Code, Codex CLI, Claude Desktop, and Codex Desktop.

[简体中文](README_CN.md)

</div>

## What It Does

VibeWbz is now a small desktop-only setup and launch tool for macOS and Windows.

- Configure one gateway profile with your API key.
- Use the default gateway base URL: `http://ai.939593.xyz`.
- Launch Claude Code CLI, Codex CLI, Claude Desktop, or Codex Desktop.
- Pick a workspace folder for CLI tools before launch.

This build intentionally keeps only the desktop setup and launch flow.

## Supported Targets

| Target | macOS | Windows |
|---|---:|---:|
| Claude Code CLI | Yes | Yes |
| Codex CLI | Yes | Yes |
| Claude Desktop | Yes | Yes |
| Codex Desktop | Yes | Yes |

## Gateway Defaults

New profiles are created as `VibeWbz Gateway`.

| Setting | Value |
|---|---|
| Base URL | `http://ai.939593.xyz` |
| Key | Entered by the user |
| Anthropic model | `claude-sonnet-4-5` |
| OpenAI Responses model | `gpt-5.5` |

The key is stored locally in VibeWbz profile storage.

## Use

1. Open VibeWbz Desktop.
2. Select Claude Code, Codex CLI, Claude Desktop, or Codex Desktop.
3. Click `New profile` and enter your gateway key.
4. Choose a workspace when launching a CLI target.
5. Click `LAUNCH`.

Desktop targets open the installed desktop app. CLI targets open the corresponding local CLI with the selected profile configuration.

Codex Desktop downloads are opened from `https://codexapp.agentsmirror.com/latest/` for the current macOS or Windows architecture.

## Development

```bash
bun install
bun run --cwd src/desktop-ui build
cargo test --manifest-path src/Cargo.toml
```

Current scope is intentionally narrow: desktop setup plus one-click launch for the four supported targets.

## Thanks

- [jazzenchen/VibeAround](https://github.com/jazzenchen/VibeAround)
- [Codex App Agents Mirror](https://codexapp.agentsmirror.com)
