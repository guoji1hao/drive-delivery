<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "../services/api";
import type { AuthChallenge, AuthStatus, Provider } from "../types";
import { bindingError, driveGuides, parseGoogleDesktopConfig } from "../utils/driveBinding";

const props = defineProps<{ provider: Provider; status?: AuthStatus }>();
const emit = defineEmits<{ changed: [status: AuthStatus]; toast: [message: string] }>();
const guide = computed(() => driveGuides[props.provider]);
const handlers = computed(() => ({
  baidu: { start: api.startBaiduAuth, poll: api.pollBaiduAuth, clear: api.clearBaiduAuth },
  google: { start: api.startGoogleAuth, poll: api.pollGoogleAuth, clear: api.clearGoogleAuth },
  onedrive: { start: api.startOneDriveAuth, poll: api.pollOneDriveAuth, clear: api.clearOneDriveAuth },
}[props.provider]));
const setupOpen = ref(false);
const manualOpen = ref(false);
const clientId = ref("");
const secret = ref("");
const imported = ref(false);
const busy = ref(false);
const challenge = ref<AuthChallenge>();
const message = ref("");
const error = ref("");
const technicalError = ref("");
const codeCopied = ref(false);
const confirmUnbind = ref(false);
let disposed = false;
let run = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
let wake: (() => void) | undefined;

function stopWaiting() {
  run++;
  clearTimeout(timer);
  wake?.();
  wake = undefined;
  busy.value = false;
  challenge.value = undefined;
}
onBeforeUnmount(() => { disposed = true; stopWaiting(); });

async function openOfficial(url: string) {
  try { await openUrl(url); } catch { error.value = "浏览器没能打开。请检查电脑的默认浏览器，再点一次对应的打开按钮。"; }
}

async function copyCode() {
  if (!challenge.value?.userCode) return;
  try {
    await navigator.clipboard.writeText(challenge.value.userCode);
    codeCopied.value = true;
  } catch {
    codeCopied.value = false;
    message.value = "验证码没能自动复制，请选中下方验证码，按 Ctrl+C 复制。";
  }
}

async function importGoogle(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  error.value = "";
  imported.value = false;
  try {
    if (file.size > 65536) throw new Error("请选择 Google 下载的应用配置 JSON 文件，文件应小于 64 KB。");
    const config = parseGoogleDesktopConfig(await file.text());
    clientId.value = config.clientId;
    secret.value = config.clientSecret ?? "";
    imported.value = true;
  } catch (cause) {
    clientId.value = "";
    secret.value = "";
    error.value = (cause as Error).message;
  } finally { input.value = ""; }
}

async function bind(saveConfig = false) {
  if (busy.value) return;
  error.value = "";
  technicalError.value = "";
  message.value = "";
  if (!saveConfig && !props.status?.available) {
    setupOpen.value = true;
    message.value = "还差首次设置：请按下面的说明准备一次，保存后就不用再填了。";
    return;
  }
  if (saveConfig && (!clientId.value.trim() || (props.provider === "baidu" && !secret.value.trim()))) {
    error.value = props.provider === "google" ? "先导入 Google 配置文件，或展开手动填写。" : "请先填写下面的应用配置。这不是你的网盘账号或密码。";
    return;
  }
  busy.value = true;
  codeCopied.value = false;
  const currentRun = ++run;
  try {
    if (saveConfig) {
      const status = props.provider === "baidu"
        ? await api.saveBaiduCredentials(clientId.value.trim(), secret.value.trim())
        : props.provider === "google"
          ? await api.saveGoogleCredentials(clientId.value.trim(), secret.value.trim() || undefined)
          : await api.saveOneDriveCredentials(clientId.value.trim());
      if (disposed || currentRun !== run) return;
      emit("changed", status);
      secret.value = "";
      clientId.value = "";
      imported.value = false;
      setupOpen.value = false;
    }
    message.value = "正在打开官方登录页…";
    const next = await handlers.value.start();
    if (disposed || currentRun !== run) return;
    challenge.value = next;
    message.value = guide.value.hint;
    if (next.userCode) await copyCode();
    await openOfficial(next.verificationUrl);
    const expiresAt = Date.now() + next.expiresIn * 1000;
    while (!disposed && currentRun === run && Date.now() < expiresAt) {
      await new Promise<void>((resolve) => { wake = resolve; timer = setTimeout(resolve, Math.max(1, next.interval) * 1000); });
      wake = undefined;
      if (disposed || currentRun !== run) return;
      const result = await handlers.value.poll(next.deviceCode);
      if (disposed || currentRun !== run) return;
      if (result.authenticated) {
        emit("changed", { provider: props.provider, authenticated: true, available: true, expiresAt: result.expiresAt, message: "已绑定，下次打开无需重复登录" });
        message.value = "绑定成功！回到“发货任务”，拖入文件就能上传。";
        emit("toast", `${guide.value.name}已绑定`);
        return;
      }
      if (!result.pending) throw new Error("授权未完成");
    }
    if (currentRun === run) error.value = "这次登录已超时。再点一次绑定，用新页面或新验证码继续。";
  } catch (cause) {
    if (disposed || currentRun !== run) return;
    error.value = bindingError(props.provider, cause);
    // Do not expose raw provider responses or entered credentials in the page.
    technicalError.value = "请检查应用类型、测试用户和公共客户端设置。更多原因可在本机日志中查看。";
  } finally {
    if (currentRun === run) {
      busy.value = false;
      challenge.value = undefined;
    }
  }
}

async function unbind() {
  busy.value = true;
  try {
    emit("changed", await handlers.value.clear());
    confirmUnbind.value = false;
    message.value = "已解除本机绑定。应用配置已保留，再点绑定即可换账号。网盘里的文件不会删除。";
  } catch (cause) { error.value = bindingError(props.provider, cause); }
  finally { busy.value = false; }
}
</script>

<template>
  <section class="settings-card auth-card binding-card" :class="{ 'setup-expanded': setupOpen }" :aria-label="guide.name + '绑定'">
    <div class="card-title-row">
      <div><span class="provider-dot" :class="provider" /><strong>{{ guide.name }}</strong></div>
      <span class="status-chip" :class="{ online: status?.authenticated }">{{ status?.authenticated ? '已绑定' : '未绑定' }}</span>
    </div>
    <p v-if="status?.authenticated" class="binding-description">已经可以上传了，下次打开不用再登录。</p>
    <template v-else>
      <p class="binding-description">{{ guide.hint }}</p>
      <button class="primary-button bind-button" :disabled="busy" @click="bind()">{{ busy ? '等你在网页确认…' : `绑定${guide.name}` }}</button>
      <p class="binding-footnote">{{ status?.available ? '配置已保存，直接登录就行。' : '首次使用还需要应用配置，点绑定后会显示操作说明。' }}</p>
    </template>

    <div v-if="challenge" class="binding-wait" aria-live="polite">
      <template v-if="challenge.userCode">
        <p>把这个验证码填到刚打开的官方网页：</p>
        <strong class="device-code">{{ challenge.userCode }}</strong>
        <button class="small-button" @click="copyCode">{{ codeCopied ? '已复制，再复制一次' : '复制验证码' }}</button>
        <p>{{ codeCopied ? '已复制，去网页按 Ctrl+V 粘贴。' : '也可以选中验证码，按 Ctrl+C 复制。' }}</p>
      </template>
      <p v-else>在 Google 网页选择账号并同意，完成后这里会自动变成“已绑定”。</p>
      <div class="binding-actions">
        <button class="small-button" @click="openOfficial(challenge.verificationUrl)">重新打开登录页</button>
        <button class="text-button" @click="stopWaiting(); message = '已停止等待。网页上的登录也请关闭，需要时可重新绑定。'">暂时不绑</button>
      </div>
    </div>
    <p v-if="message" class="binding-message" role="status">{{ message }}</p>
    <div v-if="error" class="binding-error" role="alert">
      <p>{{ error }}</p>
      <details v-if="technicalError"><summary>排查信息</summary><p>{{ technicalError }}</p></details>
    </div>

    <template v-if="status?.authenticated">
      <button v-if="!confirmUnbind" class="text-button" :disabled="busy" @click="confirmUnbind = true">换一个账号</button>
      <div v-else class="binding-wait">
        <p>先解除本机绑定，再登录另一个账号。网盘里的文件不会删除；旧账号的上传任务请先暂停。</p>
        <button class="small-button" :disabled="busy" @click="unbind">解除绑定</button>
        <button class="text-button" @click="confirmUnbind = false">保留当前账号</button>
      </div>
    </template>

    <details v-else class="binding-setup" :open="setupOpen" @toggle="setupOpen = ($event.target as HTMLDetailsElement).open">
      <summary>{{ status?.available ? '首次设置已完成 · 需要时修改配置' : '首次设置 · 不知道怎么填就看这里' }}</summary>
      <p class="setup-explanation">这是“让软件获得登录入口”的设置，不是填写你的网盘密码。通常由给你软件的人准备；已经有配置就直接填，只有第一次需要。</p>
      <div class="binding-actions">
        <button class="small-button" @click="openOfficial(guide.setupUrl)">打开{{ guide.name }}官方设置页</button>
        <button class="text-button" @click="openOfficial(guide.helpUrl)">查看官方说明</button>
      </div>
      <div class="setup-steps">
        <div v-for="(step, index) in guide.steps" :key="step" class="setup-step"><span>{{ index + 1 }}</span><p>{{ step }}</p></div>
      </div>
      <fieldset :disabled="busy">
        <template v-if="provider === 'google'">
          <label class="config-import"><span>导入 Google 配置文件（推荐）</span><input type="file" accept=".json,application/json" @change="importGoogle" /><small>选择从 Google 下载的 JSON 文件，程序自动填写，不会上传这个文件。</small></label>
          <p v-if="imported" class="binding-message">配置已读好，直接点“保存并去登录”。</p>
          <details :open="manualOpen" @toggle="manualOpen = ($event.target as HTMLDetailsElement).open">
            <summary>没有文件，手动填写</summary>
            <label><span>应用编号（Client ID）</span><input v-model="clientId" autocomplete="off" placeholder="以 .apps.googleusercontent.com 结尾" /></label>
            <label><span>应用密钥（Client Secret，可选）</span><input v-model="secret" type="password" autocomplete="new-password" placeholder="官方配置里有这一项才填写" /></label>
          </details>
        </template>
        <template v-else>
          <label><span>{{ provider === 'baidu' ? '应用编号（App Key / API Key）' : '应用编号（应用程序 / 客户端 ID）' }}</span><input v-model="clientId" autocomplete="off" :placeholder="provider === 'baidu' ? '从百度应用详情复制' : '从微软应用概述复制，不是邮箱或密码'" /></label>
          <label v-if="provider === 'baidu'"><span>应用密钥（Secret Key）</span><input v-model="secret" type="password" autocomplete="new-password" placeholder="从百度应用详情复制，不是网盘密码" /></label>
        </template>
        <button class="small-button" @click="bind(true)">保存并去登录</button>
      </fieldset>
    </details>
    <p v-if="provider === 'baidu'" class="binding-footnote">百度当前支持上传；发货用的分享链接仍需到百度网盘里手动创建。</p>
  </section>
</template>
