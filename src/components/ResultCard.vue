<script setup lang="ts">
import type { UploadTask } from "../types";
import { formatDate } from "../utils/format";

defineProps<{ task: UploadTask }>();
const emit = defineEmits<{
  copy: [provider?: string];
}>();
</script>

<template>
  <section
    v-if="
      task.baidu.share.url ||
      task.quark.share.url ||
      task.baidu.share.notice ||
      task.quark.share.notice
    "
    class="result-card"
  >
    <div class="result-grid">
      <div class="share-block">
        <div class="share-title">
          <strong>百度网盘</strong>
          <button
            v-if="task.baidu.share.url || task.baidu.share.notice"
            class="copy-link"
            @click="emit('copy', 'baidu')"
          >
            复制百度
          </button>
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
          <strong>夸克网盘</strong>
          <button
            v-if="task.quark.share.url || task.quark.share.notice"
            class="copy-link"
            @click="emit('copy', 'quark')"
          >
            复制夸克
          </button>
        </div>
        <a
          v-if="task.quark.share.url"
          :href="task.quark.share.url"
          target="_blank"
        >
          {{ task.quark.share.url }}
        </a>
        <p v-else class="share-notice">{{ task.quark.share.notice }}</p>
        <div v-if="task.quark.share.url" class="share-meta">
          <span v-if="task.quark.share.code">提取码：{{ task.quark.share.code }}</span>
          <span>有效期：{{ formatDate(task.quark.share.expiresAt) }}</span>
        </div>
      </div>
    </div>
    <button
      v-if="task.baidu.share.url || task.quark.share.url"
      class="primary-button copy-all"
      @click="emit('copy')"
    >
      复制全部分享文案
    </button>
  </section>
</template>
