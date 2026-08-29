<script setup lang="ts">
import type { UploadTask } from "../types";
import { formatBytes, taskStateText } from "../utils/format";
import PlatformProgress from "./PlatformProgress.vue";
import ResultCard from "./ResultCard.vue";

defineProps<{ task: UploadTask }>();
const emit = defineEmits<{
  pause: [taskId: string, provider: string];
  retry: [taskId: string, provider: string];
  cancel: [taskId: string, provider: string];
  copy: [taskId: string, provider?: string];
}>();
</script>

<template>
  <article class="task-card">
    <header class="task-header">
      <div class="file-icon">{{ task.fileName.split(".").pop()?.slice(0, 4) || "FILE" }}</div>
      <div class="file-info">
        <h3 :title="task.fileName">{{ task.fileName }}</h3>
        <p>
          {{ formatBytes(task.fileSize) }}
          <span class="separator">·</span>
          <span :title="task.localPath">{{ task.localPath }}</span>
        </p>
      </div>
      <span class="task-state" :class="task.state">
        {{ taskStateText[task.state] }}
      </span>
    </header>

    <p v-if="task.message" class="task-message">{{ task.message }}</p>

    <div class="platform-grid">
      <PlatformProgress
        name="百度网盘"
        provider="baidu"
        :value="task.baidu"
        @pause="emit('pause', task.id, $event)"
        @retry="emit('retry', task.id, $event)"
        @cancel="emit('cancel', task.id, $event)"
      />
      <PlatformProgress
        name="夸克网盘"
        provider="quark"
        :value="task.quark"
        @pause="emit('pause', task.id, $event)"
        @retry="emit('retry', task.id, $event)"
        @cancel="emit('cancel', task.id, $event)"
      />
    </div>

    <ResultCard
      :task="task"
      @copy="emit('copy', task.id, $event)"
    />
  </article>
</template>
