<script setup lang="ts">
import type { UploadTask } from "../types";
import TaskCard from "./TaskCard.vue";

defineProps<{ tasks: UploadTask[] }>();
const emit = defineEmits<{
  pause: [taskId: string, provider: string];
  retry: [taskId: string, provider: string];
  cancel: [taskId: string, provider: string];
  copy: [taskId: string, provider?: string];
}>();

function forwardPause(taskId: string, provider: string) {
  emit("pause", taskId, provider);
}

function forwardRetry(taskId: string, provider: string) {
  emit("retry", taskId, provider);
}

function forwardCancel(taskId: string, provider: string) {
  emit("cancel", taskId, provider);
}

function forwardCopy(taskId: string, provider?: string) {
  emit("copy", taskId, provider);
}
</script>

<template>
  <section v-if="tasks.length" class="task-section">
    <div class="section-heading">
      <div>
        <span class="eyebrow">实时任务</span>
        <h2>处理队列</h2>
      </div>
      <span class="count-pill">{{ tasks.length }} 个文件</span>
    </div>
    <div class="task-list">
      <TaskCard
        v-for="task in tasks"
        :key="task.id"
        :task="task"
        @pause="forwardPause"
        @retry="forwardRetry"
        @cancel="forwardCancel"
        @copy="forwardCopy"
      />
    </div>
  </section>
</template>
