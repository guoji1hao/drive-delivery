import { afterEach, describe, expect, it } from "vitest";
import { locale, t, tf } from ".";
import { driveGuides } from "../utils/driveBinding";
import { formatDate, platformStateText, taskStateText } from "../utils/format";
afterEach(() => { locale.value = "zh-CN"; });
describe("English UI", () => {
  it("translates every binding step and task state", () => {
    locale.value = "en";
    for (const guide of Object.values(driveGuides)) {
      for (const text of [guide.name, guide.hint, ...guide.steps]) expect(t(text)).not.toMatch(/[\u4e00-\u9fff]/);
    }
    for (const text of [...Object.values(platformStateText), ...Object.values(taskStateText)]) expect(t(text)).not.toMatch(/[\u4e00-\u9fff]/);
    expect(formatDate()).toBe("No expiry");
  });
  it("preserves user content and translates dynamic labels and known errors", () => {
    locale.value = "en";
    expect(t("C:/用户/文件.zip")).toBe("C:/用户/文件.zip");
    expect(tf("绑定{drive}", "Connect {drive}", {drive: "Google Drive"})).toBe("Connect Google Drive");
    expect(t("配置错误：请先启用该网盘插件，再重试任务")).toBe("Configuration: Enable this drive plugin before retrying");
    locale.value = "zh-CN";
    expect(t("绑定网盘")).toBe("绑定网盘");
  });
});
