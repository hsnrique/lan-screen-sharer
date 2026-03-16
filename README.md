# LAN Screen Sharer

Real-time screen streaming from Mac to Windows over LAN using **H.264** via FFmpeg + UDP.

## Requirements

- **Mac (Sender)**: FFmpeg (`brew install ffmpeg`) + Rust
- **Windows (Receiver)**: FFmpeg (`winget install FFmpeg`) + Rust

## Quick Start

**1. Windows — start receiver first (it just listens):**
```
receive.bat
```

**2. Mac — start sender, pointing to your Windows IP:**
```bash
chmod +x send.sh
./send.sh <windows-ip>
```

Example: `./send.sh 192.168.100.50`

## Options

```bash
# Mac: custom FPS and bitrate
./send.sh 192.168.100.50 --fps 60 --bitrate 6M

# Windows: custom port
receive.bat --port 9000
```

| Flag | Default | Description |
|------|---------|-------------|
| `--fps` | 30 | Target framerate (sender) |
| `--bitrate` | 5M | Video bitrate (sender) |
| `--port` | 8765 | UDP port (both) |

## How It Works

1. Receiver opens a UDP port and waits for data
2. Sender captures Mac screen via FFmpeg `avfoundation`
3. Encodes with H.264 (`ultrafast/zerolatency`) and pushes via UDP
4. Receiver displays with FFplay (low-latency settings)