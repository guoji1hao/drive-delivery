<script setup lang="ts">
import { ref } from "vue";
import { api } from "../services/api";
import type { DrivePlugin, Provider } from "../types";
import { driveGuides } from "../utils/driveBinding";
import { t } from "../i18n";
defineProps<{ plugins: DrivePlugin[] }>();
const emit = defineEmits<{ changed: [plugins: DrivePlugin[]]; toast: [message: string] }>();
const busy = ref(false);
async function change(id: Provider, action: "install" | "enable" | "disable" | "remove", manifest?: string) {
  if (busy.value) return;
  busy.value = true;
  try { emit("changed", await api.updatePlugin(id, action, manifest)); emit("toast", "插件设置已保存"); }
  catch (error) { emit("toast", String(error)); }
  finally { busy.value = false; }
}
async function importPlugin(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  try {
    if (file.size > 4096) throw new Error("插件文件不能超过 4 KB");
    const json = await file.text();
    const value = JSON.parse(json);
    if (!["baidu", "google", "onedrive"].includes(value?.id)) throw new Error("请选择本软件支持的网盘插件文件");
    await change(value.id, "install", json);
  } catch { emit("toast", "插件导入失败，请选择支持的 .drive-plugin.json 文件（不超过 4 KB）"); }
  finally { input.value = ""; }
}
</script>

<template>
  <section class="plugin-manager preferences-panel">
    <div class="card-title-row"><div><span class="eyebrow">{{ t("按需开启") }}</span><h3>{{ t("网盘插件") }}</h3></div>
      <label class="small-button plugin-import">{{ t("导入插件") }}<input type="file" accept=".json,application/json" :disabled="busy" @change="importPlugin" /></label>
    </div>
    <p>{{ t("打开你要用的网盘，再到下面登录。以后拖入文件，只发到已启用的网盘。") }}</p>
    <div class="plugin-grid">
      <article v-for="item in plugins" :key="item.manifest.id" class="plugin-card" :data-plugin="item.manifest.id">
        <strong>{{ t(driveGuides[item.manifest.id].name) }}</strong><small>v{{ item.manifest.version }}</small>
        <span class="status-chip" :class="{ online: item.enabled }">{{ t(!item.installed ? '未安装' : item.enabled ? '已启用' : '已停用') }}</span>
        <div class="binding-actions">
          <button v-if="!item.installed" class="small-button" :disabled="busy" @click="change(item.manifest.id, 'install')">{{ t("安装并启用") }}</button>
          <template v-else>
            <button class="small-button" :disabled="busy" @click="change(item.manifest.id, item.enabled ? 'disable' : 'enable')">{{ t(item.enabled ? '停用' : '启用') }}</button>
            <button class="text-button" :disabled="busy" @click="change(item.manifest.id, 'remove')">{{ t("移除插件") }}</button>
          </template>
        </div>
      </article>
    </div>
    <p class="binding-footnote">{{ t("停用或移除不删除网盘文件、历史记录或登录配置。已经开始的上传会继续；排队中的任务和手动重试按当前开关执行。") }}</p>
    <details><summary>{{ t("插件文件是什么？") }}</summary><p>{{ t("插件文件用于安装本软件内置的官方网盘适配器，不包含运行代码。百度、Google Drive、OneDrive 已随软件提供；接入其他网盘需要更新软件适配器。") }}</p></details>
  </section>
</template>
