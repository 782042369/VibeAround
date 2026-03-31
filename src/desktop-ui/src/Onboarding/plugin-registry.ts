/**
 * Supported channel plugin registry.
 *
 * Lists all officially supported plugins with their GitHub repos.
 * The onboarding UI uses this to show installable plugins and
 * link to documentation.
 *
 * Plugins are installed to ~/.vibearound/plugins/<id>/ via git clone.
 */

export interface PluginRegistryEntry {
  /** Plugin ID matching plugin.json "id" field. */
  id: string;
  /** Display name shown in the UI. */
  name: string;
  /** Short description for the plugin card. */
  description: string;
  /** GitHub repo URL (used for clone and docs link). */
  github: string;
}

export const PLUGIN_REGISTRY: PluginRegistryEntry[] = [
  {
    id: "telegram",
    name: "Telegram",
    description: "Use a BotFather token to chat with VibeAround in Telegram.",
    github: "https://github.com/jazzenchen/vibearound-plugin-telegram",
  },
  {
    id: "feishu",
    name: "Feishu (Lark)",
    description: "Provide app credentials for your Feishu bot integration.",
    github: "https://github.com/jazzenchen/vibearound-plugin-feishu",
  },
  {
    id: "discord",
    name: "Discord",
    description: "Use a Discord bot token to chat via @mention or DM.",
    github: "https://github.com/jazzenchen/vibearound-plugin-discord",
  },
  {
    id: "weixin-openclaw-bridge",
    name: "WeChat",
    description: "Connect via QR code authorization through OpenClaw bridge.",
    github: "https://github.com/jazzenchen/vibearound-plugin-weixin-openclaw-bridge",
  },
  {
    id: "whatsapp",
    name: "WhatsApp",
    description: "Connect via pairing code (Baileys). Currently blocked by upstream issue.",
    github: "https://github.com/jazzenchen/vibearound-plugin-whatsapp",
  },
];
