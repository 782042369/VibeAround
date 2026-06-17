import type { CatalogEntry, ProfileDraft } from "./types";

export const GATEWAY_PROFILE_LABEL = "VibeWbz Gateway";
export const GATEWAY_BASE_URL = "http://ai.939593.xyz";
export const GATEWAY_API_TYPES = ["anthropic", "openai-responses"] as const;

const GATEWAY_MODELS = {
  anthropic: "claude-sonnet-4-5",
  "openai-responses": "gpt-5.5",
} as const;

export function gatewayProvider(): CatalogEntry {
  return {
    id: "custom",
    label: "VibeWbz Gateway",
    icon: null,
    homepage: GATEWAY_BASE_URL,
    endpoints: [],
  };
}

export function gatewayProfileDraft(
  apiKey: string,
  baseUrl = GATEWAY_BASE_URL,
  label = GATEWAY_PROFILE_LABEL,
): ProfileDraft {
  const normalizedBaseUrl = baseUrl.trim().replace(/\/+$/, "") || GATEWAY_BASE_URL;
  return {
    label: label.trim() || GATEWAY_PROFILE_LABEL,
    provider: "custom",
    auth_mode: "api_key",
    api_types: [...GATEWAY_API_TYPES],
    credentials: {
      api_key: apiKey.trim(),
    },
    overrides: {
      anthropic: {
        base_url: normalizedBaseUrl,
        model: GATEWAY_MODELS.anthropic,
      },
      "openai-responses": {
        base_url: normalizedBaseUrl,
        model: GATEWAY_MODELS["openai-responses"],
      },
    },
    use_settings_proxy: false,
    provider_settings: {},
  };
}
