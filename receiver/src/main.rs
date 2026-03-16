use std::env;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_PORT: u16 = 8765;

fn main() {
    let args: Vec<String> = env::args().collect();
    let host = args.get(1).cloned().unwrap_or_else(|| {
        eprintln!("Usage: screen-receiver <mac-ip> [--port PORT]");
        eprintln!("Example: screen-receiver 192.168.1.42");
        std::process::exit(1);
    });
    let port = parse_arg(&args, "--port").unwrap_or(DEFAULT_PORT);

    let addr = if host.contains(':') {
        host.clone()
    } else {
        format!("{}:{}", host, port)
    };

    if !is_ffplay_installed() {
        eprintln!("ERROR: FFplay not found.");
        eprintln!("Install FFmpeg (includes FFplay):");
        eprintln!("  winget install FFmpeg");
        std::process::exit(1);
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))
        .expect("Failed to set Ctrl+C handler");

    println!("===========================================");
    println!("  Screen Receiver v2 (H.264)");
    println!("===========================================");
    println!("  Connecting to: {}", addr);
    println!("===========================================");

    while running.load(Ordering::SeqCst) {
        println!("  Starting stream viewer...");

        let mut child = start_ffplay(&addr);
        let status = child.wait();

        if running.load(Ordering::SeqCst) {
            match status {
                Ok(s) if !s.success() => {
                    eprintln!("  FFplay exited with: {}", s);
                }
                Err(e) => {
                    eprintln!("  FFplay error: {}", e);
                }
                _ => {
                    println!("  Stream ended.");
                }
            }
            println!("  Reconnecting in 3 seconds...");
            std::thread::sleep(Duration::from_secs(3));
        }
    }

    println!("  Shutting down.");
}

fn start_ffplay(addr: &str) -> Child {
    Command::new("ffplay")
        .args([
            "-fflags", "nobuffer",
            "-flags", "low_delay",
            "-framedrop",
            "-analyzeduration", "0",
            "-probesize", "32",
            "-sync", "ext",
            "-window_title", "Screen Viewer",
            &format!("tcp://{}", addr),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start FFplay")
}

fn is_ffplay_installed() -> bool {
    Command::new("ffplay")
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
