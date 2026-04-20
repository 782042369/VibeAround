import { useEffect, useState } from "react";

/**
 * HEADs `/` every 10s and exposes the round-trip latency. Used for the
 * "connected · XX ms" indicator in the header. Returns `null` on error.
 */
export function usePing(intervalMs = 10_000): number | null {
  const [pingMs, setPingMs] = useState<number | null>(null);

  useEffect(() => {
    if (typeof window === "undefined") return;
    const measure = async () => {
      const origin = window.location.origin;
      const start = performance.now();
      try {
        await fetch(origin + "/", { method: "HEAD", cache: "no-store" });
        setPingMs(Math.round(performance.now() - start));
      } catch {
        setPingMs(null);
      }
    };
    measure();
    const interval = setInterval(measure, intervalMs);
    return () => clearInterval(interval);
  }, [intervalMs]);

  return pingMs;
}
