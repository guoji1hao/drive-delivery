<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";
import DropZone from "./components/DropZone.vue";
import HistoryView from "./components/HistoryView.vue";
import SettingsView from "./components/SettingsView.vue";
import TaskList from "./components/TaskList.vue";
import { api } from "./services/api";
import type {
  AppSettings,
  AppSnapshot,
  AuthStatus,
  UploadTask,
} from "./types";

type Tab = "tasks" | "history" | "settings";

const snapshot = ref<AppSnapshot>();
const activeTab = ref<Tab>("tasks");
const dragActive = ref(false);
const enqueueBusy = ref(false);
const loading = ref(true);
const fatalError = ref<string>();
const toast = ref<string>();
let toastTimer: number | undefined;
let unlistenTask: UnlistenFn | undefined;
let unlistenDrop: UnlistenFn | undefined;

const tasks = computed(() =>
  [...(snapshot.value?.tasks ?? [])].sort((a, b) => b.updatedAt - a.updatedAt),
);

function showToast(message: string) {
  toast.value = message.replace(/^Error:\s*/, "");
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => (toast.value = undefined), 4200);
}

function updateTask(task: UploadTask) {
  if (!snapshot.value) return;
  const index = snapshot.value.tasks.findIndex((item) => item.id === task.id);
  if (index >= 0) snapshot.value.tasks[index] = task;
  else snapshot.value.tasks.unshift(task);
}

async function enqueue(paths: string[]) {
  if (!paths.length || enqueueBusy.value) return;
  enqueueBusy.value = true;
  activeTab.value = "tasks";
  try {
    const result = await api.enqueue(paths);
    result.tasks.forEach(updateTask);
    showToast(
      result.duplicates
        ? `已接收 ${result.accepted} 个文件，其中 ${result.duplicates} 个命中历史记录`
        : `已接收 ${result.accepted} 个文件，开始并行处理`,
    );
  } catch (error) {
    showToast(String(error));
  } finally {
    enqueueBusy.value = false;
    dragActive.value = false;
  }
}

async function control(
  action: "pause" | "retry" | "cancel",
  taskId: string,
) {
  try {
    const task = await api[`${action}Task`](taskId);
    updateTask(task);
    showToast(
      action === "pause" ? "任务已暂停" : action === "cancel" ? "任务已取消" : "已重新执行百度任务",
    );
  } catch (error) {
    showToast(String(error));
  }
}

async function copy(taskId: string) {
  try {
    await api.copyTaskResult(taskId);
    showToast("百度分享信息已复制");
  } catch (error) {
    showToast(String(error));
  }
}

function pauseTask(taskId: string) {
  return control("pause", taskId);
}

function retryTask(taskId: string) {
  return control("retry", taskId);
}

function cancelTask(taskId: string) {
  return control("cancel", taskId);
}

function replaceAuth(status: AuthStatus) {
  if (!snapshot.value) return;
  const index = snapshot.value.auth.findIndex(
    (item) => item.provider === status.provider,
  );
  if (index >= 0) snapshot.value.auth[index] = status;
  else snapshot.value.auth.push(status);
}

function replaceSettings(settings: AppSettings) {
  if (snapshot.value) snapshot.value.settings = settings;
}

onMounted(async () => {
  try {
    unlistenTask = await listen<UploadTask>("task-update", (event) =>
      updateTask(event.payload),
    );
    unlistenDrop = await getCurrentWebviewWindow().onDragDropEvent((event) => {
      if (event.payload.type === "over" || event.payload.type === "enter") {
        dragActive.value = true;
      } else if (event.payload.type === "leave") {
        dragActive.value = false;
      } else if (event.payload.type === "drop") {
        enqueue(event.payload.paths);
      }
    });
    snapshot.value = await api.initialize();
  } catch (error) {
    fatalError.value = String(error);
  } finally {
    loading.value = false;
  }
});

onBeforeUnmount(() => {
  unlistenTask?.();
  unlistenDrop?.();
  window.clearTimeout(toastTimer);
});
</script>

<template>
  <main class="app-shell">
    <header class="app-header">
      <button class="brand" @click="activeTab = 'tasks'">
        <span class="brand-mark"><i /><i /></span>
        <span>
          <strong>双盘发货助手</strong>
          <small>BAIDU DRIVE DELIVERY</small>
        </span>
      </button>
      <nav>
        <button :class="{ active: activeTab === 'tasks' }" @click="activeTab = 'tasks'">
          发货任务
        </button>
        <button :class="{ active: activeTab === 'history' }" @click="activeTab = 'history'">
          历史记录
        </button>
        <button :class="{ active: activeTab === 'settings' }" @click="activeTab = 'settings'">
          设置
        </button>
      </nav>
      <div class="header-status" title="数据仅保存在本机">
        <span /> 本地安全运行
      </div>
    </header>

    <section v-if="loading" class="boot-screen">
      <div class="spinner" />
      <p>正在打开本地数据库并恢复任务…</p>
    </section>

    <section v-else-if="fatalError" class="boot-screen error-screen">
      <strong>应用初始化失败</strong>
      <p>{{ fatalError }}</p>
    </section>

    <template v-else-if="snapshot">
      <div v-show="activeTab === 'tasks'" class="tasks-page">
        <section class="hero-copy">
          <span class="eyebrow">一次拖入 · 自动上传</span>
          <h1>把交付资料，稳稳送到百度网盘。</h1>
          <p>支持大文件分片、真实进度、断点续传、失败重试和本地历史去重。</p>
        </section>
        <DropZone :active="dragActive" :busy="enqueueBusy" />
        <TaskList
          :tasks="tasks"
          @pause="pauseTask"
          @retry="retryTask"
          @cancel="cancelTask"
          @copy="copy"
        />
        <section v-if="!tasks.length" class="first-run-note">
          <div>
            <strong>开始前请先完成平台授权</strong>
            <p>百度使用官方 OAuth。未授权任务会保留为可重试状态，不会伪造上传或分享结果。</p>
          </div>
          <button class="small-button" @click="activeTab = 'settings'">前往设置</button>
        </section>
      </div>

      <HistoryView v-if="activeTab === 'history'" @toast="showToast" />

      <SettingsView
        v-if="activeTab === 'settings'"
        :settings="snapshot.settings"
        :auth="snapshot.auth"
        :capabilities="snapshot.capabilities"
        :log-dir="snapshot.logDir"
        :data-dir="snapshot.dataDir"
        @saved="replaceSettings"
        @auth-changed="replaceAuth"
        @toast="showToast"
      />
    </template>

    <Transition name="toast">
      <div v-if="toast" class="toast-message">{{ toast }}</div>
    </Transition>
  </main>
</template>
