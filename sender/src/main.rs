use local_ip_address::local_ip;
use scrap::{Capturer, Display};
use std::env;
use std::io::Write;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use turbojpeg::{Compressor, Image, PixelFormat};

const DEFAULT_FPS: u32 = 30;
const DEFAULT_QUALITY: i32 = 60;
const DEFAULT_SCALE: f64 = 0.75;
const PORT: u16 = 8765;

fn main() {
    let args: Vec<String> = env::args().collect();
    let fps = parse_arg(&args, "--fps").unwrap_or(DEFAULT_FPS);
    let quality = parse_arg::<i32>(&args, "--quality").unwrap_or(DEFAULT_QUALITY);
    let scale: f64 = parse_arg(&args, "--scale").unwrap_or(DEFAULT_SCALE);

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))
        .expect("Failed to set Ctrl+C handler");

    let display = Display::primary().expect("Failed to find primary display");
    let capture_w = display.width();
    let capture_h = display.height();
    let send_w = (capture_w as f64 * scale) as usize;
    let send_h = (capture_h as f64 * scale) as usize;

    println!("===========================================");
    println!("  Screen Sender");
    println!("===========================================");
    println!("  Capture: {}x{}", capture_w, capture_h);
    println!("  Send:    {}x{} (scale {:.0}%)", send_w, send_h, scale * 100.0);
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
                handle_client(stream, capture_w, capture_h, send_w, send_h, fps, quality, &running);
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
    capture_w: usize,
    capture_h: usize,
    send_w: usize,
    send_h: usize,
    fps: u32,
    quality: i32,
    running: &AtomicBool,
) {
    stream.set_nodelay(true).ok();

    let header = format!("{}x{}", send_w, send_h);
    let header_bytes = header.as_bytes();

    if write_all_safe(&mut stream, &(header_bytes.len() as u32).to_be_bytes()).is_err() {
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

    let mut compressor = Compressor::new().expect("Failed to create JPEG compressor");
    let _ = compressor.set_quality(quality);

    let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
    let needs_scale = send_w != capture_w || send_h != capture_h;
    let mut rgb_buf = vec![0u8; capture_w * capture_h * 3];
    let mut scaled_buf = if needs_scale {
        vec![0u8; send_w * send_h * 3]
    } else {
        Vec::new()
    };
    let mut frame_count: u64 = 0;
    let mut fps_timer = Instant::now();

    while running.load(Ordering::SeqCst) {
        let frame_start = Instant::now();

        match capturer.frame() {
            Ok(frame) => {
                bgra_to_rgb(&frame, &mut rgb_buf, capture_w * capture_h);

                let (encode_buf, encode_w, encode_h) = if needs_scale {
                    scale_rgb(&rgb_buf, capture_w, capture_h, &mut scaled_buf, send_w, send_h);
                    (scaled_buf.as_slice(), send_w, send_h)
                } else {
                    (rgb_buf.as_slice(), capture_w, capture_h)
                };

                let image = Image {
                    pixels: encode_buf,
                    width: encode_w,
                    pitch: encode_w * 3,
                    height: encode_h,
                    format: PixelFormat::RGB,
                };

                let jpeg_data = match compressor.compress_to_vec(image) {
                    Ok(data) => data,
                    Err(_) => continue,
                };

                let len = jpeg_data.len() as u32;
                if write_all_safe(&mut stream, &len.to_be_bytes()).is_err() {
                    break;
                }
                if write_all_safe(&mut stream, &jpeg_data).is_err() {
                    break;
                }

                frame_count += 1;
                if fps_timer.elapsed() >= Duration::from_secs(2) {
                    let real_fps = frame_count as f64 / fps_timer.elapsed().as_secs_f64();
                    println!("  Streaming: {:.1} FPS | Frame: {} KB", real_fps, jpeg_data.len() / 1024);
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

fn scale_rgb(
    src: &[u8], src_w: usize, src_h: usize,
    dst: &mut [u8], dst_w: usize, dst_h: usize,
) {
    for y in 0..dst_h {
        let src_y = y * src_h / dst_h;
        for x in 0..dst_w {
            let src_x = x * src_w / dst_w;
            let si = (src_y * src_w + src_x) * 3;
            let di = (y * dst_w + x) * 3;
            dst[di] = src[si];
            dst[di + 1] = src[si + 1];
            dst[di + 2] = src[si + 2];
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
