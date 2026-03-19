<script setup lang="ts">
import { onMounted } from "vue";
import { useAppStore } from "./stores/app";
import { open } from "@tauri-apps/plugin-shell";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import pkg from "../package.json";

const store = useAppStore();

onMounted(() => {
  store.initFfmpeg();
});

const openWebsite = async () => {
  await open("https://apphub.yiun.top/");
};

// 选择本地 FFmpeg
const selectLocalFfmpeg = async () => {
  const selected = await openDialog({
    directory: false,
    multiple: false,
    filters: [{
      name: "Executable",
      extensions: ["exe"]
    }]
  });
  if (selected) {
    console.log("Selected FFmpeg path:", selected);
    // 重新初始化以检测新选择的 FFmpeg
    await store.initFfmpeg();
    store.showFfmpegGuide = false;
  }
};
</script>

<template>
  <div class="app-container">
    <router-view />
    <div class="footer">
      <span class="brand">视频压缩器-K</span>
      <span class="divider">|</span>
      <span class="version">v{{ pkg.version }}</span>
      <span class="divider">|</span>
      <a class="link" @click="openWebsite">官网: https://apphub.yiun.top/</a>
    </div>

    <!-- FFmpeg 安装引导弹窗 -->
    <el-dialog
      v-model="store.showFfmpegGuide"
      title="欢迎使用视频压缩器"
      width="420px"
      :close-on-click-modal="false"
      :show-close="false"
    >
      <div class="guide-content">
        <div class="guide-icon">📦</div>
        <template v-if="store.isInstalling">
          <p class="guide-text" style="color: #409eff;">
            正在安装 FFmpeg，请稍候...
          </p>
          <p class="guide-hint">
            首次安装可能需要1-3分钟，请耐心等待
          </p>
          <el-progress :percentage="50" :indeterminate="true" :duration="3" :show-text="false" />
        </template>
        <template v-else>
          <p class="guide-text">
            检测到您的系统尚未安装 <strong>FFmpeg</strong>，这是视频压缩的必需组件。
          </p>
          <p class="guide-hint">
            点击下方按钮自动安装，或选择本地已有的 FFmpeg。
          </p>
        </template>
      </div>
      <template #footer>
        <div class="guide-footer">
          <el-button @click="store.showFfmpegGuide = false">
            稍后再说
          </el-button>
          <el-button
            v-if="store.wingetAvailable"
            type="primary"
            :loading="store.isInstalling"
            @click="store.installFfmpeg"
          >
            {{ store.isInstalling ? '安装中...' : '一键安装 FFmpeg' }}
          </el-button>
          <el-button
            v-else
            type="primary"
            @click="selectLocalFfmpeg"
          >
            选择本地 FFmpeg
          </el-button>
        </div>
      </template>
    </el-dialog>
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

.footer .version {
  color: #909399;
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

/* FFmpeg 安装引导弹窗样式 */
.guide-content {
  text-align: center;
  padding: 10px 0;
}

.guide-icon {
  font-size: 48px;
  margin-bottom: 16px;
}

.guide-text {
  font-size: 15px;
  color: #303133;
  margin-bottom: 12px;
  line-height: 1.6;
}

.guide-text strong {
  color: #409eff;
}

.guide-hint {
  font-size: 13px;
  color: #909399;
}

.guide-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
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
