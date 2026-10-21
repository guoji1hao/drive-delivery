<script setup lang="ts">
import { t, tf, locale } from "./i18n";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
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
  Provider,
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
        ? tf("已接收 {count} 个文件，其中 {duplicates} 个命中历史记录", "Received {count} files; {duplicates} found in history", { count: result.accepted, duplicates: result.duplicates })
        : tf("已接收 {count} 个文件，开始并行处理", "Received {count} files. Processing now", { count: result.accepted }),
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
  provider: Provider,
) {
  try {
    const handler = {
      pause: api.pauseTask,
      retry: api.retryTask,
      cancel: api.cancelTask,
    }[action];
    const task = await handler(taskId, provider);
    updateTask(task);
    showToast(
      action === "pause"
        ? "该平台任务已暂停"
        : action === "cancel"
          ? "该平台任务已取消"
          : tf("已重新执行 {drive} 任务", "Restarted {drive} task", { drive: provider }),
    );
  } catch (error) {
    showToast(String(error));
  }
}

async function copy(taskId: string, provider: Provider | "all") {
  try {
    await api.copyTaskResult(taskId, provider);
    showToast(provider === "all" ? "全部分享信息已复制" : "平台分享信息已复制");
  } catch (error) {
    showToast(String(error));
  }
}

function pauseTask(taskId: string, provider: Provider) {
  return control("pause", taskId, provider);
}

function retryTask(taskId: string, provider: Provider) {
  return control("retry", taskId, provider);
}

function cancelTask(taskId: string, provider: Provider) {
  return control("cancel", taskId, provider);
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

watch(locale, () => { if (snapshot.value) api.saveLanguage().catch(error => showToast(String(error))); });

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
    await api.saveLanguage().catch(error => showToast(String(error)));
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
          <strong>{{ t("双盘发货助手") }}</strong>
          <small>BAIDU + GOOGLE + ONEDRIVE</small>
        </span>
      </button>
      <nav>
        <button :class="{ active: activeTab === 'tasks' }" @click="activeTab = 'tasks'">
          {{ t("发货任务") }}
        </button>
        <button :class="{ active: activeTab === 'history' }" @click="activeTab = 'history'">
          {{ t("历史记录") }}
        </button>
        <button :class="{ active: activeTab === 'settings' }" @click="activeTab = 'settings'">
          {{ t("绑定网盘") }}
        </button>
      </nav>
      <div class="header-tools">
      <select v-model="locale" class="language-select" aria-label="Language / 语言"><option value="zh-CN">简体中文</option><option value="en">English</option></select>
      <div class="header-status" :title="t('数据仅保存在本机')">
        <span /> {{ t("本地安全运行") }}
      </div>
      </div>
    </header>

    <section v-if="loading" class="boot-screen">
      <div class="spinner" />
      <p>{{ t("正在打开本地数据库并恢复任务…") }}</p>
    </section>

    <section v-else-if="fatalError" class="boot-screen error-screen">
      <strong>{{ t("应用初始化失败") }}</strong>
      <p>{{ t(fatalError) }}</p>
    </section>

    <template v-else-if="snapshot">
      <div v-show="activeTab === 'tasks'" class="tasks-page">
        <section class="hero-copy">
          <span class="eyebrow">{{ t("一次拖入 · 自动上传") }}</span>
          <h1>{{ t("一次拖入，送达已启用的网盘。") }}</h1>
          <p>{{ t("百度网盘、Google Drive 与 OneDrive 并行上传，支持真实进度、断点续传、独立重试和历史去重。") }}</p>
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
            <strong>{{ t("第一次用？先登录你要用的网盘") }}</strong>
            <p>{{ t("点“绑定网盘”，按提示登录并同意。绑定好了再拖文件，之后不用重复登录。") }}</p>
          </div>
          <button class="small-button" @click="activeTab = 'settings'">{{ t("去绑定网盘") }}</button>
        </section>
      </div>

      <HistoryView v-if="activeTab === 'history'" @toast="showToast" />

      <SettingsView
        v-show="activeTab === 'settings'"
        :plugins="snapshot.plugins"
        @plugins-changed="snapshot.plugins = $event"
        :settings="snapshot.settings"
        :auth="snapshot.auth"
        :capabilities="snapshot.capabilities"
        :log-dir="snapshot.logDir"
        :data-dir="snapshot.dataDir"
        @ready="activeTab = 'tasks'"
        @saved="replaceSettings"
        @auth-changed="replaceAuth"
        @toast="showToast"
      />
    </template>

    <Transition name="toast">
      <div v-if="toast" class="toast-message">{{ t(toast) }}</div>
    </Transition>
  </main>
</template>
