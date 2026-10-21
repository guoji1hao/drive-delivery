import type { Provider } from "../types";

export const driveGuides: Record<Provider, { name: string; hint: string; setupUrl: string; helpUrl: string; steps: string[] }> = {
  baidu: {
    name: "百度网盘",
    hint: "打开百度页面，登录你的账号，按页面提示同意即可。",
    setupUrl: "https://yun.baidu.com/open/platform",
    helpUrl: "https://openauth.baidu.com/doc/doc.html",
    steps: [
      "打开百度官方平台，用百度账号登录，进入控制台创建应用；如平台要求申请资格或审核，先按提示完成。",
      "在应用详情里找到 App Key（有些页面叫 API Key）和 Secret Key，把两项分别复制到下面。它们不是你的网盘密码。",
      "点“保存并去登录”。之后在百度网页确认允许本软件使用网盘，回到这里等“已绑定”即可。上传目录还要使用平台分配的 /apps/应用目录/。",
    ],
  },
  google: {
    name: "谷歌网盘",
    hint: "打开 Google 页面，选择你的账号，确认允许上传文件即可。",
    setupUrl: "https://console.cloud.google.com/",
    helpUrl: "https://developers.google.com/workspace/drive/api/quickstart/python?hl=zh-cn",
    steps: [
      "打开 Google 官方控制台，创建或选择一个项目。在“API 和服务 → 库”搜索 Google Drive API，点“启用”。",
      "进入 Google Auth Platform（Google 身份验证平台），按提示填写应用名称和联系邮箱；个人使用通常选“外部”。如果应用还在测试，在“受众 / Audience”的测试用户中添加你要登录的 Google 邮箱。",
      "在“客户端 / Clients”中创建 OAuth 客户端，应用类型一定选“桌面应用 / Desktop app”。下载 JSON 文件，在下面点“导入 Google 配置文件”，不用逐项抄写。",
      "点“保存并去登录”，用刚才添加的邮箱登录并同意。看到“已绑定”就完成了。如果网页打不开，先确认本机能正常访问 Google。",
    ],
  },
  onedrive: {
    name: "微软网盘",
    hint: "复制下方验证码，在微软网页粘贴，登录后点“接受”。",
    setupUrl: "https://entra.microsoft.com/",
    helpUrl: "https://learn.microsoft.com/zh-cn/entra/identity-platform/quickstart-register-app",
    steps: [
      "打开微软官方管理中心，进入“应用注册 → 新注册”。名称可填“网盘发货助手”；账号类型选择包含“任何组织目录”和“个人 Microsoft 账号”的选项，再点注册。没有创建权限时，请让软件提供者或管理员完成。",
      "在应用“概述”页复制“应用程序（客户端）ID”，粘贴到下面。不要复制“目录（租户）ID”，也不用创建客户端密码。",
      "进入应用的“身份验证 / Authentication”，找到“允许公共客户端流 / Allow public client flows”，改为“是”并保存。",
      "在“API 权限 → 添加权限 → Microsoft Graph → 委托的权限”中添加 Files.ReadWrite；如需管理员批准，请联系管理员。然后回到这里点“保存并去登录”，按提示输入验证码并同意。",
    ],
  },
};

export function parseGoogleDesktopConfig(text: string): { clientId: string; clientSecret?: string } {
  if (text.length > 65536) throw new Error("这个文件太大了，请选择 Google 下载的应用配置 JSON 文件。");
  let value;
  try { value = JSON.parse(text); } catch { throw new Error("文件格式不对，请选择 Google 下载的 JSON 文件，不要选截图或文档。"); }
  const installed = value?.installed;
  if (!installed || typeof installed !== "object") throw new Error("请选择“桌面应用”下载的配置文件，网页应用或服务账号的文件不能用。");
  if (typeof installed.client_id !== "string" || !/^[\w.-]+\.apps\.googleusercontent\.com$/.test(installed.client_id)) {
    throw new Error("文件里没有有效的应用编号，请重新下载桌面应用配置。");
  }
  if (installed.client_secret !== undefined && typeof installed.client_secret !== "string") throw new Error("配置文件内容不完整，请重新下载。");
  return { clientId: installed.client_id, clientSecret: installed.client_secret };
}

export function bindingError(provider: Provider, error: unknown): string {
  const text = String(error);
  if (/invalid_client|unauthorized_client|AADSTS7000218|Client ID|App Key|Secret Key/.test(text)) return "应用配置不正确或尚未开通。展开“首次设置”核对，或请软件提供者帮你配置。";
  if (/redirect_uri_mismatch/.test(text)) return "Google 应用类型选错了，请在首次设置里换成“桌面应用”的配置文件。";
  if (/access_denied|拒绝|未完成/.test(text)) return "这次没有完成同意操作。重新点绑定，在官方网页登录并同意；公司账号若被拦截，需要管理员允许。";
  if (/超时|timeout|网络|连接/i.test(text)) return provider === "google" ? "暂时连不上 Google。先试试浏览器能否打开 Google，再回来重试。" : "暂时连不上网盘，请检查网络后重试。";
  if (/expired|过期|失效/.test(text)) return "这次登录已过期。重新点绑定，使用新出现的验证码或登录页面。";
  return "绑定没有完成，请重试。也可以展开下方“排查信息”，请软件提供者帮你看看。";
}
