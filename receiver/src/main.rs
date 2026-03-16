use minifb::{Key, Window, WindowOptions};
use std::env;
use std::io::Read;
use std::net::TcpStream;
use std::time::{Duration, Instant};
use turbojpeg::{Decompressor, PixelFormat};

const PORT: u16 = 8765;
const RECONNECT_DELAY: Duration = Duration::from_secs(3);

fn main() {
    let args: Vec<String> = env::args().collect();
    let host = args.get(1).unwrap_or_else(|| {
        eprintln!("Usage: screen-receiver <mac-ip>");
        eprintln!("Example: screen-receiver 192.168.1.42");
        std::process::exit(1);
    });

    let addr = if host.contains(':') {
        host.clone()
    } else {
        format!("{}:{}", host, PORT)
    };
    println!("===========================================");
    println!("  Screen Receiver");
    println!("===========================================");

    loop {
        println!("  Connecting to {}...", addr);

        match TcpStream::connect(&addr) {
            Ok(stream) => {
                println!("  Connected!");
                if let Err(e) = run_stream(stream) {
                    eprintln!("  Stream ended: {}", e);
                }
            }
            Err(e) => {
                eprintln!("  Connection failed: {}", e);
            }
        }

        println!("  Reconnecting in {} seconds...", RECONNECT_DELAY.as_secs());
        std::thread::sleep(RECONNECT_DELAY);
    }
}

fn run_stream(mut stream: TcpStream) -> Result<(), String> {
    stream.set_nodelay(true).map_err(|e| e.to_string())?;

    let (width, height) = read_header(&mut stream)?;
    println!("  Resolution: {}x{}", width, height);

    let mut window = create_window(width, height)?;
    let mut decompressor = Decompressor::new().map_err(|e| e.to_string())?;
    let mut len_buf = [0u8; 4];
    let mut rgb_buf = vec![0u8; width * height * 3];
    let mut argb_buf = vec![0u32; width * height];
    let mut frame_count: u64 = 0;
    let mut fps_timer = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if read_exact_safe(&mut stream, &mut len_buf).is_err() {
            return Err("Connection lost".into());
        }

        let frame_len = u32::from_be_bytes(len_buf) as usize;
        if frame_len == 0 || frame_len > 50_000_000 {
            return Err("Invalid frame size".into());
        }

        let mut jpeg_data = vec![0u8; frame_len];
        if read_exact_safe(&mut stream, &mut jpeg_data).is_err() {
            return Err("Connection lost".into());
        }

        decode_jpeg_to_argb(
            &mut decompressor,
            &jpeg_data,
            &mut rgb_buf,
            &mut argb_buf,
        )?;

        window
            .update_with_buffer(&argb_buf, width, height)
            .map_err(|e| e.to_string())?;

        frame_count += 1;
        let elapsed = fps_timer.elapsed();
        if elapsed >= Duration::from_secs(1) {
            let current_fps = frame_count as f64 / elapsed.as_secs_f64();
            window.set_title(&format!(
                "Screen Viewer  |  {:.0} FPS  |  {}x{}",
                current_fps, width, height
            ));
            frame_count = 0;
            fps_timer = Instant::now();
        }
    }

    Ok(())
}

fn read_header(stream: &mut TcpStream) -> Result<(usize, usize), String> {
    let mut len_buf = [0u8; 4];
    read_exact_safe(stream, &mut len_buf).map_err(|_| "Failed to read header length")?;
    let header_len = u32::from_be_bytes(len_buf) as usize;

    if header_len > 64 {
        return Err("Invalid header".into());
    }

    let mut header_buf = vec![0u8; header_len];
    read_exact_safe(stream, &mut header_buf).map_err(|_| "Failed to read header")?;

    let header = String::from_utf8(header_buf).map_err(|_| "Invalid header text")?;
    let parts: Vec<&str> = header.split('x').collect();

    if parts.len() != 2 {
        return Err("Invalid header format".into());
    }

    let w: usize = parts[0].parse().map_err(|_| "Invalid width")?;
    let h: usize = parts[1].parse().map_err(|_| "Invalid height")?;

    Ok((w, h))
}

fn create_window(width: usize, height: usize) -> Result<Window, String> {
    let (win_w, win_h) = scale_to_screen(width, height);

    Window::new(
        "Screen Viewer  |  Connecting...",
        win_w,
        win_h,
        WindowOptions {
            resize: true,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| e.to_string())
}

fn scale_to_screen(width: usize, height: usize) -> (usize, usize) {
    let max_w = 1600;
    let max_h = 900;

    if width <= max_w && height <= max_h {
        return (width, height);
    }

    let scale_w = max_w as f64 / width as f64;
    let scale_h = max_h as f64 / height as f64;
    let scale = scale_w.min(scale_h);

    ((width as f64 * scale) as usize, (height as f64 * scale) as usize)
}

fn decode_jpeg_to_argb(
    decompressor: &mut Decompressor,
    jpeg_data: &[u8],
    rgb_buf: &mut Vec<u8>,
    argb_buf: &mut [u32],
) -> Result<(), String> {
    let header = decompressor
        .read_header(jpeg_data)
        .map_err(|e| e.to_string())?;

    let pixel_count = header.width * header.height;
    let needed = pixel_count * 3;
    if rgb_buf.len() < needed {
        rgb_buf.resize(needed, 0);
    }

    let image = turbojpeg::Image {
        pixels: rgb_buf.as_mut_slice(),
        width: header.width,
        pitch: header.width * 3,
        height: header.height,
        format: PixelFormat::RGB,
    };

    decompressor
        .decompress(jpeg_data, image)
        .map_err(|e| e.to_string())?;

    for i in 0..pixel_count.min(argb_buf.len()) {
        let si = i * 3;
        argb_buf[i] = ((rgb_buf[si] as u32) << 16)
            | ((rgb_buf[si + 1] as u32) << 8)
            | (rgb_buf[si + 2] as u32);
    }

    Ok(())
}

fn read_exact_safe(stream: &mut TcpStream, buf: &mut [u8]) -> Result<(), ()> {
    stream.read_exact(buf).map_err(|_| ())
}
