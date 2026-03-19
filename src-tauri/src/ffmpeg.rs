use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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

/// FFmpeg 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfmpegInfo {
    pub version: String,
    pub path: String,
    pub available: bool,
}

/// 获取 FFmpeg 路径
pub fn get_ffmpeg_path() -> Option<PathBuf> {
    // 1. 优先检查打包资源目录 (生产环境)
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(dir) = exe_dir.parent() {
            let ffmpeg_path = dir.join("resources").join("ffmpeg").join("ffmpeg.exe");
            if ffmpeg_path.exists() {
                return Some(ffmpeg_path);
            }
            // 尝试多级父目录
            if let Some(parent) = dir.parent() {
                let ffmpeg_path2 = parent.join("resources").join("ffmpeg").join("ffmpeg.exe");
                if ffmpeg_path2.exists() {
                    return Some(ffmpeg_path2);
                }
            }
        }
    }

    // 2. 检查系统 PATH - 直接运行 ffmpeg 看是否可用
    if let Ok(output) = create_hidden_command("ffmpeg").arg("-version").output() {
        if output.status.success() {
            // ffmpeg 在 PATH 中，使用 which/where 获取路径
            if let Ok(path_output) = create_hidden_command("where").arg("ffmpeg").output() {
                if path_output.status.success() {
                    let path_str = String::from_utf8_lossy(&path_output.stdout);
                    if let Some(first) = path_str.lines().next() {
                        return Some(PathBuf::from(first.trim()));
                    }
                }
            }
            // 如果 where 失败，返回默认命令
            return Some(PathBuf::from("ffmpeg"));
        }
    }

    // 3. 使用 where 命令查找
    if let Ok(output) = create_hidden_command("where").arg("ffmpeg").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout);
            if let Some(first) = path.lines().next() {
                let p = PathBuf::from(first.trim());
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }

    // 4. 检查 WinGet 默认安装目录
    let winget_path = std::env::var("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("Microsoft").join("WinGet").join("Links").join("ffmpeg.exe"))
        .ok();
    if let Some(p) = winget_path {
        if p.exists() {
            return Some(p);
        }
    }

    // 5. 检查开发模式资源目录 (src-tauri/resources/ffmpeg/)
    let current_dir = std::env::current_dir().ok();
    if let Some(dir) = current_dir {
        let ffmpeg_path = dir.join("src-tauri").join("resources").join("ffmpeg").join("ffmpeg.exe");
        if ffmpeg_path.exists() {
            return Some(ffmpeg_path);
        }
        // 也支持从 src-tauri/resources/ffmpeg 直接查找（相对路径）
        let ffmpeg_path2 = dir.join("resources").join("ffmpeg").join("ffmpeg.exe");
        if ffmpeg_path2.exists() {
            return Some(ffmpeg_path2);
        }
    }

    None
}

/// 获取 FFmpeg 版本
fn get_ffmpeg_version(path: &PathBuf) -> String {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    if let Ok(output) = Command::new(path)
        .creation_flags(CREATE_NO_WINDOW)
        .arg("-version")
        .output()
    {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            // 提取版本号，如 "ffmpeg version 6.1"
            if let Some(line) = version.lines().next() {
                if let Some(v) = line.split("version ").nth(1) {
                    return v.split_whitespace().next().unwrap_or("unknown").to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

/// 获取 FFmpeg 信息
#[tauri::command]
pub fn get_ffmpeg_info() -> Result<FfmpegInfo, String> {
    let path = get_ffmpeg_path().ok_or("FFmpeg not found")?;
    let version = get_ffmpeg_version(&path);

    Ok(FfmpegInfo {
        version,
        path: path.to_string_lossy().to_string(),
        available: true,
    })
}

/// 检查 winget 是否可用
#[tauri::command]
pub fn check_winget() -> bool {
    // 使用 PowerShell 检测 winget（不使用 create_hidden_command 以避免创建窗口标志冲突）
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-Command", "Get-Command winget -ErrorAction SilentlyContinue"])
        .output();

    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

/// 安装 FFmpeg
#[tauri::command]
pub async fn install_ffmpeg() -> Result<String, String> {
    // 检查是否已有打包的 FFmpeg
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(dir) = exe_dir.parent() {
            let bundled_ffmpeg = dir.join("resources").join("ffmpeg").join("ffmpeg.exe");
            if bundled_ffmpeg.exists() {
                return Ok("FFmpeg 已内置于应用中，无需额外安装".to_string());
            }
        }
    }

    // 使用 winget 安装 FFmpeg (仅当没有内置版本时)
    let output = create_hidden_command("winget")
        .args(["install", "-e", "--id", "Gyan.FFmpeg", "--accept-source-agreements", "--accept-package-agreements"])
        .output()
        .map_err(|e| format!("Failed to run winget: {}", e))?;

    if output.status.success() {
        Ok("FFmpeg installed successfully".to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to install FFmpeg: {}", error))
    }
}
