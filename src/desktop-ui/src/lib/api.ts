import { invoke } from "@tauri-apps/api/core";
import { loopbackBaseUrl, loopbackWsBaseUrl } from "@va/client";

export const DAEMON_PORT = 12358;
export const API_BASE = loopbackBaseUrl(DAEMON_PORT);
const WS_BASE = loopbackWsBaseUrl(DAEMON_PORT);

export async function apiFetch(path: string, init: RequestInit = {}): Promise<Response> {
  return fetch(`${API_BASE}${path}`, init);
}

export async function authedWsUrl(path: string): Promise<string> {
  return `${WS_BASE}${path}`;
}

export async function openExternalUrl(url: string): Promise<void> {
  try {
    await invoke("open_external_url", { url });
  } catch (e) {
    console.warn("[desktop-ui] open_external_url failed, falling back:", e);
    window.open(url, "_blank", "noopener,noreferrer");
  }
}
