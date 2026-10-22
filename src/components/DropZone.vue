<script setup lang="ts">
import { t } from "../i18n";
defineProps<{ active: boolean; busy: boolean }>();
const emit = defineEmits<{ chooseFiles: []; chooseFolder: [] }>();
</script>

<template>
  <section class="drop-zone" :class="{ active, busy }" :aria-busy="busy">
    <button class="drop-mark" :disabled="busy" :aria-label="t('选择文件上传')" :title="t('选择文件上传')" @click="emit('chooseFiles')"><span>＋</span></button>
    <div>
      <h2>{{ t(busy ? '正在读取文件…' : '点 + 选文件，也可以直接拖进来') }}</h2>
      <p>{{ t('整个文件夹也能上传，里面的子目录和空文件夹也会保留。') }}</p>
      <div class="upload-actions">
        <button class="small-button" :disabled="busy" @click="emit('chooseFiles')">{{ t('选择文件') }}</button>
        <button class="small-button" :disabled="busy" @click="emit('chooseFolder')">{{ t('上传整个文件夹') }}</button>
      </div>
    </div>
  </section>
</template>
