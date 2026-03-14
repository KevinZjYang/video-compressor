<script setup lang="ts">
import { onMounted } from "vue";
import { useAppStore } from "./stores/app";

const store = useAppStore();

onMounted(() => {
  store.initFfmpeg();
});
</script>

<template>
  <div class="app-container">
    <router-view />
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body, #app {
  height: 100%;
  width: 100%;
  overflow: hidden;
}

body {
  font-family: "Microsoft YaHei", "Segoe UI", sans-serif;
  font-size: 14px;
  line-height: 1.5;
  color: #333;
  background-color: #f5f7fa;
}

.app-container {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
}

/* Element Plus 覆盖样式 */
.el-button--primary {
  --el-button-bg-color: #409eff;
  --el-button-border-color: #409eff;
}

.el-menu {
  border-bottom: none !important;
}
</style>
