// Video Compressor - Rust Backend

mod ffmpeg;
mod encoder;
mod analyzer;
mod preset;
mod compressor;

use std::process::Command;

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

pub use ffmpeg::*;
pub use encoder::*;
pub use analyzer::*;
pub use preset::*;
pub use compressor::*;

/// 显卡信息结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub gpu_type: String, // "integrated" (核显) 或 "dedicated" (独显)
}

/// 获取所有显卡信息
#[tauri::command]
fn get_all_gpus() -> Result<Vec<GpuInfo>, String> {
    let output = create_hidden_command("powershell")
        .args(["-Command", "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name"])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("Failed to get GPU info".to_string());
    }

    let names = String::from_utf8_lossy(&output.stdout);
    let mut gpus = Vec::new();

    // 独显关键字
    let dedicated_keywords = ["nvidia", "geforce", "rtx", "gtx", "radeon", "amd", "arc"];
    // 核显关键字
    let integrated_keywords = ["intel", "uhd", "hd graphics", "iris"];

    for line in names.lines() {
        let name = line.trim();
        if name.is_empty() {
            continue;
        }
        let name_lower = name.to_lowercase();

        // 判断是独显还是核显
        let gpu_type = if dedicated_keywords.iter().any(|kw| name_lower.contains(kw)) {
            "dedicated"
        } else if integrated_keywords.iter().any(|kw| name_lower.contains(kw)) {
            "integrated"
        } else {
            continue; // 跳过未知类型的显卡
        };

        gpus.push(GpuInfo {
            name: name.to_string(),
            gpu_type: gpu_type.to_string(),
        });
    }

    Ok(gpus)
}

/// 获取显卡名称（优先返回高性能显卡，保持向后兼容）
#[tauri::command]
fn get_gpu_name() -> Result<String, String> {
    // 获取所有显卡，优先返回包含关键字的（Intel Arc, NVIDIA, AMD）
    let output = create_hidden_command("powershell")
        .args(["-Command", "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name"])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        let names = String::from_utf8_lossy(&output.stdout);
        // 查找最可能是高性能显卡的
        let keywords = ["arc", "nvidia", "geforce", "rtx", "gtx", "radeon", "amd", "intel"];
        for line in names.lines() {
            let line_lower = line.to_lowercase();
            for kw in &keywords {
                if line_lower.contains(kw) {
                    return Ok(line.trim().to_string());
                }
            }
        }
        // 如果没找到，返回第一个
        if let Some(first) = names.lines().next() {
            return Ok(first.trim().to_string());
        }
    }
    Ok("Unknown".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    log::info!("Starting Video Compressor...");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_ffmpeg_info,
            get_available_encoders,
            analyze_video,
            get_compress_presets,
            get_estimate,
            compress_videos,
            stop_compress,
            trim_video,
            install_ffmpeg,
            check_winget,
            get_gpu_name,
            get_all_gpus
        ])
        .setup(|app| {
            log::info!("App setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
