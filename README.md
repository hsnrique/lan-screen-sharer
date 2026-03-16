# LAN Screen Sharer

Real-time screen streaming from Mac to Windows over LAN using **H.264** via FFmpeg.

## Requirements

- **Mac (Sender)**: FFmpeg (`brew install ffmpeg`) + Rust
- **Windows (Receiver)**: FFmpeg (`winget install FFmpeg`) + Rust

## Quick Start

**Mac (send your screen):**
```bash
chmod +x send.sh
./send.sh
```

**Windows (receive the stream):**
```
receive.bat 192.168.100.99
```

That's it. The scripts auto-build and run.

## Options

Both scripts forward arguments to the underlying app:

```bash
# Mac: custom FPS and bitrate
./send.sh --fps 60 --bitrate 6M

# Windows: custom port
receive.bat 192.168.1.42 --port 9000
```

| Flag | Default | Description |
|------|---------|-------------|
| `--fps` | 30 | Target framerate (sender) |
| `--bitrate` | 4M | Video bitrate (sender) |
| `--port` | 8765 | TCP port (both) |

## How It Works

1. Sender captures the Mac screen via FFmpeg `avfoundation`
2. Encodes with H.264 (`ultrafast` preset, `zerolatency` tune)
3. Streams over TCP as MPEG-TS
4. Receiver displays via FFplay with low-latency flags