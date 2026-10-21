<script setup lang="ts">
import { t } from "../i18n";
defineProps<{
  active: boolean;
  busy: boolean;
}>();
</script>

<template>
  <section class="drop-zone" :class="{ active, busy }">
    <div class="drop-mark" aria-hidden="true">
      <span>＋</span>
    </div>
    <div>
      <h2>{{ t(busy ? "正在读取文件…" : "拖入文件或文件夹") }}</h2>
      <p>{{ t("松开后自动校验并上传已启用的网盘，支持多文件和完整文件夹") }}</p>
    </div>
    <div class="drop-hints">
      <span>{{ t("流式分片") }}</span>
      <span>{{ t("断点续传") }}</span>
      <span>{{ t("历史去重") }}</span>
    </div>
  </section>
</template>
