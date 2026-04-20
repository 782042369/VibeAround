/**
 * Shared routing constants for both the web dashboard and the Tauri
 * desktop-ui. Matches the `.nest("/va", ...)` mount point defined in
 * `src/server/src/web_server/mod.rs`.
 *
 * Keeping this in one place is what stops the four separate copies this
 * used to live in from drifting (they did).
 */

/**
 * All dashboard HTTP + WebSocket routes live under this prefix. The root
 * namespace is reserved for the cookie-based dev-server preview proxy,
 * so everything the SPA talks to must include it.
 */
export const VA_PREFIX = "/va";

/** Prepend `VA_PREFIX` to an API path. */
export function apiPath(path: string): string {
  return `${VA_PREFIX}${path}`;
}

/** Base URL for the browser-served web dashboard. Uses the current page's
 *  origin so it works for loopback, tunnels, and any reverse proxy. */
export function browserBaseUrl(): string {
  if (typeof window === "undefined") {
    // Non-browser context (SSR, tests). Fall back to loopback.
    return `http://127.0.0.1:12358${VA_PREFIX}`;
  }
  return `${window.location.origin}${VA_PREFIX}`;
}

/** Base WebSocket URL for the browser-served web dashboard. */
export function browserWsBaseUrl(): string {
  if (typeof window === "undefined") {
    return `ws://127.0.0.1:12358${VA_PREFIX}`;
  }
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}${VA_PREFIX}`;
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
