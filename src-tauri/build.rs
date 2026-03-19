use std::fs;
use std::path::Path;

fn main() {
    // 开发模式下复制 FFmpeg 到 target/debug/resources/
    #[cfg(debug_assertions)]
    {
        let source = Path::new("resources/ffmpeg");
        let debug_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let target_dir = debug_dir.parent().unwrap().join("target").join("debug");
        let dest = target_dir.join("resources").join("ffmpeg");

        if source.exists() && !dest.exists() {
            println!("Copying FFmpeg resources to debug directory...");
            copy_dir_recursive(source, &dest).ok();
        }
    }

    tauri_build::build()
}

#[cfg(debug_assertions)]
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_file() {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}