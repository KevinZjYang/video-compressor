use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Window};

use crate::ffmpeg::get_ffmpeg_path;

/// 跨平台创建隐藏窗口的命令
#[cfg(windows)]
fn create_hidden_command(program: &str) -> Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut cmd = Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
fn create_hidden_command(program: &str) -> Command {
    Command::new(program)
}

/// 获取视频时长（秒）
fn get_video_duration(path: &str) -> Option<f64> {
    let ffprobe_path = crate::analyzer::get_ffprobe_path().ok()?;

    let output = create_hidden_command(ffprobe_path.to_str().unwrap_or(""))
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            path
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&json_str).ok()?;

    let duration = json.get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|d| d.parse::<f64>().ok())?;

    Some(duration)
}

/// 压缩任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressJob {
    #[serde(rename = "inputPath")]
    pub input_path: String,
    #[serde(rename = "outputPath")]
    pub output_path: String,
    pub options: CompressOptions,
}

/// 压缩选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressOptions {
    pub encoder: String,
    pub bitrate: u64,
    pub crf: Option<u32>,
    pub preset: String,
    #[serde(rename = "outputFormat")]
    pub output_format: String,
    #[serde(rename = "audioBitrate")]
    pub audio_bitrate: u64,
    pub scale: Option<u32>,
    pub fps: Option<u32>,
}

/// 预估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateResult {
    #[serde(rename = "estimatedSize")]
    pub estimated_size: u64,
    #[serde(rename = "estimatedTime")]
    pub estimated_time: f64,
    pub bitrate: u64,
}

// 全局停止标志
static STOP_FLAG: AtomicBool = AtomicBool::new(false);

/// 停止压缩
#[tauri::command]
pub fn stop_compress() {
    STOP_FLAG.store(true, Ordering::SeqCst);
    // 在 Windows 上使用 taskkill 杀死所有 ffmpeg 进程
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "ffmpeg.exe"])
            .output();
    }
}

/// 获取预估
#[tauri::command]
pub fn get_estimate(
    input_path: String,
    options: CompressOptions,
) -> Result<EstimateResult, String> {
    let ffprobe_path = crate::analyzer::get_ffprobe_path().map_err(|e| e)?;
    let output = create_hidden_command(ffprobe_path.to_str().unwrap_or(""))
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            &input_path
        ])
        .output()
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| e.to_string())?;

    let duration = json.get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(60.0);

    let bitrate = options.bitrate;
    let estimated_size = (bitrate as f64 * duration / 8.0) as u64;
    let estimated_time = duration / 12.0; // GPU 约 12x

    Ok(EstimateResult {
        estimated_size,
        estimated_time,
        bitrate,
    })
}

/// 执行压缩
#[tauri::command]
pub async fn compress_videos(
    jobs: Vec<CompressJob>,
    options: CompressOptions,
    window: Window,
) -> Result<(), String> {
    STOP_FLAG.store(false, Ordering::SeqCst);

    let ffmpeg_path = get_ffmpeg_path().ok_or("FFmpeg not found")?;
    let total = jobs.len();

    for (index, job) in jobs.iter().enumerate() {
        if STOP_FLAG.load(Ordering::SeqCst) {
            let _ = window.emit("compress-progress", serde_json::json!({
                "jobId": format!("job-{}", index),
                "filename": job.input_path,
                "progress": 0,
                "status": "cancelled"
            }));
            break;
        }

        let filename = Path::new(&job.input_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // 发送开始事件
        let _ = window.emit("compress-progress", serde_json::json!({
            "jobId": format!("job-{}", index),
            "filename": filename,
            "progress": 0,
            "status": "running",
            "elapsedTime": 0,
            "estimatedRemainingTime": 0
        }));

        // 先获取视频时长用于计算进度
        let video_duration = get_video_duration(&job.input_path).unwrap_or(60.0);

        // 构建 FFmpeg 命令
        let mut cmd = vec![
            "-i".to_string(),
            job.input_path.clone(),
            "-y".to_string(), // 覆盖输出文件
        ];

        // 视频编码器 - 根据用户选择的编码器使用对应的GPU编码
        let encoder_name = options.encoder.clone();
        if encoder_name.contains("qsv") {
            // Intel QSV: h264_qsv 或 hevc_qsv
            let video_encoder = if encoder_name.contains("hevc") || encoder_name.contains("h265") {
                "hevc_qsv"
            } else if encoder_name.contains("av1") {
                "av1_qsv"
            } else {
                "h264_qsv"
            };
            cmd.extend(["-c:v".to_string(), video_encoder.to_string()]);
            cmd.extend(["-b:v".to_string(), format!("{}k", options.bitrate / 1000)]);
        } else if encoder_name.contains("nvenc") {
            // NVIDIA NVENC: h264_nvenc 或 hevc_nvenc
            let video_encoder = if encoder_name.contains("hevc") || encoder_name.contains("h265") {
                "hevc_nvenc"
            } else if encoder_name.contains("av1") {
                "av1_nvenc"
            } else {
                "h264_nvenc"
            };
            cmd.extend(["-c:v".to_string(), video_encoder.to_string()]);
            cmd.extend(["-b:v".to_string(), format!("{}k", options.bitrate / 1000)]);
        } else if encoder_name.contains("amf") {
            // AMD AMF: h264_amf 或 hevc_amf
            let video_encoder = if encoder_name.contains("hevc") || encoder_name.contains("h265") {
                "hevc_amf"
            } else if encoder_name.contains("av1") {
                "av1_amf"
            } else {
                "h264_amf"
            };
            cmd.extend(["-c:v".to_string(), video_encoder.to_string()]);
            cmd.extend(["-b:v".to_string(), format!("{}k", options.bitrate / 1000)]);
        } else {
            // 软件编码
            let video_encoder = if encoder_name.contains("hevc") || encoder_name.contains("h265") {
                "libx265"
            } else if encoder_name.contains("av1") {
                "libaom-av1"
            } else {
                "libx264"
            };
            cmd.extend(["-c:v".to_string(), video_encoder.to_string()]);
            cmd.extend(["-b:v".to_string(), format!("{}k", options.bitrate / 1000)]);
        }

        // 编码预设
        cmd.extend(["-preset".to_string(), options.preset.clone()]);

        // 音频直接复制或重新编码
        cmd.extend(["-c:a".to_string(), "aac".to_string()]);
        cmd.extend(["-b:a".to_string(), format!("{}k", options.audio_bitrate / 1000)]);

        // 添加进度输出参数
        cmd.extend(["-progress".to_string(), "pipe:1".to_string()]);

        // 输出文件
        cmd.push(job.output_path.clone());

        // 执行压缩
        let mut child = create_hidden_command(ffmpeg_path.to_str().unwrap_or(""))
            .args(&cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start ffmpeg: {}", e))?;

        // 读取进度输出
        let start_time = std::time::Instant::now();
        if let Some(stdout) = child.stdout.take() {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stdout);
            let mut current_time_ms: u64 = 0;
            let mut progress_interval = std::time::Duration::from_millis(500); // 每500ms更新一次

            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.starts_with("out_time_ms=") {
                        if let Ok(time_ms) = line.trim_start_matches("out_time_ms=").parse::<u64>() {
                            current_time_ms = time_ms;

                            // 计算进度百分比
                            let current_seconds = current_time_ms as f64 / 1_000_000.0;
                            let progress = if video_duration > 0.0 {
                                ((current_seconds / video_duration) * 100.0).min(99.0) as u32
                            } else {
                                0
                            };

                            // 计算已使用时间和预估剩余时间
                            let elapsed = start_time.elapsed().as_secs_f64();
                            let estimated_total = if progress > 0 {
                                elapsed / (progress as f64 / 100.0)
                            } else {
                                0.0
                            };
                            let remaining = (estimated_total - elapsed).max(0.0);

                            // 发送进度事件
                            let _ = window.emit("compress-progress", serde_json::json!({
                                "jobId": format!("job-{}", index),
                                "filename": filename,
                                "progress": progress,
                                "status": "running",
                                "elapsedTime": elapsed,
                                "estimatedRemainingTime": remaining
                            }));
                        }
                    }
                }
            }
        }

        // 等待完成
        let status = child.wait().map_err(|e| e.to_string())?;

        // 清除当前子进程
        let progress = if status.success() { 100 } else { 0 };
        // 如果是被取消的，状态为 cancelled
        let result_status = if STOP_FLAG.load(Ordering::SeqCst) {
            "cancelled"
        } else if status.success() {
            "completed"
        } else {
            "failed"
        };
        let elapsed = start_time.elapsed().as_secs_f64();

        // 发送完成事件
        let _ = window.emit("compress-progress", serde_json::json!({
            "jobId": format!("job-{}", index),
            "filename": filename,
            "progress": progress,
            "status": result_status,
            "outputPath": job.output_path,
            "elapsedTime": elapsed,
            "estimatedRemainingTime": 0
        }));
    }

    Ok(())
}

/// 剪切视频
#[tauri::command]
pub async fn trim_video(
    input_path: String,
    start_time: f64,
    end_time: f64,
    output_path: String,
    window: Window,
) -> Result<String, String> {
    let ffmpeg_path = get_ffmpeg_path().ok_or("FFmpeg not found")?;

    let start_str = format_time(start_time);
    let duration_str = format_time(end_time - start_time);

    let _ = window.emit("trim-progress", serde_json::json!({
        "status": "running",
        "progress": 0
    }));

    let output = create_hidden_command(ffmpeg_path.to_str().unwrap_or(""))
        .args([
            "-i", &input_path,
            "-ss", &start_str,
            "-t", &duration_str,
            "-c", "copy",
            "-y",
            &output_path,
        ])
        .output()
        .map_err(|e| format!("Failed to trim: {}", e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Trim failed: {}", error));
    }

    let _ = window.emit("trim-progress", serde_json::json!({
        "status": "completed",
        "progress": 100
    }));

    Ok(output_path)
}

fn format_time(seconds: f64) -> String {
    let hours = (seconds / 3600.0).floor() as u32;
    let minutes = ((seconds % 3600.0) / 60.0).floor() as u32;
    let secs = seconds % 60.0;
    format!("{:02}:{:02}:{:06.3}", hours, minutes, secs)
}
