<script setup lang="ts">
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
      <div class="share-block">
        <div class="share-title">
          <strong>百度网盘</strong>
          <button
            class="copy-link"
            :disabled="!task.baidu.share.url && !task.baidu.share.notice"
            @click="emit('copy', 'baidu')"
          >复制百度</button>
        </div>
        <a
          v-if="task.baidu.share.url"
          :href="task.baidu.share.url"
          target="_blank"
        >
          {{ task.baidu.share.url }}
        </a>
        <p v-else class="share-notice">{{ task.baidu.share.notice }}</p>
        <div v-if="task.baidu.share.url" class="share-meta">
          <span v-if="task.baidu.share.code">提取码：{{ task.baidu.share.code }}</span>
          <span>有效期：{{ formatDate(task.baidu.share.expiresAt) }}</span>
        </div>
      </div>

      <div class="share-block">
        <div class="share-title">
          <strong>Google Drive</strong>
          <button
            class="copy-link"
            :disabled="!task.google.share.url && !task.google.share.notice"
            @click="emit('copy', 'google')"
          >复制 Google</button>
        </div>
        <a
          v-if="task.google.share.url"
          :href="task.google.share.url"
          target="_blank"
        >
          {{ task.google.share.url }}
        </a>
        <p v-else class="share-notice">{{ task.google.share.notice || "暂未生成分享链接" }}</p>
        <div v-if="task.google.share.url" class="share-meta">
          <span v-if="task.google.share.code">提取码：{{ task.google.share.code }}</span>
          <span>有效期：{{ formatDate(task.google.share.expiresAt) }}</span>
        </div>
      </div>
      <div class="share-block">
        <div class="share-title">
          <strong>OneDrive</strong>
          <button
            class="copy-link"
            :disabled="!task.onedrive.share.url && !task.onedrive.share.notice"
            @click="emit('copy', 'onedrive')"
          >复制 OneDrive</button>
        </div>
        <a
          v-if="task.onedrive.share.url"
          :href="task.onedrive.share.url"
          target="_blank"
        >
          {{ task.onedrive.share.url }}
        </a>
        <p v-else class="share-notice">{{ task.onedrive.share.notice || "暂未生成分享链接" }}</p>
        <div v-if="task.onedrive.share.url" class="share-meta">
          <span v-if="task.onedrive.share.code">提取码：{{ task.onedrive.share.code }}</span>
          <span>有效期：{{ task.onedrive.share.expiresAt ? formatDate(task.onedrive.share.expiresAt) : "以微软平台策略为准" }}</span>
        </div>
      </div>
    </div>
    <button class="primary-button copy-all-button" @click="emit('copy', 'all')">复制全部</button>
  </section>
</template>
