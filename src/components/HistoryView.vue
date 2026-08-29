<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import type { HistoryRecord } from "../types";
import { api } from "../services/api";
import { formatBytes, formatDate, taskStateText } from "../utils/format";

const emit = defineEmits<{ toast: [message: string] }>();
const query = ref("");
const records = ref<HistoryRecord[]>([]);
const loading = ref(false);
let debounceTimer: number | undefined;

async function search() {
  loading.value = true;
  try {
    records.value = await api.searchHistory(query.value.trim());
  } catch (error) {
    emit("toast", String(error));
  } finally {
    loading.value = false;
  }
}

async function copy(record: HistoryRecord) {
  try {
    await api.copyHistoryResult(record);
    emit("toast", "历史分享文案已复制");
  } catch (error) {
    emit("toast", String(error));
  }
}

watch(query, () => {
  window.clearTimeout(debounceTimer);
  debounceTimer = window.setTimeout(search, 220);
});

onMounted(search);
</script>

<template>
  <section class="page-panel history-page">
    <div class="page-heading">
      <div>
        <span class="eyebrow">本地 SQLite</span>
        <h2>历史记录</h2>
        <p>分享结果保存在本机，再次复制不需要重新上传。</p>
      </div>
      <label class="search-box">
        <span>⌕</span>
        <input v-model="query" placeholder="按文件名搜索" />
      </label>
    </div>

    <div v-if="loading" class="empty-state">正在读取历史记录…</div>
    <div v-else-if="!records.length" class="empty-state">
      <strong>还没有匹配的历史记录</strong>
      <p>完成一次上传后，文件与分享信息会出现在这里。</p>
    </div>
    <div v-else class="history-table-wrap">
      <table class="history-table">
        <thead>
          <tr>
            <th>文件</th>
            <th>上传时间</th>
            <th>百度</th>
            <th>夸克</th>
            <th>状态</th>
            <th />
          </tr>
        </thead>
        <tbody>
          <tr v-for="record in records" :key="record.id">
            <td>
              <strong>{{ record.fileName }}</strong>
              <span>{{ formatBytes(record.fileSize) }}</span>
            </td>
            <td>{{ formatDate(record.uploadedAt) }}</td>
            <td>
              <a v-if="record.baiduUrl" :href="record.baiduUrl" target="_blank">已生成</a>
              <span v-else>无链接</span>
            </td>
            <td>
              <a v-if="record.quarkUrl" :href="record.quarkUrl" target="_blank">已生成</a>
              <span v-else>无链接</span>
            </td>
            <td>{{ taskStateText[record.taskStatus] }}</td>
            <td>
              <button
                class="small-button"
                :disabled="!record.baiduUrl && !record.quarkUrl"
                @click="copy(record)"
              >
                复制分享
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
