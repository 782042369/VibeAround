---
name: vibearound
description: "VibeAround Session Handover \u2014 hand over your current coding session to an IM channel (Feishu, Discord, WeChat, Telegram) so you can continue the conversation on your phone or another device. Use when the user says \"/vibearound handover\", \"hand over to feishu\", \"continue on my phone\", \"send this to discord\", or similar session transfer requests."
---

# VibeAround Session Handover

This skill hands over the current Claude Code session to an IM channel via the VibeAround orchestrator, allowing the user to continue the conversation on another device.

## When to Use

**Trigger conditions:**
- User says `/vibearound handover <channel>`
- User asks to "hand over", "transfer", or "continue" the session on an IM platform
- User mentions sending the session to Feishu, Discord, Telegram, WeChat, or the web dashboard

## Prerequisites

The VibeAround MCP server must be available. If the `vibearound` MCP server is not configured, tell the user:
> "VibeAround MCP server is not connected. Make sure the VibeAround desktop app is running."

## Handover Flow

### Step 1: Resolve the session ID

Read the current Claude session metadata:

```bash
cat ~/.claude/sessions/$PPID.json
```

Extract the `sessionId` field from the JSON output. If the file doesn't exist or has no `sessionId`, inform the user that session metadata is unavailable.

### Step 2: Determine the target channel

Map the user's request to a channel identifier:

| User says | Channel value |
|-----------|---------------|
| feishu, lark | `feishu` |
| telegram | `telegram` |
| discord | `discord` |
| wechat, weixin | `weixin` |
| web, dashboard, browser | `web` |

If the channel is ambiguous, ask the user which one they mean.

### Step 3: Call prepare_handover

```
Tool: prepare_handover
Server: vibearound
Arguments:
  target_channel: "<channel>"
  session_id: "<sessionId>"
  cwd: "<current working directory>"
  agent_kind: "claude"
```

**If the workspace is not registered**, the tool will return an error indicating so. Ask the user for confirmation, then register it:

```
Tool: register_workspace
Server: vibearound
Arguments:
  cwd: "<current working directory>"
```

Then retry `prepare_handover`.

### Step 4: Present the result

On success, the tool returns a `/pickup` command string. Show it to the user clearly:

> Session is ready for handover. Send this command in your **{channel}** chat:
>
> `/pickup {session_id} {cwd}`

Explain that pasting this command in the IM channel will resume the session there with full context.

## Error Handling

- **MCP server not available**: Tell the user to start the VibeAround desktop app.
- **Workspace not registered**: Offer to register it (requires user confirmation).
- **Channel not recognized**: List the supported channels and ask the user to pick one.
- **Session ID not found**: The session metadata file may not exist; inform the user.
