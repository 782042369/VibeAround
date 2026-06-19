<div align="center">

<img src="Logo.png" alt="VibeWbz logo" width="96" />

# VibeWbz

用于 Claude Code、Codex CLI、Claude Desktop、Codex Desktop 的基础环境配置器。

[English](README.md)

</div>

## 功能范围

VibeWbz 现在只保留 macOS / Windows 桌面端基础环境检测与安装能力。

- 检测 Claude Code CLI、Codex CLI、Claude Desktop、Codex Desktop 是否已安装。
- 自动安装缺失的 CLI 基础依赖与 CLI 工具。
- 桌面 App 缺失时打开对应下载页。
- 不写入 Claude / Codex 配置，不生成 Profile，不启动工具。
- 环境完成后引导用户安装 CCS、在我的中转站创建令牌。
- 引导入群，赠送 5 美元试用。

本版本刻意只保留环境安装与后续使用引导。

## 支持目标

| 目标 | macOS | Windows |
|---|---:|---:|
| Claude Code CLI | 支持 | 支持 |
| Codex CLI | 支持 | 支持 |
| Claude Desktop | 支持 | 支持 |
| Codex Desktop | 支持 | 支持 |

## 使用方式

1. 打开 VibeWbz Desktop。
2. 选择 Claude Code、Codex CLI、Claude Desktop 或 Codex Desktop。
3. 点击安装，VibeWbz 会检测缺失项并安装 CLI 环境。
4. 桌面 App 缺失时，按页面下载入口安装。
5. 环境完成后，按引导安装 CCS，并在我的中转站创建令牌。

VibeWbz 不会写入 Claude / Codex 配置，也不会启动 CLI 或桌面应用。

Codex Desktop 会按当前 macOS / Windows 架构打开 `https://codexapp.agentsmirror.com/latest/` 下的安装包下载地址。

## 开发

```bash
bun install
bun run --cwd src/desktop-ui build
cargo test --manifest-path src/Cargo.toml
```

当前范围刻意保持精简：只做四个目标工具的环境检测、安装和 CCS / 中转令牌引导。

## 感谢

- [jazzenchen/VibeAround](https://github.com/jazzenchen/VibeAround)
- [Codex App Agents Mirror](https://codexapp.agentsmirror.com)
