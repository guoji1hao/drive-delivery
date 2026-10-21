import { describe, expect, it } from "vitest";
import { bindingError, parseGoogleDesktopConfig } from "./driveBinding";

describe("Google 配置导入", () => {
  it("reads a desktop download without needing manual fields", () => {
    expect(parseGoogleDesktopConfig(JSON.stringify({ installed: { client_id: "123-demo.apps.googleusercontent.com", client_secret: "fixture-secret" } })))
      .toEqual({ clientId: "123-demo.apps.googleusercontent.com", clientSecret: "fixture-secret" });
  });
  it("rejects wrong app types, oversized files and invalid values without echoing input", () => {
    for (const text of ['{"web":{}}', 'null', '"private-input"', '{"installed":{"client_id":12}}', 'x'.repeat(65537)]) {
      expect(() => parseGoogleDesktopConfig(text)).toThrow();
    }
    expect(() => parseGoogleDesktopConfig('private-input')).toThrow(/文件格式不对/);
  });
  it("explains common failures in plain Chinese", () => {
    expect(bindingError("google", "redirect_uri_mismatch")).toContain("桌面应用");
    expect(bindingError("onedrive", "网络请求超时")).toContain("检查网络");
    expect(bindingError("baidu", "secret unexpected error")).not.toContain("secret");
  });
});
