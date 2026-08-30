<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import type {
  AppSettings,
  AuthStatus,
  CapabilityReport,
} from "../types";
import { api } from "../services/api";

const props = defineProps<{
  settings: AppSettings;
  auth: AuthStatus[];
  capabilities: CapabilityReport[];
  logDir: string;
  dataDir: string;
}>();

const emit = defineEmits<{
  saved: [settings: AppSettings];
  authChanged: [status: AuthStatus];
  toast: [message: string];
}>();

const form = reactive<AppSettings>({ ...props.settings });
const appKey = ref("");
const secretKey = ref("");
const busy = ref<string>();
const authCode = ref<string>();

watch(
  () => props.settings,
  (value) => Object.assign(form, value),
);

const baiduStatus = computed(() =>
  props.auth.find((item) => item.provider === "baidu"),
);

async function save() {
  busy.value = "settings";
  try {
    const saved = await api.saveSettings({ ...form });
    emit("saved", saved);
    emit("toast", "设置已保存");
  } catch (error) {
    emit("toast", String(error));
  } finally {
    busy.value = undefined;
  }
}

async function authorizeBaidu() {
  if (!appKey.value.trim() || !secretKey.value.trim()) {
    emit("toast", "请先填写百度 App Key 和 Secret Key");
    return;
  }
  busy.value = "baidu";
  authCode.value = undefined;
  try {
    const configured = await api.saveBaiduCredentials(
      appKey.value,
      secretKey.value,
    );
    emit("authChanged", configured);
    const challenge = await api.startBaiduAuth();
    authCode.value = challenge.userCode;
    await openUrl(challenge.verificationUrl);
    const startedAt = Date.now();
    const expiresAt = startedAt + challenge.expiresIn * 1000;
    while (Date.now() < expiresAt) {
      await new Promise((resolve) =>
        window.setTimeout(resolve, challenge.interval * 1000),
      );
      const result = await api.pollBaiduAuth(challenge.deviceCode);
      if (result.authenticated) {
        emit("authChanged", {
          provider: "baidu",
          authenticated: true,
          available: true,
          expiresAt: result.expiresAt,
          message: result.message,
        });
        emit("toast", result.message);
        authCode.value = undefined;
        return;
      }
    }
    emit("toast", "百度授权已超时，请重新发起授权");
  } catch (error) {
    emit("toast", String(error));
  } finally {
    busy.value = undefined;
  }
}

async function clearBaidu() {
  try {
    const status = await api.clearBaiduAuth();
    emit("authChanged", status);
    emit("toast", "已清除本机百度授权，可重新绑定");
  } catch (error) {
    emit("toast", String(error));
  }
}

</script>

<template>
  <section class="page-panel settings-page">
    <div class="page-heading">
      <div>
        <span class="eyebrow">安全与偏好</span>
        <h2>设置</h2>
        <p>Token 不写入数据库；百度敏感凭据保存在 Windows 凭据管理器。</p>
      </div>
      <button class="primary-button" :disabled="busy === 'settings'" @click="save">
        {{ busy === "settings" ? "保存中…" : "保存设置" }}
      </button>
    </div>

    <div class="settings-grid">
      <section class="settings-card auth-card">
        <div class="card-title-row">
          <div>
            <span class="provider-dot baidu" />
            <strong>百度网盘授权</strong>
          </div>
          <span class="status-chip" :class="{ online: baiduStatus?.authenticated }">
            {{ baiduStatus?.authenticated ? "已授权" : "未授权" }}
          </span>
        </div>
        <p>{{ baiduStatus?.message }}</p>
        <template v-if="!baiduStatus?.authenticated">
          <label>
            <span>App Key</span>
            <input v-model="appKey" autocomplete="off" placeholder="开放平台 App Key" />
          </label>
          <label>
            <span>Secret Key</span>
            <input
              v-model="secretKey"
              type="password"
              autocomplete="new-password"
              placeholder="仅写入系统安全存储"
            />
          </label>
          <p v-if="authCode" class="auth-code">
            授权页设备码：<strong>{{ authCode }}</strong>
          </p>
          <button class="small-button" :disabled="busy === 'baidu'" @click="authorizeBaidu">
            {{ busy === "baidu" ? "等待浏览器授权…" : "绑定百度网盘" }}
          </button>
        </template>
        <button v-else class="text-button danger-text" @click="clearBaidu">
          清除授权并重新绑定
        </button>
      </section>

      <section class="settings-card form-card">
        <h3>上传目录</h3>
        <label>
          <span>百度默认目录</span>
          <input v-model="form.baiduDefaultDir" />
          <small>必须位于开放平台分配的 /apps/应用目录/ 下</small>
        </label>
        <label class="check-row">
          <input v-model="form.datedSubdirectory" type="checkbox" />
          <span>按日期创建子目录，例如 2026-08-29</span>
        </label>
      </section>

      <section class="settings-card form-card">
        <h3>任务策略</h3>
        <div class="two-column-fields">
          <label>
            <span>同时处理文件数</span>
            <input v-model.number="form.maxConcurrentFiles" type="number" min="1" max="8" />
          </label>
          <label>
            <span>失败自动重试</span>
            <input v-model.number="form.retryLimit" type="number" min="0" max="8" />
          </label>
        </div>
        <label>
          <span>分享有效期</span>
          <select v-model.number="form.sharePeriodDays">
            <option :value="0">永久或官方最长有效期</option>
            <option :value="7">7 天</option>
            <option :value="30">30 天</option>
            <option :value="180">180 天</option>
          </select>
        </label>
        <label class="check-row">
          <input v-model="form.autoCopy" type="checkbox" />
          <span>处理完成后自动复制可用分享结果</span>
        </label>
      </section>

      <section class="settings-card form-card full-span">
        <h3>复制文本模板</h3>
        <textarea v-model="form.copyTemplate" rows="7" />
        <p class="field-help">
          可用变量：{filename}、{baidu_url}、{baidu_code_line}
        </p>
      </section>

      <section class="settings-card full-span storage-card">
        <div>
          <h3>本地数据与日志</h3>
          <p>数据库：{{ dataDir }}</p>
          <p>日志：{{ logDir }}</p>
        </div>
        <button class="small-button" @click="openPath(logDir)">打开日志目录</button>
      </section>
    </div>

    <section class="capability-section">
      <span class="eyebrow">官方能力边界</span>
      <h3>当前 Adapter 实现依据</h3>
      <div class="capability-grid">
        <article v-for="item in capabilities" :key="item.provider">
          <strong>百度网盘</strong>
          <p>{{ item.oauth }}</p>
          <p>{{ item.upload }}</p>
          <p>{{ item.resume }}</p>
          <p>{{ item.share }}</p>
          <div v-for="limit in item.limitations" :key="limit" class="limit-note">
            {{ limit }}
          </div>
        </article>
      </div>
    </section>
  </section>
</template>
