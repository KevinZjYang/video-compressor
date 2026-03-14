<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useAppStore } from "../stores/app";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { onMounted, onUnmounted } from "vue";
import type { CompressProgress, CompressPreset } from "../types";

const router = useRouter();
const store = useAppStore();

// 本地状态
const dragOver = ref(false);
const expandedVideo = ref<string | null>(null);

// 时间格式化
function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }
  return `${m}:${s.toString().padStart(2, "0")}`;
}

// 文件大小格式化
function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  if (bytes < 1024 * 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + " MB";
  return (bytes / 1024 / 1024 / 1024).toFixed(2) + " GB";
}

// 选择预设时应用参数到手动设置
function selectPreset(preset: CompressPreset) {
  store.selectedPreset = preset;
  store.manualOptions.bitrate = preset.bitrate;
  store.manualOptions.encoder = preset.encoder;
  store.manualOptions.preset = preset.preset;
}

// 手动参数改变时取消预设选中
function onManualParamChange() {
  store.selectedPreset = null;
}

// 获取用户友好的编码器名称
function getEncoderLabel(enc: { name: string; hardware: string }): string {
  const name = enc.name.toLowerCase();
  let format = '';
  if (name.includes('h264') || name === 'libx264') {
    format = 'H.264';
  } else if (name.includes('h265') || name.includes('hevc') || name === 'libx265') {
    format = 'H.265/HEVC';
  } else if (name.includes('av1')) {
    format = 'AV1';
  } else {
    format = enc.name;
  }

  let hw = '';
  if (enc.hardware === 'qsv') {
    hw = 'Intel GPU';
  } else if (enc.hardware === 'nvenc') {
    hw = 'NVIDIA GPU';
  } else if (enc.hardware === 'amf') {
    hw = 'AMD GPU';
  } else if (enc.hardware === 'software') {
    hw = '软件';
  }

  return hw ? `${format} (${hw})` : format;
}

// 视频编码格式用户友好显示
function getCodecLabel(codec: string): string {
  const c = codec.toLowerCase();
  if (c === 'h264' || c === 'avc') return 'H.264 (AVC)';
  if (c === 'hevc' || c === 'h265') return 'H.265 (HEVC)';
  if (c === 'av1') return 'AV1';
  if (c === 'vp8') return 'VP8';
  if (c === 'vp9') return 'VP9';
  if (c === 'mpeg4' || c === 'xvid') return 'MPEG-4';
  return codec.toUpperCase();
}

// 音频编码格式用户友好显示
function getAudioCodecLabel(codec: string | undefined): string {
  if (!codec) return '-';
  const c = codec.toLowerCase();
  if (c === 'aac') return 'AAC';
  if (c === 'mp3') return 'MP3';
  if (c === 'ac3') return 'AC3';
  if (c === 'eac3' || c === 'e-ac3') return 'E-AC3';
  if (c === 'opus') return 'Opus';
  if (c === 'vorbis') return 'Vorbis';
  if (c === 'flac') return 'FLAC';
  if (c === 'pcm' || c === 'wav') return 'PCM/WAV';
  return codec.toUpperCase();
}

// 码率格式化
function formatBitrate(bps: number): string {
  if (bps >= 1000000) {
    return (bps / 1000000).toFixed(1) + ' Mbps';
  } else if (bps >= 1000) {
    return (bps / 1000).toFixed(0) + ' kbps';
  }
  return bps + ' bps';
}

// 切换视频详情展开状态
function toggleVideoDetail(path: string) {
  if (expandedVideo.value === path) {
    expandedVideo.value = null;
  } else {
    expandedVideo.value = path;
  }
}

// 选择文件
async function selectFiles() {
  const files = await open({
    multiple: true,
    filters: [
      { name: "视频文件", extensions: ["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm"] }
    ]
  });

  if (files) {
    const paths = Array.isArray(files) ? files : [files];
    for (const path of paths) {
      await store.addVideo(path);
    }
  }
}

// 拖拽处理
function onDragOver(e: DragEvent) {
  e.preventDefault();
  dragOver.value = true;
}

function onDragLeave() {
  dragOver.value = false;
}

async function onDrop(e: DragEvent) {
  e.preventDefault();
  dragOver.value = false;

  const files = e.dataTransfer?.files;
  if (files) {
    for (let i = 0; i < files.length; i++) {
      // 注意：浏览器中无法直接获取文件路径，需要使用其他方式
      // 拖拽功能目前需要通过文件对话框实现
    }
  }
}

// 切换到剪切页面
function goToTrim() {
  router.push("/trim");
}

// 监听压缩进度
let unlisten: (() => void) | null = null;

onMounted(async () => {
  // 检查 winget 是否可用
  await store.checkWinget();

  unlisten = await listen<CompressProgress>("compress-progress", (event) => {
    const progress = event.payload;
    const index = store.compressProgress.findIndex(p => p.jobId === progress.jobId);
    if (index > -1) {
      store.compressProgress[index] = progress;
    }
  });
});

onUnmounted(() => {
  if (unlisten) unlisten();
});

// 使用 store 中的预估计算（根据手动选项实时计算）
const totalEstimatedSize = computed(() => store.estimatedSizeByManual);
const totalEstimatedTime = computed(() => store.estimatedTimeByManual);

// 开始压缩
async function startCompress() {
  if (store.videoList.length === 0) return;

  const outputDir = await open({
    directory: true,
    title: "选择输出目录"
  });

  if (!outputDir) return;

  // 获取编码简称
  function getEncoderShortName(encoder: string): string {
    const name = encoder.toLowerCase();
    if (name.includes('hevc') || name.includes('h265')) return 'hevc';
    if (name.includes('av1')) return 'av1';
    if (name.includes('h264') || name.includes('avc')) return 'h264';
    return 'video';
  }

  // 获取当前日期
  const now = new Date();
  const dateStr = `${now.getFullYear()}${String(now.getMonth() + 1).padStart(2, '0')}${String(now.getDate()).padStart(2, '0')}`;

  const jobs = store.videoList.map((video) => {
    const ext = store.manualOptions.outputFormat || "mp4";
    const encoderShort = getEncoderShortName(store.manualOptions.encoder);
    const outputPath = `${outputDir}/${video.filename.replace(/\.[^.]+$/, "")}_${encoderShort}_${dateStr}.${ext}`;
    return {
      inputPath: video.path,
      outputPath,
      options: store.manualOptions
    };
  });

  await store.startCompress(jobs);
}
</script>

<template>
  <div class="compress-view">
    <!-- 顶部导航 -->
    <div class="header">
      <div class="logo">
        <span class="icon">🎬</span>
        <span class="title">视频压缩器</span>
      </div>
      <div class="nav-tabs">
        <div class="nav-tab active">
          <span class="icon">📦</span>
          压缩
        </div>
        <div class="nav-tab" @click="goToTrim">
          <span class="icon">✂️</span>
          剪切
        </div>
      </div>
    </div>

    <div class="main-content">
      <!-- 左侧：文件选择 -->
      <div class="left-panel">
        <!-- FFmpeg 状态 - 放在最上面 -->
        <div class="ffmpeg-status">
          <!-- FFmpeg 版本 -->
          <el-tag v-if="store.ffmpegInfo?.available" type="success" effect="plain">
            ✓ FFmpeg {{ store.ffmpegInfo.version }}
          </el-tag>
          <template v-else>
            <el-tag type="danger" effect="plain">
              ✗ FFmpeg 未找到
            </el-tag>
            <el-button
              v-if="store.wingetAvailable"
              type="primary"
              size="small"
              :loading="store.isInstalling"
              @click="store.installFfmpeg"
            >
              {{ store.isInstalling ? '安装中...' : '一键安装 FFmpeg' }}
            </el-button>
            <el-button
              v-else
              type="primary"
              size="small"
              @click="selectFiles"
            >
              选择本地 FFmpeg
            </el-button>
          </template>
          <!-- GPU 加速 -->
          <el-tag v-if="store.hasGpuEncoder" type="success" effect="plain">
            ✓ GPU 加速可用
          </el-tag>
          <!-- 显卡信息 -->
          <el-tag type="info" effect="plain">
            显卡: {{ store.gpuName || store.gpuType }}
          </el-tag>
        </div>

        <div
          class="drop-zone"
          :class="{ 'drag-over': dragOver }"
          @click="selectFiles"
          @dragover="onDragOver"
          @dragleave="onDragLeave"
          @drop="onDrop"
        >
          <div class="drop-icon">📁</div>
          <div class="drop-text">点击选择视频文件</div>
          <div class="drop-hint">或拖拽文件到此处</div>
          <div class="drop-formats">支持 MP4, MKV, AVI, MOV, WMV, FLV, WEBM</div>
        </div>

        <!-- 视频列表 -->
        <div class="video-list" v-if="store.videoList.length > 0">
          <div class="list-header">
            <span>已选择 {{ store.videoList.length }} 个文件</span>
            <el-button type="danger" text size="small" @click="store.clearVideos">清空</el-button>
          </div>
          <div class="list-items">
            <div
              v-for="video in store.videoList"
              :key="video.path"
              class="video-item-wrapper"
            >
              <div
                class="video-item"
                :class="{ expanded: expandedVideo === video.path }"
                @click="toggleVideoDetail(video.path)"
              >
                <div class="video-icon">🎥</div>
                <div class="video-info">
                  <div class="video-name">{{ video.filename }}</div>
                  <div class="video-meta">
                    <span class="expand-hint">{{ expandedVideo === video.path ? '▼ 点击收起详情' : '▶ 点击查看详情' }}</span>
                  </div>
                </div>
                <el-button
                  type="danger"
                  circle
                  size="small"
                  @click.stop="store.removeVideo(video.path)"
                >
                  ×
                </el-button>
              </div>
              <!-- 展开的详情面板 -->
              <div v-if="expandedVideo === video.path" class="video-detail">
                <div class="detail-section">
                  <div class="detail-title">基本信息</div>
                  <div class="detail-row">
                    <span class="detail-label">分辨率</span>
                    <span class="detail-value">{{ video.width }} × {{ video.height }}</span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label">时长</span>
                    <span class="detail-value">{{ formatTime(video.duration) }}</span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label">帧率</span>
                    <span class="detail-value">{{ video.fps }} fps</span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label">文件大小</span>
                    <span class="detail-value">{{ formatSize(video.size) }}</span>
                  </div>
                </div>
                <div class="detail-section">
                  <div class="detail-title">视频信息</div>
                  <div class="detail-row">
                    <span class="detail-label">码率</span>
                    <span class="detail-value">{{ formatBitrate(video.bitrate) }}</span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label">编码</span>
                    <span class="detail-value">{{ getCodecLabel(video.codec) }}</span>
                  </div>
                </div>
                <div class="detail-section">
                  <div class="detail-title">音频信息</div>
                  <div class="detail-row">
                    <span class="detail-label">编码</span>
                    <span class="detail-value">{{ getAudioCodecLabel(video.audioCodec) }}</span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label">码率</span>
                    <span class="detail-value">{{ video.audioBitrate && video.audioBitrate > 0 ? formatBitrate(video.audioBitrate) : '-' }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 右侧：设置 -->
      <div class="right-panel">
        <!-- 预设选择 -->
        <div class="panel-card" v-if="store.presets.length > 0">
          <div class="card-title">压缩预设</div>
          <div class="preset-grid">
            <div
              v-for="preset in store.presets"
              :key="preset.id"
              class="preset-card"
              :class="{ active: store.selectedPreset?.id === preset.id }"
              @click="selectPreset(preset)"
            >
              <div class="preset-name">{{ preset.name }}</div>
              <div class="preset-desc">{{ preset.description }}</div>
              <div class="preset-bitrate">{{ preset.bitrate / 1000000 }} Mbps</div>
              <div class="preset-size">约 {{ formatSize(preset.estimatedSize) }}</div>
            </div>
          </div>
        </div>

        <!-- 手动设置 -->
        <div class="panel-card">
          <div class="card-title">手动设置</div>

          <el-form label-width="80px" size="default">
            <el-form-item label="编码器">
              <el-select v-model="store.manualOptions.encoder" placeholder="选择编码器">
                <el-option
                  v-for="enc in store.usefulVideoEncoders"
                  :key="enc.name"
                  :label="getEncoderLabel(enc)"
                  :value="enc.name"
                >
                  <span>{{ getEncoderLabel(enc) }}</span>
                </el-option>
              </el-select>
              <div class="form-tip">视频编码方式，GPU加速更快</div>
            </el-form-item>

            <el-form-item label="码率">
              <div class="bitrate-control">
                <el-slider
                  v-model="store.manualOptions.bitrate"
                  :min="500000"
                  :max="50000000"
                  :step="500000"
                  :format-tooltip="(v: number) => (v / 1000000).toFixed(0) + ' Mbps'"
                  @change="onManualParamChange"
                />
                <span class="bitrate-value">{{ (store.manualOptions.bitrate / 1000000).toFixed(0) }} Mbps</span>
              </div>
              <div class="form-tip">数值越大越清晰但文件越大</div>
            </el-form-item>

            <el-form-item label="编码预设">
              <el-select v-model="store.manualOptions.preset">
                <el-option label="ultrafast - 最快，画质最差" value="ultrafast" />
                <el-option label="superfast - 很快" value="superfast" />
                <el-option label="veryfast - 较快" value="veryfast" />
                <el-option label="faster - 较快" value="faster" />
                <el-option label="fast - 快速" value="fast" />
                <el-option label="medium - 均衡 (推荐)" value="medium" />
                <el-option label="slow - 较慢，画质好" value="slow" />
                <el-option label="slower - 很慢，画质更好" value="slower" />
                <el-option label="veryslow - 最慢，画质最好" value="veryslow" />
              </el-select>
              <div class="form-tip">压缩速度：快→慢，画质：差→好</div>
            </el-form-item>

            <el-form-item label="输出格式">
              <el-radio-group v-model="store.manualOptions.outputFormat">
                <el-radio value="mp4">MP4 (兼容性好)</el-radio>
                <el-radio value="mkv">MKV (支持更多编码)</el-radio>
              </el-radio-group>
            </el-form-item>

            <el-form-item label="音频码率">
              <el-select v-model="store.manualOptions.audioBitrate">
                <el-option label="64 kbps - 低" :value="64000" />
                <el-option label="128 kbps - 标准 (推荐)" :value="128000" />
                <el-option label="192 kbps - 高" :value="192000" />
                <el-option label="256 kbps - 很高" :value="256000" />
                <el-option label="320 kbps - 最高" :value="320000" />
              </el-select>
              <div class="form-tip">128kbps 足够日常使用</div>
            </el-form-item>
          </el-form>
        </div>

        <!-- 预估信息 -->
        <div class="panel-card estimate-card" v-if="store.videoList.length > 0">
          <div class="estimate-info">
            <div class="estimate-item">
              <span class="label">预估总大小:</span>
              <span class="value">{{ formatSize(totalEstimatedSize) }}</span>
            </div>
            <div class="estimate-item">
              <span class="label">预估总时间:</span>
              <span class="value">{{ formatTime(totalEstimatedTime) }}</span>
            </div>
          </div>
          <el-button
            type="primary"
            size="large"
            class="compress-btn"
            :loading="store.isCompressing"
            :disabled="store.videoList.length === 0"
            @click="startCompress"
          >
            {{ store.isCompressing ? '压缩中...' : '开始压缩' }}
          </el-button>
        </div>

        <!-- 压缩进度 -->
        <div class="panel-card" v-if="store.compressProgress.length > 0">
          <div class="card-title">压缩进度</div>
          <div class="progress-list">
            <div
              v-for="progress in store.compressProgress"
              :key="progress.jobId"
              class="progress-item"
            >
              <div class="progress-filename">{{ progress.filename }}</div>
              <el-progress
                :percentage="progress.progress"
                :status="progress.status === 'completed' ? 'success' : progress.status === 'failed' ? 'exception' : undefined"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.compress-view {
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

.logo {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 18px;
  font-weight: 600;
  color: #303133;
  margin-right: 40px;
}

.logo .icon {
  font-size: 24px;
}

.nav-tabs {
  display: flex;
  gap: 8px;
}

.nav-tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: 6px;
  cursor: pointer;
  color: #606266;
  transition: all 0.2s;
}

.nav-tab:hover {
  background: #f5f7fa;
  color: #409eff;
}

.nav-tab.active {
  background: #ecf5ff;
  color: #409eff;
}

.main-content {
  flex: 1;
  display: flex;
  padding: 20px;
  gap: 20px;
  overflow: hidden;
}

.left-panel {
  width: 380px;
  min-width: 380px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
  padding: 16px;
}

.drop-zone {
  background: #fff;
  border: 2px dashed #dcdfe6;
  border-radius: 12px;
  padding: 26px 20px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s;
}

.drop-zone:hover {
  border-color: #409eff;
  background: #f5f7fa;
}

.drop-zone.drag-over {
  border-color: #409eff;
  background: #ecf5ff;
}

.drop-icon {
  font-size: 48px;
  margin-bottom: 12px;
}

.drop-text {
  font-size: 16px;
  color: #303133;
  margin-bottom: 8px;
}

.drop-hint {
  color: #909399;
  margin-bottom: 12px;
}

.drop-formats {
  font-size: 12px;
  color: #c0c4cc;
}

.video-list {
  background: #fff;
  border-radius: 12px;
  overflow: hidden;
}

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #e4e7ed;
  font-size: 14px;
  color: #606266;
}

.list-items {
  max-height: 400px;
  overflow-y: auto;
}

.video-item:hover {
  background: #f5f7fa;
}

.video-icon {
  font-size: 24px;
}

.video-info {
  flex: 1;
  min-width: 0;
}

.video-name {
  font-size: 14px;
  color: #303133;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.video-meta {
  display: flex;
  gap: 6px;
  margin-top: 4px;
  flex-wrap: wrap;
}

.tag {
  font-size: 12px;
  padding: 2px 6px;
  background: #f0f2f5;
  color: #909399;
  border-radius: 4px;
}

.expand-hint {
  font-size: 12px;
  color: #409eff;
  margin-left: auto;
  cursor: pointer;
}

.video-item-wrapper {
  border-bottom: 1px solid #f0f2f5;
}

.video-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  cursor: pointer;
  transition: background 0.2s;
}

.video-item:hover {
  background: #f5f7fa;
}

.video-item.expanded {
  background: #ecf5ff;
}

.video-detail {
  padding: 12px 16px;
  background: #fafafa;
  border-top: 1px solid #ebeef5;
  animation: slideDown 0.2s ease-out;
}

@keyframes slideDown {
  from {
    opacity: 0;
    max-height: 0;
  }
  to {
    opacity: 1;
    max-height: 500px;
  }
}

.detail-section {
  margin-bottom: 12px;
}

.detail-section:last-child {
  margin-bottom: 0;
}

.detail-title {
  font-size: 12px;
  font-weight: 600;
  color: #606266;
  margin-bottom: 8px;
  padding-bottom: 4px;
  border-bottom: 1px solid #ebeef5;
}

.detail-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 0;
  font-size: 13px;
}

.detail-label {
  color: #909399;
}

.detail-value {
  color: #303133;
  font-weight: 500;
}

.ffmpeg-status {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
  margin-top: 12px;
  padding-bottom: 12px;
}

.right-panel {
  flex: 1;
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

.preset-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}

.preset-card {
  padding: 16px;
  border: 2px solid #e4e7ed;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  text-align: center;
}

.preset-card:hover {
  border-color: #409eff;
}

.preset-card.active {
  border-color: #409eff;
  background: #ecf5ff;
}

.preset-name {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 4px;
}

.preset-desc {
  font-size: 12px;
  color: #909399;
  margin-bottom: 8px;
}

.preset-bitrate {
  font-size: 14px;
  color: #409eff;
  font-weight: 500;
}

.preset-size {
  font-size: 12px;
  color: #67c23a;
  margin-top: 4px;
}

.form-tip {
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}

.bitrate-control {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
}

.bitrate-control .el-slider {
  flex: 1;
}

.bitrate-value {
  min-width: 70px;
  text-align: right;
  font-size: 14px;
  color: #409eff;
  font-weight: 500;
}

.estimate-card {
  text-align: center;
}

.estimate-info {
  display: flex;
  justify-content: space-around;
  margin-bottom: 20px;
}

.estimate-item {
  text-align: center;
}

.estimate-item .label {
  display: block;
  font-size: 12px;
  color: #909399;
  margin-bottom: 4px;
}

.estimate-item .value {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
}

.compress-btn {
  width: 100%;
  height: 44px;
  font-size: 16px;
}

.progress-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.progress-filename {
  font-size: 13px;
  color: #606266;
  margin-bottom: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
