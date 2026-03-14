// FFmpeg 信息
export interface FfmpegInfo {
  version: string;
  path: string;
  available: boolean;
}

// 编码器信息
export interface Encoder {
  name: string;
  type: "video" | "audio";
  hardware: "nvenc" | "qsv" | "amf" | "videotoolbox" | "software";
  description: string;
}

// 视频信息
export interface VideoInfo {
  path: string;
  filename: string;
  duration: number;
  width: number;
  height: number;
  fps: number;
  bitrate: number;
  codec: string;
  codecLong: string;
  size: number;
  audioCodec?: string;
  audioBitrate?: number;
}

// 压缩预设
export interface CompressPreset {
  id: string;
  name: string;
  description: string;
  encoder: string;
  bitrate: number;
  preset: string;
  estimatedSize: number;
  estimatedTime: number;
}

// 压缩选项
export interface CompressOptions {
  encoder: string;
  bitrate: number;
  crf?: number;
  preset: string;
  outputFormat: string;
  audioBitrate: number;
  scale?: number;
  fps?: number;
}

// 压缩任务
export interface CompressJob {
  inputPath: string;
  outputPath: string;
  options: CompressOptions;
}

// 压缩进度
export interface CompressProgress {
  jobId: string;
  filename: string;
  progress: number;
  status: "pending" | "running" | "completed" | "failed";
  error?: string;
  outputPath?: string;
  elapsedTime: number; // 已使用时间（秒）
  estimatedRemainingTime: number; // 预估剩余时间（秒）
}

// 预估结果
export interface EstimateResult {
  estimatedSize: number;
  estimatedTime: number;
  bitrate: number;
}

// 剪切选项
export interface TrimOptions {
  startTime: number;
  endTime: number;
  outputPath: string;
}
