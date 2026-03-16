use image::codecs::jpeg::JpegEncoder;
use image::ColorType;
use local_ip_address::local_ip;
use scrap::{Capturer, Display};
use std::env;
use std::io::Write;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_FPS: u32 = 30;
const DEFAULT_QUALITY: u8 = 92;
const PORT: u16 = 8765;

fn main() {
    let args: Vec<String> = env::args().collect();
    let fps = parse_arg(&args, "--fps").unwrap_or(DEFAULT_FPS);
    let quality = parse_arg::<u8>(&args, "--quality").unwrap_or(DEFAULT_QUALITY);

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))
        .expect("Failed to set Ctrl+C handler");

    let display = Display::primary().expect("Failed to find primary display");
    let width = display.width();
    let height = display.height();

    println!("===========================================");
    println!("  Screen Sender");
    println!("===========================================");
    println!("  Resolution: {}x{}", width, height);
    println!("  FPS: {} | Quality: {}%", fps, quality);

    if let Ok(ip) = local_ip() {
        println!("  Address: {}:{}", ip, PORT);
    }

    println!("===========================================");
    println!("  Waiting for connection...");

    let listener = TcpListener::bind(format!("0.0.0.0:{}", PORT))
        .expect("Failed to bind port");

    while running.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, addr)) => {
                println!("  Connected: {}", addr);
                handle_client(stream, width, height, fps, quality, &running);
                println!("  Disconnected: {}", addr);
                println!("  Waiting for connection...");
            }
            Err(e) => {
                eprintln!("Accept error: {}", e);
                break;
            }
        }
    }

    println!("Shutting down.");
}

fn handle_client(
    mut stream: std::net::TcpStream,
    width: usize,
    height: usize,
    fps: u32,
    quality: u8,
    running: &AtomicBool,
) {
    stream.set_nodelay(true).ok();

    let header = format!("{}x{}", width, height);
    let header_bytes = header.as_bytes();
    let header_len = header_bytes.len() as u32;

    if write_all_safe(&mut stream, &header_len.to_be_bytes()).is_err() {
        return;
    }
    if write_all_safe(&mut stream, header_bytes).is_err() {
        return;
    }

    let mut capturer = match Capturer::new(
        Display::primary().expect("Failed to find display"),
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create capturer: {}", e);
            return;
        }
    };

    let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
    let pixel_count = width * height;
    let mut rgb_buf = vec![0u8; pixel_count * 3];
    let mut jpeg_buf = Vec::with_capacity(pixel_count);
    let mut frame_count: u64 = 0;
    let mut fps_timer = Instant::now();

    while running.load(Ordering::SeqCst) {
        let frame_start = Instant::now();

        match capturer.frame() {
            Ok(frame) => {
                bgra_to_rgb(&frame, &mut rgb_buf, pixel_count);

                jpeg_buf.clear();
                let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buf, quality);

                if encoder
                    .encode(&rgb_buf, width as u32, height as u32, ColorType::Rgb8)
                    .is_err()
                {
                    continue;
                }

                let len = jpeg_buf.len() as u32;
                if write_all_safe(&mut stream, &len.to_be_bytes()).is_err() {
                    break;
                }
                if write_all_safe(&mut stream, &jpeg_buf).is_err() {
                    break;
                }

                frame_count += 1;
                if fps_timer.elapsed() >= Duration::from_secs(2) {
                    let real_fps = frame_count as f64 / fps_timer.elapsed().as_secs_f64();
                    println!("  Streaming: {:.1} FPS | Frame: {} KB", real_fps, jpeg_buf.len() / 1024);
                    frame_count = 0;
                    fps_timer = Instant::now();
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }
}

fn bgra_to_rgb(bgra: &[u8], rgb: &mut [u8], pixel_count: usize) {
    for i in 0..pixel_count {
        let src = i * 4;
        let dst = i * 3;
        rgb[dst] = bgra[src + 2];
        rgb[dst + 1] = bgra[src + 1];
        rgb[dst + 2] = bgra[src];
    }
}

fn write_all_safe(stream: &mut std::net::TcpStream, data: &[u8]) -> Result<(), ()> {
    stream.write_all(data).map_err(|_| ())
}

fn parse_arg<T: std::str::FromStr>(args: &[String], flag: &str) -> Option<T> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
}
