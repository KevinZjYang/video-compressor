<script setup lang="ts">
import { onMounted } from "vue";
import { useAppStore } from "./stores/app";
import { open } from "@tauri-apps/plugin-shell";

const store = useAppStore();

onMounted(() => {
  store.initFfmpeg();
});

const openWebsite = async () => {
  await open("https://apphub.yiun.top/");
};
</script>

<template>
  <div class="app-container">
    <router-view />
    <div class="footer">
      <span class="brand">视频压缩器-K</span>
      <span class="divider">|</span>
      <a class="link" @click="openWebsite">官网: https://apphub.yiun.top/</a>
    </div>
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

.footer {
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #fff;
  border-top: 1px solid #e4e7ed;
  font-size: 12px;
  color: #909399;
  flex-shrink: 0;
}

.footer .brand {
  font-weight: 500;
  color: #606266;
}

.footer .divider {
  margin: 0 8px;
}

.footer .link {
  color: #409eff;
  cursor: pointer;
  text-decoration: none;
}

.footer .link:hover {
  color: #66b1ff;
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
