import { describe, expect, it } from "vitest";
import { formatBytes, platformStateText } from "./format";

describe("format helpers", () => {
  it("formats binary file sizes", () => {
    expect(formatBytes(1024 * 1024)).toBe("1.0 MB");
  });

  it("keeps user-facing retry text", () => {
    expect(platformStateText.retryable).toContain("可重试");
  });
});
