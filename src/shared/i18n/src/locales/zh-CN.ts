import { zhCNShared } from "./zh-CN/shared";
import { zhCNLaunch } from "./zh-CN/launch";
import { zhCNOnboarding } from "./zh-CN/onboarding";

export const zhCN: Record<string, string> = {
  ...zhCNShared,
  ...zhCNLaunch,
  ...zhCNOnboarding,
};
