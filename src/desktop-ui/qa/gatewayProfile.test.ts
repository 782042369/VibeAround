import { expect, test } from "bun:test";

import {
  GATEWAY_PROFILE_LABEL,
  gatewayProfileDraft,
} from "../src/Launch/gatewayProfile";

test("gateway profile default label has no whitespace", () => {
  expect(GATEWAY_PROFILE_LABEL).toBe("VibeWbzGateway");
});

test("gateway profile draft removes whitespace from labels", () => {
  expect(
    gatewayProfileDraft("sk-test", undefined, " VibeWbz Gateway\tOne ", "codex")
      .label,
  ).toBe("VibeWbzGatewayOne");
  expect(gatewayProfileDraft("sk-test", undefined, " \n\t ", "codex").label).toBe(
    GATEWAY_PROFILE_LABEL,
  );
});
