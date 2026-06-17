<div align="center">

<img src="Logo.png" alt="VibeWbz logo" width="96" />

# VibeWbz

用于 Claude Code、Codex CLI、Claude Desktop、Codex Desktop 的桌面启动器。

[English](README.md)

</div>

## 功能范围

VibeWbz 现在只保留 macOS / Windows 桌面端的一键配置与启动能力。

- 配置一个默认中转站 Profile。
- 默认中转站地址：`http://ai.939593.xyz`。
- 用户只需要输入自己的 key。
- 启动 Claude Code CLI、Codex CLI、Claude Desktop、Codex Desktop。
- CLI 工具启动前可以选择工作目录。

本版本刻意只保留桌面端配置与启动流程。

## 支持目标

| 目标 | macOS | Windows |
|---|---:|---:|
| Claude Code CLI | 支持 | 支持 |
| Codex CLI | 支持 | 支持 |
| Claude Desktop | 支持 | 支持 |
| Codex Desktop | 支持 | 支持 |

## 默认中转站配置

新建 Profile 时会生成 `VibeWbz Gateway`。

| 配置项 | 默认值 |
|---|---|
| Base URL | `http://ai.939593.xyz` |
| Key | 用户输入 |
| Anthropic 模型 | `claude-sonnet-4-5` |
| OpenAI Responses 模型 | `gpt-5.5` |

key 只保存在本机 VibeWbz Profile 存储中。

## 使用方式

1. 打开 VibeWbz Desktop。
2. 选择 Claude Code、Codex CLI、Claude Desktop 或 Codex Desktop。
3. 点击 `New profile`，输入中转站 key。
4. 启动 CLI 目标时选择工作目录。
5. 点击 `LAUNCH`。

Desktop 目标会唤醒已安装的桌面应用。CLI 目标会用所选 Profile 配置启动本机对应 CLI。

Codex Desktop 会按当前 macOS / Windows 架构打开 `https://codexapp.agentsmirror.com/latest/` 下的安装包下载地址。

## 开发

```bash
bun install
bun run --cwd src/desktop-ui build
cargo test --manifest-path src/Cargo.toml
```

当前范围刻意保持精简：只做四个目标工具的桌面端配置与一键启动。

## 感谢

- [jazzenchen/VibeAround](https://github.com/jazzenchen/VibeAround)
- [Codex App Agents Mirror](https://codexapp.agentsmirror.com)
