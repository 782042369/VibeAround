---
name: va-session
description: Resolves the current VibeAround session ID for use by other skills. Use when another skill (va-preview, vibearound handover) needs session context, or when the user asks "what is my session ID", "get session info", or "check session status".
---

# VibeAround Session ID

Resolves the current session ID. Other VibeAround skills call this when they need session context for preview, handover, or lifecycle management.

## How to Resolve

### Method 1: Via VibeAround env vars (preferred)

Check if `VIBEAROUND_CHANNEL_KIND` and `VIBEAROUND_CHAT_ID` are set. If yes, call `get_session_id`:

```
Tool: get_session_id
Server: vibearound
Arguments:
  channel_kind: "<value of $VIBEAROUND_CHANNEL_KIND>"
  chat_id: "<value of $VIBEAROUND_CHAT_ID>"
```

Returns the exact session ID from VibeAround's internal state.

### Method 2: Let prepare_handover auto-discover

If the env vars are not set, return nothing. The calling skill should omit
`session_id`; the `prepare_handover` tool will attempt workspace-aware
auto-discovery for the current agent.

## Return Value

Return the session ID string to the calling skill. If neither method succeeds, return nothing — callers handle the missing case gracefully.
