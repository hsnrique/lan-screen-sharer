use local_ip_address::local_ip;
use std::env;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_FPS: u32 = 30;
const DEFAULT_BITRATE: &str = "5M";
const PORT: u16 = 8765;

fn main() {
    let args: Vec<String> = env::args().collect();
    let receiver_ip = args.get(1).cloned().unwrap_or_else(|| {
        eprintln!("Usage: screen-sender <receiver-ip> [OPTIONS]");
        eprintln!("Example: screen-sender 192.168.1.100");
        eprintln!("Options:");
        eprintln!("  --fps N       Target FPS (default: 30)");
        eprintln!("  --bitrate N   Video bitrate (default: 5M)");
        eprintln!("  --port N      UDP port (default: 8765)");
        std::process::exit(1);
    });

    let fps = parse_arg(&args, "--fps").unwrap_or(DEFAULT_FPS);
    let bitrate = parse_str_arg(&args, "--bitrate").unwrap_or(DEFAULT_BITRATE.into());
    let port = parse_arg(&args, "--port").unwrap_or(PORT);

    if !is_ffmpeg_installed() {
        eprintln!("ERROR: FFmpeg not found.");
        eprintln!("Install with: brew install ffmpeg");
        std::process::exit(1);
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))
        .expect("Failed to set Ctrl+C handler");

    print_banner(&receiver_ip, fps, &bitrate, port);

    while running.load(Ordering::SeqCst) {
        println!("  Streaming to {}:{}...", receiver_ip, port);

        let mut child = start_ffmpeg(&receiver_ip, fps, &bitrate, port);
        let status = child.wait();

        if running.load(Ordering::SeqCst) {
            match status {
                Ok(s) if !s.success() => {
                    eprintln!("  FFmpeg exited with: {}", s);
                }
                Err(e) => {
                    eprintln!("  FFmpeg error: {}", e);
                }
                _ => {
                    println!("  Stream ended.");
                }
            }
            println!("  Restarting in 2 seconds...");
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    println!("  Shutting down.");
}

fn start_ffmpeg(receiver_ip: &str, fps: u32, bitrate: &str, port: u16) -> Child {
    let fps_str = fps.to_string();
    let dest = format!("udp://{}:{}?pkt_size=1316", receiver_ip, port);

    Command::new("ffmpeg")
        .args([
            "-f", "avfoundation",
            "-pixel_format", "uyvy422",
            "-framerate", &fps_str,
            "-capture_cursor", "1",
            "-i", "1:none",
            "-c:v", "libx264",
            "-preset", "ultrafast",
            "-tune", "zerolatency",
            "-pix_fmt", "yuv420p",
            "-b:v", bitrate,
            "-maxrate", bitrate,
            "-bufsize", "512k",
            "-g", &fps_str,
            "-f", "mpegts",
            &dest,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to start FFmpeg")
}

fn print_banner(receiver_ip: &str, fps: u32, bitrate: &str, port: u16) {
    println!("===========================================");
    println!("  Screen Sender v2 (H.264 / UDP)");
    println!("===========================================");
    println!("  Codec:    H.264 ultrafast/zerolatency");
    println!("  FPS:      {}", fps);
    println!("  Bitrate:  {}", bitrate);
    println!("  Target:   {}:{}", receiver_ip, port);
    if let Ok(ip) = local_ip() {
        println!("  Local IP: {}", ip);
    }
    println!("===========================================");
}

fn is_ffmpeg_installed() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn parse_arg<T: std::str::FromStr>(args: &[String], flag: &str) -> Option<T> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
}

fn parse_str_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
