use serde::{Deserialize, Serialize};

use crate::analyzer::VideoInfo;
use crate::encoder::Encoder;

/// 压缩预设
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub encoder: String,
    pub bitrate: u64,
    pub preset: String,
    #[serde(rename = "estimatedSize")]
    pub estimated_size: u64,
    #[serde(rename = "estimatedTime")]
    pub estimated_time: f64,
}

/// 获取GPU编码器
fn get_gpu_encoder(gpu_type: &str) -> String {
    match gpu_type {
        "intel" => "hevc_qsv".to_string(),
        "nvidia" => "hevc_nvenc".to_string(),
        "amd" => "hevc_amf".to_string(),
        _ => "libx264".to_string(),
    }
}

/// 获取推荐预设
#[tauri::command]
pub fn get_compress_presets(
    video_info: VideoInfo,
    gpu_type: Option<String>,
) -> Result<Vec<CompressPreset>, String> {
    // 根据分辨率和码率确定预设
    let (width, height) = (video_info.width, video_info.height);
    let bitrate = video_info.bitrate;

    let is_4k = width >= 3840 || height >= 2160;
    let is_1080p = width >= 1920 || height >= 1080;
    let is_720p = width >= 1280 || height >= 720;

    // 根据分辨率和原码率推荐预设
    let presets = match (is_4k, is_1080p, is_720p) {
        (true, _, _) if bitrate > 50_000_000 => vec![
            ("高质量", 20_000_000, "保持较高画质"),
            ("均衡", 12_000_000, "画质与体积平衡"),
            ("低体积", 6_000_000, "最小体积"),
        ],
        (true, _, _) => vec![
            ("高质量", 12_000_000, "保持较高画质"),
            ("均衡", 8_000_000, "画质与体积平衡"),
            ("低体积", 4_000_000, "最小体积"),
        ],
        (_, true, _) => vec![
            ("高质量", 10_000_000, "保持较高画质"),
            ("均衡", 6_000_000, "画质与体积平衡"),
            ("低体积", 3_000_000, "最小体积"),
        ],
        (_, _, true) => vec![
            ("高质量", 5_000_000, "保持较高画质"),
            ("均衡", 2_000_000, "画质与体积平衡"),
            ("低体积", 1_000_000, "最小体积"),
        ],
        _ => vec![
            ("高质量", 4_000_000, "保持较高画质"),
            ("均衡", 2_000_000, "画质与体积平衡"),
            ("低体积", 1_000_000, "最小体积"),
        ],
    };

    let duration = video_info.duration;

    // 根据GPU类型选择编码器
    let gpu = gpu_type.unwrap_or_else(|| "unknown".to_string());
    let encoder = get_gpu_encoder(&gpu);

    Ok(presets
        .iter()
        .enumerate()
        .map(|(i, (name, bitrate, desc))| {
            let estimated_size = (bitrate * duration as u64) / 8;
            // 预估时间：GPU 约 10-15x 实时
            let estimated_time = duration / 12.0;

            CompressPreset {
                id: format!("preset-{}", i),
                name: name.to_string(),
                description: desc.to_string(),
                encoder: encoder.clone(),
                bitrate: *bitrate,
                preset: "medium".to_string(),
                estimated_size,
                estimated_time,
            }
        })
        .collect())
}
