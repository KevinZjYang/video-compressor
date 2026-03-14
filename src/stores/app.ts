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

  // FFmpeg 安装引导弹窗
  const showFfmpegGuide = ref(false);

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

  // 根据手动选项计算预估总时间
  const estimatedTimeByManual = computed(() => {
    if (videoList.value.length === 0) return 0;

    // 编码器基础速度（帧/秒）- 基于经验值
    let encoderSpeed = 30; // 默认 CPU 编码速度

    // 根据编码器调整基础速度
    const encoder = manualOptions.value.encoder.toLowerCase();
    if (encoder.includes("nvenc")) {
      encoderSpeed = 150; // NVIDIA GPU 编码速度
    } else if (encoder.includes("qsv")) {
      encoderSpeed = 100; // Intel QuickSync 速度
    } else if (encoder.includes("amf")) {
      encoderSpeed = 80; // AMD GPU 速度
    } else if (encoder.includes("x264")) {
      encoderSpeed = 30; // x264 软件编码
    } else if (encoder.includes("x265") || encoder.includes("hevc")) {
      encoderSpeed = 15; // x265 软件编码较慢
    }

    // 根据 preset 调整速度
    const preset = manualOptions.value.preset;
    let presetSpeed = 1.0;
    switch (preset) {
      case "ultrafast": presetSpeed = 4.0; break;
      case "superfast": presetSpeed = 3.0; break;
      case "veryfast": presetSpeed = 2.0; break;
      case "faster": presetSpeed = 1.5; break;
      case "fast": presetSpeed = 1.2; break;
      case "medium": presetSpeed = 1.0; break;
      case "slow": presetSpeed = 0.6; break;
      case "slower": presetSpeed = 0.4; break;
      case "veryslow": presetSpeed = 0.25; break;
    }

    // 实际编码速度 = 编码器基础速度 × preset系数
    const actualSpeed = encoderSpeed * presetSpeed;

    // 计算每个视频的预估时间
    let totalTime = 0;
    for (const video of videoList.value) {
      // 分辨率系数：分辨率越高，编码越慢
      const pixels = video.width * video.height;
      let resolutionFactor = 1.0;
      if (pixels >= 3840 * 2160) {
        resolutionFactor = 4.0; // 4K
      } else if (pixels >= 2560 * 1440) {
        resolutionFactor = 2.0; // 2K
      } else if (pixels >= 1920 * 1080) {
        resolutionFactor = 1.0; // 1080p
      } else if (pixels >= 1280 * 720) {
        resolutionFactor = 0.5; // 720p
      } else {
        resolutionFactor = 0.3; // 480p 及以下
      }

      // 帧率系数：帧率越高，处理越慢
      let fpsFactor = 1.0;
      if (video.fps >= 60) {
        fpsFactor = 1.5;
      } else if (video.fps >= 30) {
        fpsFactor = 1.0;
      } else {
        fpsFactor = 0.7;
      }

      // CRF 模式会稍慢
      let crfFactor = 1.0;
      if (manualOptions.value.crf !== undefined) {
        crfFactor = 1.1;
      }

      // 预估时间 = 视频时长 × 分辨率系数 × 帧率系数 × CRF系数 / 实际编码速度
      // 额外增加 20% 的 IO 和开销时间
      const videoTime = video.duration * resolutionFactor * fpsFactor * crfFactor / actualSpeed * 1.2;
      totalTime += videoTime;
    }

    return totalTime;
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
      // 关闭引导弹窗
      showFfmpegGuide.value = false;
    } catch (e) {
      console.error("Install FFmpeg error:", e);
      throw e;
    } finally {
      isInstalling.value = false;
    }
  }

  // 显示 FFmpeg 安装引导弹窗
  function showFfmpegGuideDialog() {
    showFfmpegGuide.value = true;
  }

  // 初始化 FFmpeg
  async function initFfmpeg() {
    loading.value = true;
    try {
      // 先检测显卡类型
      await detectGpu();

      console.log("Initializing FFmpeg...");
      try {
        ffmpegInfo.value = await invoke<FfmpegInfo>("get_ffmpeg_info");
        console.log("FFmpeg info:", ffmpegInfo.value);
      } catch (e) {
        // FFmpeg 未安装
        console.log("FFmpeg not found:", e);
        ffmpegInfo.value = null;
      }

      // 如果 FFmpeg 未安装，显示引导弹窗
      if (!ffmpegInfo.value?.available) {
        await checkWinget();
        showFfmpegGuide.value = true;
      }

      encoders.value = await invoke<Encoder[]>("get_available_encoders");
      console.log("Encoders:", encoders.value);

      // 如果没有GPU编码器，添加软件编码器作为备选
      if (encoders.value.length === 0) {
        encoders.value = [
          { name: "libx264", type: "video", hardware: "software", description: "H.264 (AVC) software encoder" },
          { name: "libx265", type: "video", hardware: "software", description: "H.265 (HEVC) software encoder" }
        ];
      }

      // 自动选择 GPU 编码器（优先选择h264，兼容性更好）
      const gpuEncoder = encoders.value.find(
        e => (e.hardware === "qsv" || e.hardware === "nvenc" || e.hardware === "amf") &&
             (e.name.includes("h264") || e.name.includes("avc"))
      );
      const fallbackEncoder = encoders.value.find(e => e.type === "video" && e.name.includes("h264"));
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
      status: "pending" as const,
      elapsedTime: 0,
      estimatedRemainingTime: 0
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
    showFfmpegGuide,
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
    showFfmpegGuideDialog,
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
