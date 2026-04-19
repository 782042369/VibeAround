import { Minimize2 } from "lucide-react";
import { browserWsBaseUrl } from "@va/client";

import { Button } from "@/components/ui/button";
import type { ViewMode } from "@/lib/terminal-types";

interface AppFooterProps {
  viewMode: ViewMode;
  runningSessions: number;
  maximizedSession: string | null;
  onExitMaximized: () => void;
}

export function AppFooter({
  viewMode,
  runningSessions,
  maximizedSession,
  onExitMaximized,
}: AppFooterProps) {
  return (
    <footer className="flex items-center justify-between px-3 py-1 shrink-0 bg-muted/60 dark:bg-muted/40 border-t border-border">
      <div className="flex items-center gap-3">
        <span
          className="text-[10px] text-muted-foreground/40 font-mono truncate max-w-[180px]"
          title="WebSocket follows page host (tunnel works on phone)"
        >
          WS: {browserWsBaseUrl()}/ws
        </span>
        <span className="text-[10px] text-muted-foreground/30 font-mono">
          Tunnel: — (see desktop tray)
        </span>
      </div>
      <div className="flex items-center gap-3">
        {maximizedSession && (
          <Button
            variant="ghost"
            size="sm"
            onClick={onExitMaximized}
            className="gap-1 text-[10px] font-mono text-primary/60 hover:text-primary h-auto py-1 px-2"
          >
            <Minimize2 className="h-2.5 w-2.5" />
            Exit Maximized
          </Button>
        )}
        <span className="text-[10px] text-muted-foreground/30 font-mono uppercase">{viewMode}</span>
        <span className="text-[10px] text-muted-foreground/40 font-mono">{runningSessions} proc</span>
      </div>
    </footer>
  );
}
