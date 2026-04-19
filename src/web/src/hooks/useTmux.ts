import { useCallback, useEffect, useState } from "react";

import { getTmuxSessions } from "@/api/sessions";

interface UseTmuxResult {
  available: boolean | null;
  sessions: string[];
  refresh: () => Promise<void>;
}

/**
 * Owns tmux availability + session-list state. Pre-fetches on mount so
 * the dropdown has data the first time it opens; callers can re-invoke
 * `refresh` when the dropdown opens or after a new attach.
 */
export function useTmux(): UseTmuxResult {
  const [available, setAvailable] = useState<boolean | null>(null);
  const [sessions, setSessions] = useState<string[]>([]);

  const refresh = useCallback(async () => {
    try {
      const res = await getTmuxSessions();
      setAvailable(res.available);
      setSessions(res.sessions);
    } catch {
      setAvailable(false);
      setSessions([]);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { available, sessions, refresh };
}
