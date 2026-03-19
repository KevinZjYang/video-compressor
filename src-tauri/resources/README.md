# FFmpeg 资源文件

本目录用于存放 FFmpeg 二进制文件，打包时会自动包含在应用中。

## 下载 FFmpeg

推荐使用 Gyan.dev 的 essentials 版本（约 80MB），已包含所有 GPU 编码器支持：

1. 访问 https://www.gyan.dev/ffmpeg/builds/
2. 下载 `ffmpeg-git-essentials.7z`
3. 解压后将 `ffmpeg.exe` 和 `ffprobe.exe` 复制到此目录

## 文件结构

```
resources/
└── ffmpeg/
    ├── ffmpeg.exe    # 必须
    └── ffprobe.exe   # 必须
```

## 包含的硬件编码器

- **NVIDIA NVENC**: `h264_nvenc`, `hevc_nvenc`, `av1_nvenc`
- **Intel QSV**: `h264_qsv`, `hevc_qsv`, `av1_qsv`
- **AMD AMF**: `h264_amf`, `hevc_amf`, `av1_amf`
- **软件编码**: `libx264`, `libx265`, `libaom-av1`

## 注意事项

- 如果此目录已有 FFmpeg，应用会优先使用内置版本
- 用户仍可使用自己安装的 FFmpeg（系统 PATH 中的版本会被用作备选）
