use local_ip_address::local_ip;
use std::env;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_FPS: u32 = 30;
const DEFAULT_BITRATE: &str = "4M";
const PORT: u16 = 8765;

fn main() {
    let args: Vec<String> = env::args().collect();
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

    print_banner(fps, &bitrate, port);

    while running.load(Ordering::SeqCst) {
        println!("  Waiting for receiver to connect...");

        let mut child = start_ffmpeg(fps, &bitrate, port);

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
                    println!("  Receiver disconnected.");
                }
            }
            println!("  Restarting in 2 seconds...");
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    println!("  Shutting down.");
}

fn start_ffmpeg(fps: u32, bitrate: &str, port: u16) -> Child {
    Command::new("ffmpeg")
        .args([
            "-f", "avfoundation",
            "-pixel_format", "uyvy422",
            "-probesize", "5000000",
            "-framerate", &fps.to_string(),
            "-capture_cursor", "1",
            "-i", "1:none",
            "-c:v", "libx264",
            "-preset", "ultrafast",
            "-tune", "zerolatency",
            "-pix_fmt", "yuv420p",
            "-b:v", bitrate,
            "-maxrate", bitrate,
            "-bufsize", "1M",
            "-g", &fps.to_string(),
            "-f", "mpegts",
            &format!("tcp://0.0.0.0:{}?listen", port),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to start FFmpeg")
}

fn print_banner(fps: u32, bitrate: &str, port: u16) {
    println!("===========================================");
    println!("  Screen Sender v2 (H.264)");
    println!("===========================================");
    println!("  Codec:   H.264 (ultrafast/zerolatency)");
    println!("  FPS:     {}", fps);
    println!("  Bitrate: {}", bitrate);
    println!("  Port:    {}", port);
    if let Ok(ip) = local_ip() {
        println!("  Address: {}:{}", ip, port);
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
