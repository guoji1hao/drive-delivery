<script setup lang="ts">
import type { UploadTask } from "../types";
import TaskCard from "./TaskCard.vue";

defineProps<{ tasks: UploadTask[] }>();
const emit = defineEmits<{
  pause: [taskId: string];
  retry: [taskId: string];
  cancel: [taskId: string];
  copy: [taskId: string];
}>();

function forwardPause(taskId: string) {
  emit("pause", taskId);
}

function forwardRetry(taskId: string) {
  emit("retry", taskId);
}

function forwardCancel(taskId: string) {
  emit("cancel", taskId);
}

function forwardCopy(taskId: string) {
  emit("copy", taskId);
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
