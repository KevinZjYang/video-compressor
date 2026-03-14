import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  FfmpegInfo,
  Encoder,
  VideoInfo,
  CompressPreset,
  CompressOptions,
  CompressJob,
  CompressProgress,
  EstimateResult
} from "../types";

export const useAppStore = defineStore("app", () => {
  // FFmpeg 状态
  const ffmpegInfo = ref<FfmpegInfo | null>(null);
  const encoders = ref<Encoder[]>([]);
  const loading = ref(false);

  // 视频列表
  const videoList = ref<VideoInfo[]>([]);

  // 预设
  const presets = ref<CompressPreset[]>([]);
  const selectedPreset = ref<CompressPreset | null>(null);

  // 手动选项
  const manualOptions = ref<CompressOptions>({
    encoder: "",
    bitrate: 8000000,
    preset: "medium",
    outputFormat: "mp4",
    audioBitrate: 128000
  });

  // 压缩进度
  const compressProgress = ref<CompressProgress[]>([]);
  const isCompressing = ref(false);
  const wingetAvailable = ref(false);
  const isInstalling = ref(false);

  // 计算属性
  const hasGpuEncoder = computed(() =>
    encoders.value.some(e => e.hardware !== "software")
  );

  // 根据手动选项计算预估总大小
  const estimatedSizeByManual = computed(() => {
    if (videoList.value.length === 0) return 0;
    return videoList.value.reduce((sum, video) => {
      // 视频大小 = (视频码率 + 音频码率) / 8 * 时长
      let videoSize = (manualOptions.value.bitrate + manualOptions.value.audioBitrate) / 8 * video.duration;
      // 如果有缩放，按比例减少（scale 是目标宽度相对于原宽度的比例）
      if (manualOptions.value.scale && manualOptions.value.scale > 0 && manualOptions.value.scale < 100) {
        const scaleRatio = manualOptions.value.scale / 100;
        videoSize = videoSize * scaleRatio * scaleRatio; // 宽高都缩放，面积减少是平方关系
      }
      return sum + videoSize;
    }, 0);
  });

  // 根据手动选项计算预估总时间（相对基准时间的倍数）
  const estimatedTimeByManual = computed(() => {
    if (videoList.value.length === 0) return 0;
    // 基准时间是使用 CPU medium 预设的预估
    const baseTime = videoList.value.reduce((sum, video) => sum + video.duration * 0.5, 0);

    let speedMultiplier = 1.0;

    // 根据编码器调整速度
    const encoder = manualOptions.value.encoder.toLowerCase();
    if (encoder.includes("qsv") || encoder.includes("nvenc") || encoder.includes("amf")) {
      speedMultiplier *= 0.2; // GPU 编码大约快 5 倍
    }

    // 根据 preset 调整速度
    const preset = manualOptions.value.preset;
    switch (preset) {
      case "ultrafast": speedMultiplier *= 0.3; break;
      case "superfast": speedMultiplier *= 0.4; break;
      case "veryfast": speedMultiplier *= 0.5; break;
      case "faster": speedMultiplier *= 0.6; break;
      case "fast": speedMultiplier *= 0.7; break;
      case "medium": speedMultiplier *= 1.0; break;
      case "slow": speedMultiplier *= 1.5; break;
      case "slower": speedMultiplier *= 2.0; break;
      case "veryslow": speedMultiplier *= 3.0; break;
    }

    // 如果使用 CRF 模式，时间会稍长一些
    if (manualOptions.value.crf !== undefined) {
      speedMultiplier *= 1.2;
    }

    return baseTime * speedMultiplier;
  });

  const videoEncoders = computed(() =>
    encoders.value.filter(e => e.type === "video")
  );

  // 显卡类型检测
  const gpuType = ref<'intel' | 'nvidia' | 'amd' | 'unknown'>('unknown');
  const gpuName = ref<string>('');

  // 检测显卡类型
  async function detectGpu() {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      gpuName.value = await invoke<string>("get_gpu_name");
      const name = gpuName.value.toLowerCase();
      if (name.includes('intel') || name.includes('arc') || name.includes('zako')) {
        gpuType.value = 'intel';
      } else if (name.includes('nvidia') || name.includes('geforce') || name.includes('rtx') || name.includes('gtx')) {
        gpuType.value = 'nvidia';
      } else if (name.includes('amd') || name.includes('radeon')) {
        gpuType.value = 'amd';
      }
      console.log('Detected GPU:', gpuName, 'Type:', gpuType.value);
    } catch (e) {
      console.error('Failed to detect GPU:', e);
      gpuType.value = 'unknown';
    }
  }

  // 有用的视频编码器（根据用户显卡动态过滤）
  const usefulVideoEncoders = computed(() => {
    // 根据显卡类型决定允许的GPU编码器
    let gpuEncoders: string[] = [];
    switch (gpuType.value) {
      case 'intel':
        gpuEncoders = ['h264_qsv', 'hevc_qsv', 'av1_qsv'];
        break;
      case 'nvidia':
        gpuEncoders = ['h264_nvenc', 'hevc_nvenc', 'av1_nvenc'];
        break;
      case 'amd':
        gpuEncoders = ['h264_amf', 'hevc_amf', 'av1_amf'];
        break;
      default:
        // 未知显卡，显示所有GPU编码器供选择
        gpuEncoders = [
          'h264_qsv', 'hevc_qsv', 'av1_qsv',
          'h264_nvenc', 'hevc_nvenc', 'av1_nvenc',
          'h264_amf', 'hevc_amf', 'av1_amf'
        ];
    }

    // 允许列表：GPU编码器 + 软件编码器
    const allowList = [
      ...gpuEncoders,
      'libx264', 'libx265',
    ];

    const useful = videoEncoders.value.filter(e =>
      allowList.includes(e.name)
    );

    // 排序：GPU硬件编码器优先，然后是常用软件编码器
    const hardwareOrder: Record<string, number> = {
      'qsv': 1,
      'nvenc': 2,
      'amf': 3,
      'videotoolbox': 4,
      'software': 5
    };

    return useful.sort((a, b) => {
      const orderA = hardwareOrder[a.hardware] ?? 99;
      const orderB = hardwareOrder[b.hardware] ?? 99;
      return orderA - orderB;
    });
  });

  // 检查 winget
  async function checkWinget() {
    try {
      wingetAvailable.value = await invoke<boolean>("check_winget");
    } catch {
      wingetAvailable.value = false;
    }
  }

  // 安装 FFmpeg
  async function installFfmpeg() {
    isInstalling.value = true;
    try {
      await invoke("install_ffmpeg");
      // 重新初始化
      await initFfmpeg();
    } catch (e) {
      console.error("Install FFmpeg error:", e);
      throw e;
    } finally {
      isInstalling.value = false;
    }
  }

  // 初始化 FFmpeg
  async function initFfmpeg() {
    loading.value = true;
    try {
      // 先检测显卡类型
      await detectGpu();

      console.log("Initializing FFmpeg...");
      ffmpegInfo.value = await invoke<FfmpegInfo>("get_ffmpeg_info");
      console.log("FFmpeg info:", ffmpegInfo.value);

      encoders.value = await invoke<Encoder[]>("get_available_encoders");
      console.log("Encoders:", encoders.value);

      // 如果没有GPU编码器，添加软件编码器作为备选
      if (encoders.value.length === 0) {
        encoders.value = [
          { name: "libx264", type: "video", hardware: "software", description: "H.264 (AVC) software encoder" },
          { name: "libx265", type: "video", hardware: "software", description: "H.265 (HEVC) software encoder" }
        ];
      }

      // 自动选择 GPU 编码器
      const gpuEncoder = encoders.value.find(
        e => e.hardware === "qsv" || e.hardware === "nvenc" || e.hardware === "amf"
      );
      const fallbackEncoder = encoders.value.find(e => e.type === "video");
      if (gpuEncoder) {
        manualOptions.value.encoder = gpuEncoder.name;
      } else if (fallbackEncoder) {
        manualOptions.value.encoder = fallbackEncoder.name;
      }
    } catch (e) {
      console.error("Init FFmpeg error:", e);
    } finally {
      loading.value = false;
    }
  }

  // 分析视频
  async function analyzeVideo(path: string): Promise<VideoInfo | null> {
    try {
      console.log("Analyzing video:", path);
      const info = await invoke<VideoInfo>("analyze_video", { path });
      console.log("Video info:", info);
      return info;
    } catch (e) {
      console.error("Analyze video error:", e);
      return null;
    }
  }

  // 添加视频到列表
  async function addVideo(path: string) {
    console.log("Adding video:", path);
    const info = await analyzeVideo(path);
    if (info) {
      videoList.value.push(info);
      console.log("Video added, loading presets...");
      // 获取推荐预设
      await loadPresets(info);
    }
    return info;
  }

  // 移除视频
  function removeVideo(path: string) {
    const index = videoList.value.findIndex(v => v.path === path);
    if (index > -1) {
      videoList.value.splice(index, 1);
    }
  }

  // 清空视频列表
  function clearVideos() {
    videoList.value = [];
    presets.value = [];
    selectedPreset.value = null;
  }

  // 加载预设
  async function loadPresets(videoInfo: VideoInfo) {
    try {
      presets.value = await invoke<CompressPreset[]>("get_compress_presets", {
        videoInfo,
        gpuType: gpuType.value
      });
      if (presets.value.length > 0) {
        selectedPreset.value = presets.value[1]; // 默认选均衡
        // 同时更新手动选项的编码器为GPU编码器
        if (presets.value[0]?.encoder) {
          manualOptions.value.encoder = presets.value[0].encoder;
        }
      }
    } catch (e) {
      console.error("Load presets error:", e);
    }
  }

  // 获取预估
  async function getEstimate(inputPath: string, options: CompressOptions): Promise<EstimateResult | null> {
    try {
      return await invoke<EstimateResult>("get_estimate", {
        inputPath,
        options
      });
    } catch (e) {
      console.error("Get estimate error:", e);
      return null;
    }
  }

  // 开始压缩
  async function startCompress(jobs: CompressJob[]) {
    isCompressing.value = true;
    compressProgress.value = jobs.map((job, index) => ({
      jobId: `job-${index}`,
      filename: job.inputPath.split(/[\\/]/).pop() || "",
      progress: 0,
      status: "pending" as const
    }));

    try {
      // 如果使用预设，合并手动选项中的 outputFormat 和 audioBitrate
      const options = selectedPreset.value
        ? {
            ...selectedPreset.value,
            outputFormat: manualOptions.value.outputFormat,
            audioBitrate: manualOptions.value.audioBitrate
          }
        : manualOptions.value;

      await invoke("compress_videos", {
        jobs,
        options
      });
    } catch (e) {
      console.error("Compress error:", e);
    } finally {
      isCompressing.value = false;
    }
  }

  // 停止压缩
  async function stopCompress() {
    try {
      await invoke("stop_compress");
    } catch (e) {
      console.error("Stop compress error:", e);
    }
    isCompressing.value = false;
  }

  return {
    // State
    ffmpegInfo,
    encoders,
    loading,
    videoList,
    presets,
    selectedPreset,
    manualOptions,
    compressProgress,
    isCompressing,
    wingetAvailable,
    isInstalling,
    gpuType,
    gpuName,

    // Computed
    hasGpuEncoder,
    videoEncoders,
    usefulVideoEncoders,
    estimatedSizeByManual,
    estimatedTimeByManual,

    // Actions
    initFfmpeg,
    checkWinget,
    installFfmpeg,
    analyzeVideo,
    addVideo,
    removeVideo,
    clearVideos,
    loadPresets,
    getEstimate,
    startCompress,
    stopCompress
  };
});
