<script setup lang="ts">
import { computed } from "vue";
import type { PlatformTask, Provider } from "../types";
import {
  canRetry,
  formatBytes,
  isBusy,
  platformStateText,
} from "../utils/format";

const props = defineProps<{
  name: string;
  provider: Provider;
  value: PlatformTask;
}>();

const emit = defineEmits<{
  pause: [];
  retry: [];
  cancel: [];
}>();

const tone = computed(() => {
  if (props.value.state === "completed") return "success";
  if (["retryable", "failed", "unavailable"].includes(props.value.state)) {
    return "danger";
  }
  if (props.value.state === "paused") return "warning";
  return "neutral";
});
</script>

<template>
  <article class="platform-panel" :class="tone">
    <header>
      <div class="provider-name">
        <span class="provider-dot" :class="provider" />
        <strong>{{ name }}</strong>
      </div>
      <span class="state-label">{{ platformStateText[value.state] }}</span>
    </header>

    <div class="progress-track" aria-label="上传进度">
      <div class="progress-value" :style="{ width: `${value.progress}%` }" />
    </div>
    <div class="progress-meta">
      <span>{{ value.progress.toFixed(value.progress < 10 ? 1 : 0) }}%</span>
      <span v-if="value.totalBytes">
        {{ formatBytes(value.uploadedBytes) }} / {{ formatBytes(value.totalBytes) }}
      </span>
    </div>

    <p v-if="value.remotePath" class="remote-path" :title="value.remotePath">
      {{ value.remotePath }}
    </p>
    <p v-if="value.error" class="platform-error">{{ value.error }}</p>

    <footer v-if="isBusy(value.state) || canRetry(value.state)">
      <button
        v-if="isBusy(value.state)"
        class="text-button"
        @click="emit('pause')"
      >
        暂停
      </button>
      <button
        v-if="isBusy(value.state)"
        class="text-button danger-text"
        @click="emit('cancel')"
      >
        取消
      </button>
      <button
        v-if="canRetry(value.state)"
        class="small-button"
        @click="emit('retry')"
      >
        {{ value.state === "paused" ? "继续" : "重试" }}
      </button>
    </footer>
  </article>
</template>
