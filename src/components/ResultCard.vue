<script setup lang="ts">
import type { UploadTask } from "../types";
import { formatDate } from "../utils/format";

defineProps<{ task: UploadTask }>();
const emit = defineEmits<{
  copy: [];
}>();
</script>

<template>
  <section
    v-if="task.baidu.share.url || task.baidu.share.notice"
    class="result-card"
  >
    <div class="result-grid">
      <div class="share-block">
        <div class="share-title">
          <strong>百度网盘</strong>
          <button class="copy-link" @click="emit('copy')">复制分享信息</button>
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

    </div>
  </section>
</template>
