<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { open, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { VideoInfo } from "../types";

const router = useRouter();

// 状态
const videoPath = ref("");
const videoSrc = ref("");
const videoInfo = ref<VideoInfo | null>(null);
const isPlaying = ref(false);
const currentTime = ref(0);
const duration = ref(0);

// 剪切时间
const startTime = ref(0);
const endTime = ref(0);

// 状态
const isTrimming = ref(false);
const trimProgress = ref(0);

// 视频元素引用
const videoRef = ref<HTMLVideoElement | null>(null);

// 格式化时间
function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  const ms = Math.floor((seconds % 1) * 10);
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}.${ms}`;
  }
  return `${m}:${s.toString().padStart(2, "0")}.${ms}`;
}

// 解析时间字符串为秒数
function parseTime(timeStr: string): number {
  const parts = timeStr.split(":");
  let seconds = 0;
  if (parts.length === 3) {
    seconds = parseInt(parts[0]) * 3600 + parseInt(parts[1]) * 60 + parseFloat(parts[2]);
  } else if (parts.length === 2) {
    seconds = parseInt(parts[0]) * 60 + parseFloat(parts[1]);
  }
  return seconds;
}

// 选择视频
async function selectVideo() {
  const file = await open({
    multiple: false,
    filters: [
      { name: "视频文件", extensions: ["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm"] }
    ]
  });

  if (file) {
    videoPath.value = file as string;

    // 修复视频播放问题：确保路径格式正确
    try {
      // 标准化路径：将反斜杠转换为正斜杠
      const normalizedPath = (file as string).replace(/\\/g, '/');
      videoSrc.value = convertFileSrc(normalizedPath);
      console.log('Video src:', videoSrc.value);
    } catch (e) {
      console.error('Failed to convert file src:', e);
      // 回退方案：直接使用file协议
      const encodedPath = encodeURI('file:///' + (file as string).replace(/\\/g, '/'));
      videoSrc.value = encodedPath;
      console.log('Fallback video src:', videoSrc.value);
    }

    // 分析视频
    try {
      videoInfo.value = await invoke<VideoInfo>("analyze_video", { path: file });
      duration.value = videoInfo.value.duration;
      startTime.value = 0;
      endTime.value = videoInfo.value.duration;
    } catch (e) {
      console.error("Failed to analyze video:", e);
    }
  }
}

// 视频加载错误处理
function onVideoError(event: Event) {
  const video = event.target as HTMLVideoElement;
  console.error('Video load error:', video.error);
  console.log('Video src:', videoSrc.value);
  console.log('Video path:', videoPath.value);
}

// 播放/暂停
function togglePlay() {
  if (!videoRef.value) return;
  if (isPlaying.value) {
    videoRef.value.pause();
  } else {
    videoRef.value.play();
  }
  isPlaying.value = !isPlaying.value;
}

// 时间更新
function onTimeUpdate() {
  if (!videoRef.value) return;
  currentTime.value = videoRef.value.currentTime;

  // 如果播放到结束时间，暂停
  if (currentTime.value >= endTime.value) {
    videoRef.value.pause();
    isPlaying.value = false;
    videoRef.value.currentTime = startTime.value;
  }
}

// 跳转到指定时间
function seekTo(time: number) {
  if (!videoRef.value) return;
  videoRef.value.currentTime = time;
  currentTime.value = time;
}

// 设置开始时间
function setStartTime(time: number) {
  startTime.value = Math.min(time, endTime.value - 0.1);
  seekTo(startTime.value);
}

// 设置结束时间
function setEndTime(time: number) {
  endTime.value = Math.max(time, startTime.value + 0.1);
}

// 调整时间
function adjustStartTime(delta: number) {
  setStartTime(startTime.value + delta);
}

function adjustEndTime(delta: number) {
  setEndTime(endTime.value + delta);
}

// 拖拽状态
const dragging = ref<'start' | 'end' | null>(null);

function startDragging(type: 'start' | 'end', event: MouseEvent) {
  event.preventDefault();
  dragging.value = type;

  const onMouseMove = (e: MouseEvent) => {
    if (!duration.value) return;
    const rect = (e.currentTarget as HTMLElement).parentElement?.getBoundingClientRect();
    if (!rect) return;
    const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const time = percent * duration.value;

    if (dragging.value === 'start') {
      setStartTime(Math.min(time, endTime.value - 0.1));
    } else if (dragging.value === 'end') {
      setEndTime(Math.max(time, startTime.value + 0.1));
    }
  };

  const onMouseUp = () => {
    dragging.value = null;
    document.removeEventListener('mousemove', onMouseMove);
    document.removeEventListener('mouseup', onMouseUp);
  };

  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('mouseup', onMouseUp);
}

// 剪切后时长
const trimmedDuration = computed(() => endTime.value - startTime.value);

// 预览剪切效果
function previewTrim() {
  seekTo(startTime.value);
  isPlaying.value = true;
  if (videoRef.value) {
    videoRef.value.play();
  }
}

// 导出视频
async function exportVideo() {
  if (!videoPath.value) return;

  const outputPath = await save({
    defaultPath: videoPath.value.replace(/\.[^.]+$/, "_trimmed.mp4"),
    filters: [
      { name: "MP4", extensions: ["mp4"] },
      { name: "MKV", extensions: ["mkv"] }
    ]
  });

  if (!outputPath) return;

  isTrimming.value = true;
  trimProgress.value = 0;

  try {
    await invoke("trim_video", {
      inputPath: videoPath.value,
      startTime: startTime.value,
      endTime: endTime.value,
      outputPath
    });
  } catch (e) {
    console.error("Trim failed:", e);
    alert("剪切失败: " + e);
  } finally {
    isTrimming.value = false;
  }
}

// 监听剪切进度
let unlisten: (() => void) | null = null;

onMounted(async () => {
  unlisten = await listen<{ status: string; progress: number }>("trim-progress", (event) => {
    trimProgress.value = event.payload.progress;
    if (event.payload.status === "completed") {
      isTrimming.value = false;
    }
  });
});

onUnmounted(() => {
  if (unlisten) unlisten();
});

// 返回压缩页面
function goToCompress() {
  router.push("/compress");
}
</script>

<template>
  <div class="trim-view">
    <!-- 顶部导航 -->
    <div class="header">
      <div class="back-btn" @click="goToCompress">
        <span class="icon">←</span>
        返回
      </div>
      <div class="title">剪切视频</div>
    </div>

    <div class="main-content">
      <!-- 左侧：视频预览 -->
      <div class="left-panel">
        <div class="video-container">
          <video
            v-if="videoSrc"
            ref="videoRef"
            :src="videoSrc"
            @timeupdate="onTimeUpdate"
            @loadedmetadata="duration = videoRef?.duration || 0"
            @error="onVideoError"
          />
          <div v-else class="video-placeholder" @click="selectVideo">
            <div class="placeholder-icon">🎬</div>
            <div class="placeholder-text">点击选择视频</div>
          </div>
        </div>

        <!-- 播放控制 -->
        <div class="playback-controls" v-if="videoSrc">
          <el-button circle @click="togglePlay">
            {{ isPlaying ? '⏸' : '▶' }}
          </el-button>
          <span class="time-display">
            {{ formatTime(currentTime) }} / {{ formatTime(duration) }}
          </span>
        </div>
      </div>

      <!-- 右侧：剪切设置 -->
      <div class="right-panel">
        <!-- 视频信息 -->
        <div class="panel-card" v-if="videoInfo">
          <div class="card-title">视频信息</div>
          <div class="info-grid">
            <div class="info-item">
              <span class="label">文件名</span>
              <span class="value">{{ videoInfo.filename }}</span>
            </div>
            <div class="info-item">
              <span class="label">分辨率</span>
              <span class="value">{{ videoInfo.width }} × {{ videoInfo.height }}</span>
            </div>
            <div class="info-item">
              <span class="label">时长</span>
              <span class="value">{{ formatTime(videoInfo.duration) }}</span>
            </div>
            <div class="info-item">
              <span class="label">帧率</span>
              <span class="value">{{ videoInfo.fps }} fps</span>
            </div>
          </div>
        </div>

        <!-- 时间轴 -->
        <div class="panel-card" v-if="videoSrc">
          <div class="card-title">选择时间段</div>

          <!-- 时间轴滑块 -->
          <div class="timeline-container">
            <div class="timeline-track">
              <div
                class="timeline-range"
                :style="{
                  left: (startTime / duration * 100) + '%',
                  width: ((endTime - startTime) / duration * 100) + '%'
                }"
              />
              <div
                class="timeline-handle start"
                :style="{ left: (startTime / duration * 100) + '%' }"
                @mousedown.stop="startDragging('start', $event)"
              />
              <div
                class="timeline-handle end"
                :style="{ left: (endTime / duration * 100) + '%' }"
                @mousedown.stop="startDragging('end', $event)"
              />
              <div
                class="timeline-playhead"
                :style="{ left: (currentTime / duration * 100) + '%' }"
              />
            </div>
          </div>

          <!-- 时间输入 -->
          <div class="time-inputs">
            <div class="time-input-group">
              <label>开始时间</label>
              <div class="input-row">
                <el-input
                  :model-value="formatTime(startTime)"
                  @change="(v: string) => setStartTime(parseTime(v))"
                  size="small"
                />
                <el-button size="small" @click="adjustStartTime(0.1)">+0.1s</el-button>
                <el-button size="small" @click="adjustStartTime(-0.1)">-0.1s</el-button>
                <el-button size="small" @click="adjustStartTime(1)">+1s</el-button>
                <el-button size="small" @click="adjustStartTime(-1)">-1s</el-button>
              </div>
            </div>

            <div class="time-input-group">
              <label>结束时间</label>
              <div class="input-row">
                <el-input
                  :model-value="formatTime(endTime)"
                  @change="(v: string) => setEndTime(parseTime(v))"
                  size="small"
                />
                <el-button size="small" @click="adjustEndTime(0.1)">+0.1s</el-button>
                <el-button size="small" @click="adjustEndTime(-0.1)">-0.1s</el-button>
                <el-button size="small" @click="adjustEndTime(1)">+1s</el-button>
                <el-button size="small" @click="adjustEndTime(-1)">-1s</el-button>
              </div>
            </div>
          </div>

          <!-- 剪切后时长 -->
          <div class="trim-duration">
            剪切后时长: <strong>{{ formatTime(trimmedDuration) }}</strong>
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="panel-card actions" v-if="videoSrc">
          <el-button
            type="primary"
            size="large"
            @click="previewTrim"
          >
            预览剪切效果
          </el-button>
          <el-button
            type="success"
            size="large"
            :loading="isTrimming"
            @click="exportVideo"
          >
            {{ isTrimming ? '导出中...' : '导出视频' }}
          </el-button>
        </div>

        <!-- 进度 -->
        <div class="panel-card" v-if="isTrimming">
          <el-progress :percentage="trimProgress" status="success" />
          <div class="trim-hint">正在导出视频，请稍候...</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.trim-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #f5f7fa;
}

.header {
  display: flex;
  align-items: center;
  padding: 0 20px;
  height: 56px;
  background: #fff;
  border-bottom: 1px solid #e4e7ed;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.08);
}

.back-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 12px;
  border-radius: 6px;
  cursor: pointer;
  color: #606266;
  transition: all 0.2s;
}

.back-btn:hover {
  background: #f5f7fa;
  color: #409eff;
}

.back-btn .icon {
  font-size: 18px;
}

.title {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
  margin-left: 20px;
}

.main-content {
  flex: 1;
  display: flex;
  padding: 20px;
  gap: 20px;
  overflow: hidden;
}

.left-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.video-container {
  flex: 1;
  background: #000;
  border-radius: 12px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

.video-container video {
  max-width: 100%;
  max-height: 100%;
}

.video-placeholder {
  text-align: center;
  cursor: pointer;
  color: #fff;
}

.placeholder-icon {
  font-size: 64px;
  margin-bottom: 16px;
}

.placeholder-text {
  font-size: 16px;
}

.playback-controls {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 16px;
  background: #fff;
  border-radius: 8px;
}

.time-display {
  font-size: 14px;
  color: #606266;
  font-family: monospace;
}

.right-panel {
  width: 400px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}

.panel-card {
  background: #fff;
  border-radius: 12px;
  padding: 20px;
}

.card-title {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 16px;
}

.info-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.info-item {
  display: flex;
  flex-direction: column;
}

.info-item .label {
  font-size: 12px;
  color: #909399;
}

.info-item .value {
  font-size: 14px;
  color: #303133;
}

.timeline-container {
  margin-bottom: 20px;
}

.timeline-track {
  position: relative;
  height: 24px;
  background: #e4e7ed;
  border-radius: 4px;
  cursor: pointer;
}

.timeline-range {
  position: absolute;
  top: 0;
  height: 100%;
  background: #409eff;
  opacity: 0.3;
  border-radius: 4px;
}

.timeline-handle {
  position: absolute;
  top: 50%;
  width: 12px;
  height: 24px;
  background: #409eff;
  border-radius: 2px;
  transform: translate(-50%, -50%);
  cursor: ew-resize;
}

.timeline-handle.start {
  background: #67c23a;
}

.timeline-handle.end {
  background: #e6a23c;
}

.timeline-playhead {
  position: absolute;
  top: 0;
  width: 2px;
  height: 100%;
  background: #f56c6c;
  transform: translateX(-50%);
}

.time-inputs {
  display: flex;
  flex-direction: column;
  gap: 16px;
  margin-bottom: 16px;
}

.time-input-group label {
  display: block;
  font-size: 14px;
  color: #606266;
  margin-bottom: 8px;
}

.input-row {
  display: flex;
  gap: 8px;
}

.input-row .el-input {
  width: 140px;
}

.trim-duration {
  text-align: center;
  padding: 12px;
  background: #f0f9ff;
  border-radius: 8px;
  color: #409eff;
}

.trim-duration strong {
  font-size: 18px;
}

.actions {
  display: flex;
  gap: 12px;
}

.actions .el-button {
  flex: 1;
}

.trim-hint {
  text-align: center;
  margin-top: 8px;
  font-size: 12px;
  color: #909399;
}
</style>
