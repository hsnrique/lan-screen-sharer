use std::env;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const DEFAULT_PORT: u16 = 8765;

fn main() {
    let args: Vec<String> = env::args().collect();
    let port = parse_arg(&args, "--port").unwrap_or(DEFAULT_PORT);

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
    println!("  Screen Receiver v2 (H.264 / UDP)");
    println!("===========================================");
    println!("  Listening on UDP port: {}", port);
    println!("  Waiting for stream...");
    println!("===========================================");

    let mut child = start_ffplay(port);
    let _ = child.wait();

    println!("  Shutting down.");
}

fn start_ffplay(port: u16) -> Child {
    Command::new("ffplay")
        .args([
            "-loglevel", "warning",
            "-f", "mpegts",
            "-fflags", "nobuffer+discardcorrupt",
            "-flags", "low_delay",
            "-avioflags", "direct",
            "-framedrop",
            "-analyzeduration", "0",
            "-probesize", "32768",
            "-sync", "ext",
            "-vf", "setpts=0",
            "-window_title", "Screen Viewer",
            &format!("udp://0.0.0.0:{}?overrun_nonfatal=1&fifo_size=50000000", port),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
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
