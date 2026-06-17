/**
 * Shared routing constants for the Tauri desktop UI. Matches the
 * `.nest("/va", ...)` mount point defined in `src/server/src/web_server/mod.rs`.
 */

/**
 * All local daemon HTTP + WebSocket routes live under this prefix.
 */
export const VA_PREFIX = "/va";

/** Prepend `VA_PREFIX` to an API path. */
export function apiPath(path: string): string {
  return `${VA_PREFIX}${path}`;
}

/** Loopback HTTP base used by the Tauri desktop-ui, whose webview origin
 *  is NOT the daemon's origin. */
export function loopbackBaseUrl(port: number): string {
  return `http://127.0.0.1:${port}${VA_PREFIX}`;
}

/** Loopback WebSocket base for the Tauri desktop-ui. */
export function loopbackWsBaseUrl(port: number): string {
  return `ws://127.0.0.1:${port}${VA_PREFIX}`;
}
