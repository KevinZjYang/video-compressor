<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useAppStore } from "../stores/app";
import { open, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { onMounted, onUnmounted } from "vue";
import type { CompressProgress, CompressPreset, VideoInfo } from "../types";

const store = useAppStore();

// Tab 状态
const activeTab = ref<'compress' | 'trim'>('compress');

// ==================== 剪切功能相关状态 ====================
const trimVideoPath = ref("");
const trimVideoSrc = ref("");
const trimVideoInfo = ref<VideoInfo | null>(null);
const trimIsPlaying = ref(false);
const trimCurrentTime = ref(0);
const trimDuration = ref(0);
const trimStartTime = ref(0);
const trimEndTime = ref(0);
const trimVideoRef = ref<HTMLVideoElement | null>(null);
const isTrimming = ref(false);
const trimProgress = ref(0);

// 监听Tab切换，当切换到trim时检测GPU
watch(activeTab, (newTab) => {
  if (newTab === 'trim') {
    // 切换到剪切Tab时，可以添加一些初始化逻辑
  }
});

// 本地状态
const dragOver = ref(false);
const expandedVideo = ref<string | null>(null);

// 时间格式化（带单位）
function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);

  if (h > 0) {
    return `${h}小时${m}分`;
  } else if (m > 0) {
    return `${m}分${s}秒`;
  } else {
    return `${s}秒`;
  }
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

// 获取视频对应的压缩进度
function getVideoProgress(filename: string): CompressProgress | undefined {
  return store.compressProgress.find(p => p.filename === filename);
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
    } else {
      store.compressProgress.push(progress);
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

// ==================== 剪切功能相关函数 ====================

// 格式化时间
function trimFormatTime(seconds: number): string {
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
function trimParseTime(timeStr: string): number {
  const parts = timeStr.split(":");
  let seconds = 0;
  if (parts.length === 3) {
    seconds = parseInt(parts[0]) * 3600 + parseInt(parts[1]) * 60 + parseFloat(parts[2]);
  } else if (parts.length === 2) {
    seconds = parseInt(parts[0]) * 60 + parseFloat(parts[1]);
  }
  return seconds;
}

// 选择视频（剪切）
async function selectTrimVideo() {
  const file = await open({
    multiple: false,
    filters: [
      { name: "视频文件", extensions: ["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm"] }
    ]
  });

  if (file) {
    trimVideoPath.value = file as string;

    // 修复视频播放问题：使用convertFileSrc转换路径
    const filePath = file as string;
    try {
      // 直接使用convertFileSrc，它会自动处理Windows路径
      trimVideoSrc.value = convertFileSrc(filePath);
      console.log('Video src:', trimVideoSrc.value);
    } catch (e) {
      console.error('Failed to convert file src:', e);
      // 回退方案
      const normalizedPath = filePath.replace(/\\/g, '/');
      trimVideoSrc.value = 'file://' + normalizedPath;
      console.log('Fallback video src:', trimVideoSrc.value);
    }

    try {
      trimVideoInfo.value = await invoke<VideoInfo>("analyze_video", { path: file });
      trimDuration.value = trimVideoInfo.value.duration;
      trimStartTime.value = 0;
      trimEndTime.value = trimVideoInfo.value.duration;
    } catch (e) {
      console.error("Failed to analyze video:", e);
    }
  }
}

// 时间更新
function onTrimTimeUpdate() {
  if (!trimVideoRef.value) return;
  trimCurrentTime.value = trimVideoRef.value.currentTime;

  if (trimCurrentTime.value >= trimEndTime.value) {
    trimVideoRef.value.pause();
    trimIsPlaying.value = false;
    trimVideoRef.value.currentTime = trimStartTime.value;
  }
}

// 视频加载错误处理
function onTrimVideoError(event: Event) {
  const video = event.target as HTMLVideoElement;
  console.error('Video load error:', video.error);
  console.error('Video src:', trimVideoSrc.value);
  console.error('Video path:', trimVideoPath.value);
  const errorMsg = video.error?.message || '';
  if (errorMsg.includes('Format error') || errorMsg.includes('MEDIA_ELEMENT_ERROR')) {
    alert('视频格式不支持。WebView2可能不支持当前视频的编码格式。\n\n建议：\n1. 尝试使用MP4格式(H.264编码)的视频\n2. 确保视频文件未损坏\n3. 剪切功能仍然可用，导出时会自动转码');
  } else {
    alert('视频加载失败: ' + errorMsg);
  }
}

// 视频加载成功
function onTrimVideoLoaded(event: Event) {
  console.log('Video loaded successfully');
  const video = event.target as HTMLVideoElement;
  console.log('Duration:', video.duration);
}

// 跳转到指定时间
function trimSeekTo(time: number) {
  if (!trimVideoRef.value) return;
  trimVideoRef.value.currentTime = time;
  trimCurrentTime.value = time;
}

// 设置开始时间
function setTrimStartTime(time: number) {
  // 限制范围：0 到 结束时间-0.1
  const minTime = 0;
  const maxTime = trimEndTime.value - 0.1;
  trimStartTime.value = Math.max(minTime, Math.min(time, maxTime));
  trimSeekTo(trimStartTime.value);
}

// 设置结束时间
function setTrimEndTime(time: number) {
  // 限制范围：开始时间+0.1 到 视频总时长
  const minTime = trimStartTime.value + 0.1;
  const maxTime = trimDuration.value;
  trimEndTime.value = Math.max(minTime, Math.min(time, maxTime));
}

// 调整时间
function adjustTrimStartTime(delta: number) {
  setTrimStartTime(trimStartTime.value + delta);
}

function adjustTrimEndTime(delta: number) {
  setTrimEndTime(trimEndTime.value + delta);
}

// 点击时间轴跳转
function onTimelineClick(event: MouseEvent) {
  if (!trimDuration.value) return;
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  const percent = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
  const time = percent * trimDuration.value;
  trimSeekTo(time);
}

// 拖拽状态
const trimDragging = ref<'start' | 'end' | null>(null);

function startTrimDragging(type: 'start' | 'end', event: MouseEvent) {
  event.preventDefault();
  event.stopPropagation();
  trimDragging.value = type;

  // 获取时间轴容器的引用
  const trackElement = (event.currentTarget as HTMLElement).parentElement;
  if (!trackElement) return;

  const rect = trackElement.getBoundingClientRect();

  const onMouseMove = (e: MouseEvent) => {
    if (!trimDuration.value) return;
    const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const time = percent * trimDuration.value;

    if (trimDragging.value === 'start') {
      setTrimStartTime(Math.min(time, trimEndTime.value - 0.1));
    } else if (trimDragging.value === 'end') {
      setTrimEndTime(Math.max(time, trimStartTime.value + 0.1));
    }
  };

  const onMouseUp = () => {
    trimDragging.value = null;
    document.removeEventListener('mousemove', onMouseMove);
    document.removeEventListener('mouseup', onMouseUp);
  };

  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('mouseup', onMouseUp);
}

// 剪切后时长
const trimmedDuration = computed(() => trimEndTime.value - trimStartTime.value);

// 预览剪切效果
function previewTrim() {
  trimSeekTo(trimStartTime.value);
  trimIsPlaying.value = true;
  if (trimVideoRef.value) {
    trimVideoRef.value.play();
  }
}

// 导出视频
async function exportTrimVideo() {
  if (!trimVideoPath.value) return;

  const outputPath = await save({
    defaultPath: trimVideoPath.value.replace(/\.[^.]+$/, "_trimmed.mp4"),
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
      inputPath: trimVideoPath.value,
      startTime: trimStartTime.value,
      endTime: trimEndTime.value,
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
let trimUnlisten: (() => void) | null = null;

onMounted(async () => {
  // 检查 winget 是否可用
  await store.checkWinget();

  unlisten = await listen<CompressProgress>("compress-progress", (event) => {
    const progress = event.payload;
    const index = store.compressProgress.findIndex(p => p.jobId === progress.jobId);
    if (index > -1) {
      store.compressProgress[index] = progress;
    } else {
      store.compressProgress.push(progress);
    }
  });

  // 监听剪切进度
  trimUnlisten = await listen<{ status: string; progress: number }>("trim-progress", (event) => {
    trimProgress.value = event.payload.progress;
    if (event.payload.status === "completed") {
      isTrimming.value = false;
    }
  });
});

onUnmounted(() => {
  if (unlisten) unlisten();
  if (trimUnlisten) trimUnlisten();
});
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
        <div
          class="nav-tab"
          :class="{ active: activeTab === 'compress' }"
          @click="activeTab = 'compress'"
        >
          <span class="icon">📦</span>
          压缩
        </div>
        <div
          class="nav-tab"
          :class="{ active: activeTab === 'trim' }"
          @click="activeTab = 'trim'"
        >
          <span class="icon">✂️</span>
          剪切
        </div>
      </div>
    </div>

    <!-- 压缩Tab内容 -->
    <div class="main-content" v-show="activeTab === 'compress'">
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
            <template v-if="store.isInstalling">
              <el-tag type="warning" effect="plain" style="margin-right: 8px;">
                正在安装 FFmpeg...
              </el-tag>
              <el-progress :percentage="50" :indeterminate="true" :duration="3" :show-text="false" style="width: 120px; display: inline-block;" />
            </template>
            <template v-else>
              <el-button
                v-if="store.wingetAvailable"
                type="primary"
                size="small"
                @click="store.installFfmpeg"
              >
                一键安装 FFmpeg
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
                :class="{
                  expanded: expandedVideo === video.path,
                  compressing: getVideoProgress(video.filename)?.status === 'running'
                }"
                @click="toggleVideoDetail(video.path)"
              >
                <div class="video-icon">
                  <template v-if="getVideoProgress(video.filename)?.status === 'running'">⚡</template>
                  <template v-else-if="getVideoProgress(video.filename)?.status === 'completed'">✅</template>
                  <template v-else-if="getVideoProgress(video.filename)?.status === 'cancelled'">⚠️</template>
                  <template v-else-if="getVideoProgress(video.filename)?.status === 'failed'">❌</template>
                  <template v-else>🎥</template>
                </div>
                <div class="video-info">
                  <div class="video-name">{{ video.filename }}</div>
                  <div class="video-meta" v-if="getVideoProgress(video.filename) as CompressProgress">
                    <template v-if="getVideoProgress(video.filename)?.status === 'running'">
                      <span class="progress-text">
                        {{ getVideoProgress(video.filename)?.progress }}% |
                        已用: {{ formatTime(getVideoProgress(video.filename)?.elapsedTime || 0) }} |
                        剩余: {{ formatTime(getVideoProgress(video.filename)?.estimatedRemainingTime || 0) }}
                      </span>
                    </template>
                    <template v-else-if="getVideoProgress(video.filename)?.status === 'completed'">
                      <span class="status-completed">压缩完成</span>
                    </template>
                    <template v-else-if="getVideoProgress(video.filename)?.status === 'cancelled'">
                      <span class="status-failed">已取消</span>
                    </template>
                    <template v-else-if="getVideoProgress(video.filename)?.status === 'failed'">
                      <span class="status-failed">压缩失败</span>
                    </template>
                    <template v-else>
                      <span class="expand-hint">{{ expandedVideo === video.path ? '▼ 点击收起详情' : '▶ 点击查看详情' }}</span>
                    </template>
                  </div>
                  <div class="video-meta" v-else>
                    <span class="expand-hint">{{ expandedVideo === video.path ? '▼ 点击收起详情' : '▶ 点击查看详情' }}</span>
                  </div>
                </div>
                <!-- 停止按钮 - 只在正在压缩时显示 -->
                <el-button
                  v-if="getVideoProgress(video.filename)?.status === 'running'"
                  type="warning"
                  size="small"
                  @click.stop="store.stopCompress()"
                >
                  停止
                </el-button>
                <!-- 删除按钮 - 未在压缩时显示 -->
                <el-button
                  v-else
                  type="danger"
                  circle
                  size="small"
                  @click.stop="store.removeVideo(video.path)"
                >
                  ×
                </el-button>
              </div>
              <!-- 压缩进度条 -->
              <div v-if="getVideoProgress(video.filename) && (getVideoProgress(video.filename)?.status === 'running' || getVideoProgress(video.filename)?.status === 'completed')" class="video-progress">
                <el-progress
                  :percentage="getVideoProgress(video.filename)?.progress || 0"
                  :status="getVideoProgress(video.filename)?.status === 'completed' ? 'success' : undefined"
                  :stroke-width="6"
                />
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
      </div>
    </div>

    <!-- 剪切Tab内容 -->
    <div class="main-content" v-show="activeTab === 'trim'">
      <!-- 左侧：视频预览 -->
      <div class="left-panel">
        <div class="video-container">
          <video
            v-if="trimVideoSrc"
            ref="trimVideoRef"
            :src="trimVideoSrc"
            @timeupdate="onTrimTimeUpdate"
            @loadedmetadata="trimDuration = trimVideoRef?.duration || 0"
            @error="onTrimVideoError"
            @loadeddata="onTrimVideoLoaded"
            controls
            playsinline
          />
          <div v-else class="video-placeholder" @click="selectTrimVideo">
            <div class="placeholder-icon">🎬</div>
            <div class="placeholder-text">点击选择视频</div>
          </div>
        </div>
      </div>

      <!-- 右侧：剪切设置 -->
      <div class="right-panel">
        <!-- 未选择视频时的提示 -->
        <div class="panel-card empty-hint" v-if="!trimVideoSrc">
          <div class="empty-icon">🎬</div>
          <div class="empty-text">请在左侧选择要剪切的视频</div>
        </div>

        <!-- 视频信息 -->
        <div class="panel-card" v-if="trimVideoInfo">
          <div class="card-header">
            <div class="card-title">视频信息</div>
            <el-button size="small" @click="selectTrimVideo">更换视频</el-button>
          </div>
          <div class="info-grid">
            <div class="info-item full-width">
              <span class="label">文件名</span>
              <span class="value">{{ trimVideoInfo.filename }}</span>
            </div>
            <div class="info-item">
              <span class="label">分辨率</span>
              <span class="value">{{ trimVideoInfo.width }} × {{ trimVideoInfo.height }}</span>
            </div>
            <div class="info-item">
              <span class="label">时长</span>
              <span class="value">{{ trimFormatTime(trimVideoInfo.duration) }}</span>
            </div>
            <div class="info-item">
              <span class="label">帧率</span>
              <span class="value">{{ trimVideoInfo.fps }} fps</span>
            </div>
            <div class="info-item">
              <span class="label">文件大小</span>
              <span class="value">{{ formatSize(trimVideoInfo.size) }}</span>
            </div>
            <div class="info-item">
              <span class="label">视频码率</span>
              <span class="value">{{ formatBitrate(trimVideoInfo.bitrate) }}</span>
            </div>
            <div class="info-item">
              <span class="label">视频编码</span>
              <span class="value">{{ trimVideoInfo.codec }}</span>
            </div>
            <div class="info-item" v-if="trimVideoInfo.audioCodec">
              <span class="label">音频编码</span>
              <span class="value">{{ trimVideoInfo.audioCodec }}</span>
            </div>
            <div class="info-item" v-if="trimVideoInfo.audioBitrate">
              <span class="label">音频码率</span>
              <span class="value">{{ formatBitrate(trimVideoInfo.audioBitrate) }}</span>
            </div>
          </div>
        </div>

        <!-- 时间轴 -->
        <div class="panel-card" v-if="trimVideoSrc && trimDuration > 0">
          <div class="card-title">选择时间段</div>

          <!-- 时间轴滑块 -->
          <div class="timeline-container">
            <div
              class="timeline-track"
              @click="onTimelineClick"
            >
              <div
                class="timeline-range"
                :style="{
                  left: (trimStartTime / trimDuration * 100) + '%',
                  width: ((trimEndTime - trimStartTime) / trimDuration * 100) + '%'
                }"
              />
              <div
                class="timeline-handle start"
                :style="{ left: (trimStartTime / trimDuration * 100) + '%' }"
                @mousedown="startTrimDragging('start', $event)"
              />
              <div
                class="timeline-handle end"
                :style="{ left: (trimEndTime / trimDuration * 100) + '%' }"
                @mousedown="startTrimDragging('end', $event)"
              />
              <div
                class="timeline-playhead"
                :style="{ left: (trimCurrentTime / trimDuration * 100) + '%' }"
              />
            </div>
          </div>

          <!-- 时间输入 -->
          <div class="time-inputs">
            <div class="time-input-group">
              <label>开始时间</label>
              <div class="input-row">
                <el-input
                  :model-value="trimFormatTime(trimStartTime)"
                  @change="(v: string) => setTrimStartTime(trimParseTime(v))"
                  size="small"
                />
                <el-button size="small" @click="adjustTrimStartTime(0.1)">+0.1s</el-button>
                <el-button size="small" @click="adjustTrimStartTime(-0.1)">-0.1s</el-button>
                <el-button size="small" @click="adjustTrimStartTime(1)">+1s</el-button>
                <el-button size="small" @click="adjustTrimStartTime(-1)">-1s</el-button>
              </div>
            </div>

            <div class="time-input-group">
              <label>结束时间</label>
              <div class="input-row">
                <el-input
                  :model-value="trimFormatTime(trimEndTime)"
                  @change="(v: string) => setTrimEndTime(trimParseTime(v))"
                  size="small"
                />
                <el-button size="small" @click="adjustTrimEndTime(0.1)">+0.1s</el-button>
                <el-button size="small" @click="adjustTrimEndTime(-0.1)">-0.1s</el-button>
                <el-button size="small" @click="adjustTrimEndTime(1)">+1s</el-button>
                <el-button size="small" @click="adjustTrimEndTime(-1)">-1s</el-button>
              </div>
            </div>
          </div>

          <!-- 剪切后时长 -->
          <div class="trim-duration">
            剪切后时长: <strong>{{ trimFormatTime(trimmedDuration) }}</strong>
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="panel-card actions" v-if="trimVideoSrc">
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
            @click="exportTrimVideo"
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

.main-content > .left-panel,
.main-content > .right-panel {
  display: flex;
  flex-direction: column;
}

.left-panel {
  width: 480px;
  min-width: 480px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
  padding: 16px;
  min-height: 0;
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

.progress-text {
  font-size: 12px;
  color: #409eff;
}

.status-completed {
  font-size: 12px;
  color: #67c23a;
}

.status-failed {
  font-size: 12px;
  color: #f56c6c;
}

.video-item.compressing {
  background: #ecf5ff;
}

.video-progress {
  padding: 0 16px 12px 16px;
  background: #f5f7fa;
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
  min-height: 0;
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
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.card-header .card-title {
  margin-bottom: 0;
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

/* 剪切功能样式 */
.video-container {
  flex: 1;
  background: #000;
  border-radius: 12px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 300px;
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
  margin-top: 16px;
}

.time-display {
  font-size: 14px;
  color: #606266;
  font-family: monospace;
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

.info-item.full-width {
  grid-column: 1 / -1;
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
  width: 8px;
  height: 28px;
  background: #409eff;
  border-radius: 3px;
  transform: translate(-50%, -50%);
  cursor: ew-resize;
  z-index: 10;
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
  flex-wrap: wrap;
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

.empty-hint {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
  text-align: center;
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 16px;
}

.empty-text {
  font-size: 14px;
  color: #909399;
}
</style>
