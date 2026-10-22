<script setup lang="ts">
import { t } from "../i18n";
import type { Provider, UploadTask } from "../types";
import { formatBytes, taskStateText } from "../utils/format";
import PlatformProgress from "./PlatformProgress.vue";
import ResultCard from "./ResultCard.vue";

defineProps<{ task: UploadTask }>();
const emit = defineEmits<{
  pause: [taskId: string, provider: Provider];
  retry: [taskId: string, provider: Provider];
  cancel: [taskId: string, provider: Provider];
  copy: [taskId: string, provider: Provider | "all"];
}>();
</script>

<template>
  <article class="task-card">
    <header class="task-header">
      <div class="file-icon">{{ task.fileName.split(".").pop()?.slice(0, 4) || "FILE" }}</div>
      <div class="file-info">
        <h3 :title="task.fileName">{{ task.fileName }}</h3>
        <p>
          {{ task.isDirectory ? t("文件夹") : formatBytes(task.fileSize) }}
          <span class="separator">·</span>
          <span :title="task.localPath">{{ task.localPath }}</span>
        </p>
      </div>
      <span class="task-state" :class="task.state">
        {{ t(taskStateText[task.state]) }}
      </span>
    </header>

    <p v-if="task.relativeDir" class="task-message">{{ t("保留目录") }}：{{ task.relativeDir }}</p>
    <p v-if="task.message" class="task-message">{{ t(task.message) }}</p>

    <div class="platform-grid">
      <PlatformProgress
        :name="t('百度网盘')"
        provider="baidu"
        :value="task.baidu"
        @pause="emit('pause', task.id, 'baidu')"
        @retry="emit('retry', task.id, 'baidu')"
        @cancel="emit('cancel', task.id, 'baidu')"
      />
      <PlatformProgress
        name="Google Drive"
        provider="google"
        :value="task.google"
        @pause="emit('pause', task.id, 'google')"
        @retry="emit('retry', task.id, 'google')"
        @cancel="emit('cancel', task.id, 'google')"
      />
      <PlatformProgress
        name="OneDrive"
        provider="onedrive"
        :value="task.onedrive"
        @pause="emit('pause', task.id, 'onedrive')"
        @retry="emit('retry', task.id, 'onedrive')"
        @cancel="emit('cancel', task.id, 'onedrive')"
      />
    </div>

    <ResultCard
      :task="task"
      @copy="(provider) => emit('copy', task.id, provider)"
    />
  </article>
</template>
