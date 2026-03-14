# FFmpeg 资源文件

本项目需要 FFmpeg 才能运行视频压缩功能。

## 下载 FFmpeg

1. 访问 FFmpeg 官网下载: https://ffmpeg.org/download.html
2. 或者直接下载 Windows 构建版本:
   - https://github.com/BtbN/FFmpeg-Builds/releases

## 安装步骤

1. 解压下载的 FFmpeg 压缩包
2. 将以下文件复制到此目录:
   - `ffmpeg.exe`
   - `ffprobe.exe`

## 文件路径

```
resources/
└── ffmpeg/
    ├── ffmpeg.exe    # 必须
    └── ffprobe.exe   # 必须
```

## 替代方案

如果不复制 FFmpeg 到这里，应用会尝试从系统 PATH 中查找 FFmpeg。请确保 ffmpeg 和 ffprobe 命令可以在命令行中运行。
