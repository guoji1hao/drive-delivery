<script setup lang="ts">
import { t } from "../i18n";
import type { Provider, UploadTask } from "../types";
import { formatDate } from "../utils/format";

defineProps<{ task: UploadTask }>();
const emit = defineEmits<{
  copy: [provider: Provider | "all"];
}>();
</script>

<template>
  <section
    v-if="task.baidu.share.url || task.baidu.share.notice || task.google.share.url || task.google.share.notice || task.onedrive.share.url || task.onedrive.share.notice"
    class="result-card"
  >
    <div class="result-grid">
      <div v-if="task.baidu.state !== 'skipped'" class="share-block">
        <div class="share-title">
          <strong>{{ t("百度网盘") }}</strong>
          <button
            class="copy-link"
            :disabled="!task.baidu.share.url && !task.baidu.share.notice"
            @click="emit('copy', 'baidu')"
          >{{ t("复制百度") }}</button>
        </div>
        <a
          v-if="task.baidu.share.url"
          :href="task.baidu.share.url"
          target="_blank"
        >
          {{ task.baidu.share.url }}
        </a>
        <p v-else class="share-notice">{{ t(task.baidu.share.notice) }}</p>
        <div v-if="task.baidu.share.url" class="share-meta">
          <span v-if="task.baidu.share.code">{{ t("提取码：") }}{{ task.baidu.share.code }}</span>
          <span>{{ t("有效期：") }}{{ t(formatDate(task.baidu.share.expiresAt)) }}</span>
        </div>
      </div>

      <div v-if="task.google.state !== 'skipped'" class="share-block">
        <div class="share-title">
          <strong>Google Drive</strong>
          <button
            class="copy-link"
            :disabled="!task.google.share.url && !task.google.share.notice"
            @click="emit('copy', 'google')"
          >{{ t("复制 Google") }}</button>
        </div>
        <a
          v-if="task.google.share.url"
          :href="task.google.share.url"
          target="_blank"
        >
          {{ task.google.share.url }}
        </a>
        <p v-else class="share-notice">{{ t(task.google.share.notice || "暂未生成分享链接") }}</p>
        <div v-if="task.google.share.url" class="share-meta">
          <span v-if="task.google.share.code">{{ t("提取码：") }}{{ task.google.share.code }}</span>
          <span>{{ t("有效期：") }}{{ t(formatDate(task.google.share.expiresAt)) }}</span>
        </div>
      </div>
      <div v-if="task.onedrive.state !== 'skipped'" class="share-block">
        <div class="share-title">
          <strong>OneDrive</strong>
          <button
            class="copy-link"
            :disabled="!task.onedrive.share.url && !task.onedrive.share.notice"
            @click="emit('copy', 'onedrive')"
          >{{ t("复制 OneDrive") }}</button>
        </div>
        <a
          v-if="task.onedrive.share.url"
          :href="task.onedrive.share.url"
          target="_blank"
        >
          {{ task.onedrive.share.url }}
        </a>
        <p v-else class="share-notice">{{ t(task.onedrive.share.notice || "暂未生成分享链接") }}</p>
        <div v-if="task.onedrive.share.url" class="share-meta">
          <span v-if="task.onedrive.share.code">{{ t("提取码：") }}{{ task.onedrive.share.code }}</span>
          <span>{{ t("有效期：") }}{{ t(task.onedrive.share.expiresAt ? formatDate(task.onedrive.share.expiresAt) : "以微软平台策略为准") }}</span>
        </div>
      </div>
    </div>
    <button class="primary-button copy-all-button" @click="emit('copy', 'all')">{{ t("复制全部") }}</button>
  </section>
</template>
