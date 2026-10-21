<script setup lang="ts">
import { reactive, ref, watch } from "vue";
import { openPath } from "@tauri-apps/plugin-opener";
import type { AppSettings, AuthStatus, CapabilityReport, Provider } from "../types";
import { api } from "../services/api";
import DriveBindingCard from "./DriveBindingCard.vue";

const props = defineProps<{ settings: AppSettings; auth: AuthStatus[]; capabilities: CapabilityReport[]; logDir: string; dataDir: string }>();
const emit = defineEmits<{ saved: [settings: AppSettings]; authChanged: [status: AuthStatus]; toast: [message: string]; ready: [] }>();
const form = reactive<AppSettings>({ ...props.settings });
const busy = ref(false);
const providers: Provider[] = ["baidu", "google", "onedrive"];
watch(() => props.settings, (value) => Object.assign(form, value));
async function save() {
  busy.value = true;
  try {
    emit("saved", await api.saveSettings({ ...form }));
    emit("toast", "设置已保存");
  } catch (error) { emit("toast", String(error)); }
  finally { busy.value = false; }
}
</script>

<template>
  <section class="page-panel settings-page">
    <div class="page-heading">
      <div>
        <span class="eyebrow">第一次用，从这里开始</span>
        <h2>绑定网盘</h2>
        <p>不用把网盘密码填进软件，只在网盘自己的网页登录。</p>
      </div>

    </div>

    <section class="binding-intro">
      <div><span class="eyebrow">先绑定，再拖文件</span><h3>登录你要用的网盘，照着网页点“同意”就行。</h3>
      <p>选一个网盘点“绑定” → 在官方网页登录并同意 → 回到这里看到“已绑定”。绑定后会记住，下次不用再来一遍。</p>
      <p class="binding-footnote">如果提示“首次设置”，说明这份软件还没有该网盘的应用配置。请给你软件的人提供配置，或展开下方中文教程自己设置一次；网盘密码只在官方网页输入。</p></div>
      <button v-if="auth.some(item => item.authenticated)" class="primary-button" @click="emit('ready')">绑定好了，去发货</button>
    </section>
    <div class="binding-grid">
      <DriveBindingCard v-for="provider in providers" :key="provider" :provider="provider" :status="auth.find(item => item.provider === provider)"
        @changed="emit('authChanged', $event)" @toast="emit('toast', $event)" />
    </div>
    <details class="preferences-panel">
      <summary>上传位置和其他设置（一般不用改）</summary>
      <div class="settings-grid">
      <section class="settings-card form-card">
        <h3>上传目录</h3>
        <label>
          <span>百度默认目录</span>
          <input v-model="form.baiduDefaultDir" />
          <small>必须位于开放平台分配的 /apps/应用目录/ 下</small>
        </label>
        <label>
          <span>Google Drive 默认目录</span>
          <input v-model="form.googleDefaultDir" />
          <small>程序会在 My Drive 中按层级查找或创建目录</small>
        </label>
        <label><span>OneDrive 默认目录</span><input v-model="form.onedriveDefaultDir" /><small>自动创建目录；文件名追加任务 ID，避免覆盖</small></label>
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
          可用变量：{filename}、{baidu_url}、{baidu_code_line}、{google_url}、{google_code_line}、{onedrive_url}、{onedrive_code_line}
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
      <button class="primary-button" :disabled="busy" @click="save">{{ busy ? '保存中…' : '保存设置' }}</button>
      </div>
    </details>

    <details class="capability-section">
      <summary>查看各网盘支持哪些功能</summary>

      <div class="capability-grid">
        <article v-for="item in capabilities" :key="item.provider">
          <strong>{{ item.provider === "baidu" ? "百度网盘" : item.provider === "google" ? "Google Drive" : "OneDrive" }}</strong>
          <p>{{ item.oauth }}</p>
          <p>{{ item.upload }}</p>
          <p>{{ item.resume }}</p>
          <p>{{ item.share }}</p>
          <div v-for="limit in item.limitations" :key="limit" class="limit-note">
            {{ limit }}
          </div>
        </article>
      </div>
    </details>
  </section>
</template>
