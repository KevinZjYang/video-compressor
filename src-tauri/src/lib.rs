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

/// 获取显卡名称（优先返回高性能显卡）
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
        let keywords = ["arc", "nvidia", "geforce", "rtx", "gtx", "radeon", "amd"];
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
            get_gpu_name
        ])
        .setup(|app| {
            log::info!("App setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
