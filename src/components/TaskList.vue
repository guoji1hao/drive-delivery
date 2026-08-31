<script setup lang="ts">
import type { Provider, UploadTask } from "../types";
import TaskCard from "./TaskCard.vue";

defineProps<{ tasks: UploadTask[] }>();
const emit = defineEmits<{
  pause: [taskId: string, provider: Provider];
  retry: [taskId: string, provider: Provider];
  cancel: [taskId: string, provider: Provider];
  copy: [taskId: string, provider: Provider | "all"];
}>();

function forwardPause(taskId: string, provider: Provider) {
  emit("pause", taskId, provider);
}

function forwardRetry(taskId: string, provider: Provider) {
  emit("retry", taskId, provider);
}

function forwardCancel(taskId: string, provider: Provider) {
  emit("cancel", taskId, provider);
}

function forwardCopy(taskId: string, provider: Provider | "all") {
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
