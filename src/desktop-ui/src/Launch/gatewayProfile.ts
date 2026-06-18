import type { CatalogEntry, ProfileDraft } from "./types";

export const GATEWAY_PROFILE_LABEL = "VibeWbzGateway";
export const GATEWAY_BASE_URL = "https://ai.939593.xyz";

export type GatewayProfileMode = "claude" | "codex";

const GATEWAY_MODELS = {
  anthropic: "claude-sonnet-4-5",
  "openai-responses": "gpt-5.5",
} as const;

export function gatewayProvider(): CatalogEntry {
  return {
    id: "custom",
    label: GATEWAY_PROFILE_LABEL,
    icon: null,
    homepage: GATEWAY_BASE_URL,
    endpoints: [],
  };
}

export function gatewayProfileDraft(
  apiKey: string,
  baseUrl = GATEWAY_BASE_URL,
  label = GATEWAY_PROFILE_LABEL,
  mode: GatewayProfileMode,
  models: {
    claudeModel?: string;
    claudeHaikuModel?: string;
    claudeSonnetModel?: string;
    claudeOpusModel?: string;
    gptModel?: string;
  } = {},
): ProfileDraft {
  const normalizedBaseUrl = baseUrl.trim().replace(/\/+$/, "") || GATEWAY_BASE_URL;
  const claudeModel = models.claudeModel?.trim() || GATEWAY_MODELS.anthropic;
  const gptModel = models.gptModel?.trim() || GATEWAY_MODELS["openai-responses"];
  return {
    label: label.replace(/\s/g, "") || GATEWAY_PROFILE_LABEL,
    provider: "custom",
    auth_mode: "api_key",
    api_types: [mode === "claude" ? "anthropic" : "openai-responses"],
    credentials: {
      api_key: apiKey.trim(),
    },
    overrides:
      mode === "claude"
        ? {
            anthropic: {
              base_url: normalizedBaseUrl,
              model: claudeModel,
              claude_default_haiku_model: models.claudeHaikuModel?.trim() ?? "",
              claude_default_sonnet_model:
                models.claudeSonnetModel?.trim() || claudeModel,
              claude_default_opus_model: models.claudeOpusModel?.trim() ?? "",
            },
          }
        : {
            "openai-responses": {
              base_url: normalizedBaseUrl,
              model: gptModel,
            },
          },
    use_settings_proxy: false,
    provider_settings: {},
  };
}
