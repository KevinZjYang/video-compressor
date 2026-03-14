use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::ffmpeg::get_ffmpeg_path;

/// 编码器类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HardwareType {
    Nvenc,         // NVIDIA
    Qsv,           // Intel
    Amf,           // AMD
    Videotoolbox, // macOS
    Software,
}

/// 编码器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encoder {
    pub name: String,
    #[serde(rename = "type")]
    pub encoder_type: String,
    pub hardware: String,
    pub description: String,
}

impl HardwareType {
    fn from_name(name: &str) -> Self {
        let name_lower = name.to_lowercase();
        if name_lower.contains("nvenc") {
            HardwareType::Nvenc
        } else if name_lower.contains("qsv") {
            HardwareType::Qsv
        } else if name_lower.contains("amf") {
            HardwareType::Amf
        } else if name_lower.contains("videotoolbox") {
            HardwareType::Videotoolbox
        } else {
            HardwareType::Software
        }
    }
}

/// 获取可用的编码器
#[tauri::command]
pub fn get_available_encoders() -> Result<Vec<Encoder>, String> {
    let ffmpeg_path = get_ffmpeg_path().ok_or("FFmpeg not found")?;

    let output = Command::new(&ffmpeg_path)
        .arg("-encoders")
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("Failed to get encoders".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut encoders = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        // 视频编码器行以 "V" 开头，如 "V....D h264_nvenc"
        // 音频编码器行以 "A" 开头
        if !line.starts_with('V') {
            continue;
        }

        // 解析编码器行
        // 格式: V....D h264_nvenc           NVIDIA NVENC H.264 encoder (codec h264)
        // 先跳过 flags 部分 (如 "V....D")
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let name = parts[1];
        let description = parts[2..].join(" ");

        // 判断是视频还是音频编码 (已经确定是V开头，所以是视频)
        let encoder_type = "video";

        let hardware = HardwareType::from_name(name);

        // 只返回视频编码器
        if encoder_type == "video" {
            encoders.push(Encoder {
                name: name.to_string(),
                encoder_type: encoder_type.to_string(),
                hardware: match hardware {
                    HardwareType::Nvenc => "nvenc".to_string(),
                    HardwareType::Qsv => "qsv".to_string(),
                    HardwareType::Amf => "amf".to_string(),
                    HardwareType::Videotoolbox => "videotoolbox".to_string(),
                    HardwareType::Software => "software".to_string(),
                },
                description: description.trim().to_string(),
            });
        }
    }

    Ok(encoders)
}
