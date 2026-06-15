---
name: va-session
description: Resolve your current session ID for use with other VibeAround tools. Called by other skills that need session context (e.g. va-preview, vibearound handover).
---

# VibeAround Session ID

Resolve your current session ID. Other VibeAround skills reference this skill when they need session context for lifecycle management.

## How to Resolve

For Claude Code and Claude Desktop, use this rendered value as the current
session ID:

```
${CLAUDE_SESSION_ID}
```

If the rendered value is non-empty and is not the literal placeholder text,
return it directly. For testing, if the user asks to check the session ID,
show exactly this value.

## Return Value

Return the session ID string to the calling skill. If the value is empty or is
still the literal placeholder text, return nothing — callers handle the missing
case gracefully.
