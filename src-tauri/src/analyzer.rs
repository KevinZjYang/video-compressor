use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::Path;

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

/// 视频信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub path: String,
    pub filename: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub bitrate: u64,
    pub codec: String,
    #[serde(rename = "codecLong")]
    pub codec_long: String,
    pub size: u64,
    #[serde(rename = "audioCodec")]
    pub audio_codec: Option<String>,
    #[serde(rename = "audioBitrate")]
    pub audio_bitrate: Option<u64>,
}

/// 分析视频
#[tauri::command]
pub fn analyze_video(path: String) -> Result<VideoInfo, String> {
    let ffprobe_path = get_ffprobe_path()?;

    let output = create_hidden_command(ffprobe_path.to_str().unwrap_or(""))
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
            &path
        ])
        .output()
        .map_err(|e| format!("Failed to run ffprobe: {}", e))?;

    if !output.status.success() {
        return Err("ffprobe failed".to_string());
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let format = json.get("format").ok_or("No format info")?;
    let streams = json.get("streams").ok_or("No streams")?;

    // 查找视频流
    let video_stream = streams.as_array()
        .and_then(|arr| arr.iter().find(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("video")));

    let video = video_stream.ok_or("No video stream found")?;

    // 解析视频参数
    let width = video.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let height = video.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let codec = video.get("codec_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let codec_long = video.get("codec_long_name").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // 帧率
    let fps = parse_frame_rate(
        video.get("r_frame_rate").and_then(|v| v.as_str()).unwrap_or("0/1")
    );

    // 时长
    let duration = format.get("duration")
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);

    // 码率
    let bitrate = format.get("bit_rate")
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    // 文件大小
    let size = format.get("size")
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    // 音频信息
    let audio_stream = streams.as_array()
        .and_then(|arr| arr.iter().find(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("audio")));

    let audio_codec = audio_stream
        .and_then(|s| s.get("codec_name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 尝试从音频流获取码率，如果不存在则使用默认值
    let audio_bitrate = audio_stream
        .and_then(|s| s.get("bit_rate"))
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<u64>().ok())
        .or_else(|| {
            // 如果音频流中没有码率，根据编码格式返回默认值
            let default_bitrate = match audio_codec.as_deref() {
                Some("aac") | Some("ac3") | Some("eac3") => Some(128000),
                Some("mp3") => Some(192000),
                Some("opus") => Some(128000),
                Some("vorbis") => Some(128000),
                Some("flac") => Some(800000),
                _ => Some(128000),
            };
            default_bitrate
        });

    let filename = Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    Ok(VideoInfo {
        path,
        filename,
        duration,
        width,
        height,
        fps,
        bitrate,
        codec,
        codec_long,
        size,
        audio_codec,
        audio_bitrate,
    })
}

pub fn get_ffprobe_path() -> Result<std::path::PathBuf, String> {
    // 1. 检查资源目录
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(dir) = exe_dir.parent() {
            let ffprobe_path = dir.join("resources").join("ffmpeg").join("ffprobe.exe");
            if ffprobe_path.exists() {
                return Ok(ffprobe_path);
            }
        }
    }

    // 2. 尝试从 ffmpeg 路径推断
    if let Some(ffmpeg) = crate::ffmpeg::get_ffmpeg_path() {
        if let Some(parent) = ffmpeg.parent() {
            let ffprobe = parent.join("ffprobe.exe");
            if ffprobe.exists() {
                return Ok(ffprobe);
            }
        }
    }

    // 3. 检查系统 PATH
    if let Ok(output) = create_hidden_command("where").arg("ffprobe").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout);
            if let Some(first) = path.lines().next() {
                return Ok(std::path::PathBuf::from(first.trim()));
            }
        }
    }

    Err("ffprobe not found".to_string())
}

fn parse_frame_rate(rate: &str) -> f64 {
    let parts: Vec<&str> = rate.split('/').collect();
    if parts.len() == 2 {
        let num: f64 = parts[0].parse().unwrap_or(0.0);
        let den: f64 = parts[1].parse().unwrap_or(1.0);
        if den != 0.0 {
            return num / den;
        }
    }
    rate.parse().unwrap_or(0.0)
}
