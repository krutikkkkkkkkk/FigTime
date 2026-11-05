// Converts assets/icon.png at build time so the app needs no image decoder:
//   - OUT_DIR/tray.rgba  : 32x32 raw RGBA for the tray / menu bar icon
//   - OUT_DIR/icon.ico   : Windows .exe icon (MSVC builds only, via winresource)

use std::{env, fs, io::BufReader, path::PathBuf};

const ICON: &str = "assets/icon.png";
const TRAY_SIZE: u32 = 32;

fn main() {
    println!("cargo:rerun-if-changed={ICON}");
    println!("cargo:rerun-if-changed=build.rs");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let (pixels, width, height) = load_rgba(ICON);

    fs::write(out.join("tray.rgba"), resize(&pixels, width, height, TRAY_SIZE)).unwrap();

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        let ico_path = out.join("icon.ico");
        fs::write(&ico_path, build_ico(&pixels, width, height)).unwrap();

        let mut res = winresource::WindowsResource::new();
        res.set_icon(ico_path.to_str().unwrap());
        res.set("ProductName", "FigTime");
        res.set("FileDescription", "FigTime");
        res.compile().expect("failed to compile Windows resources");
    }
}

fn load_rgba(path: &str) -> (Vec<u8>, u32, u32) {
    let file = BufReader::new(fs::File::open(path).expect("icon not found"));
    let mut decoder = png::Decoder::new(file);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());

    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => buf.chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => unreachable!("expanded by EXPAND"),
    };
    (rgba, info.width, info.height)
}

/// Area-average downscale to a size x size square (alpha-weighted, so no dark fringes).
fn resize(src: &[u8], w: u32, h: u32, size: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for ty in 0..size {
        let y0 = ty * h / size;
        let y1 = ((ty + 1) * h / size).max(y0 + 1);
        for tx in 0..size {
            let x0 = tx * w / size;
            let x1 = ((tx + 1) * w / size).max(x0 + 1);
            let (mut r, mut g, mut b, mut a, mut n) = (0u64, 0u64, 0u64, 0u64, 0u64);
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = ((y * w + x) * 4) as usize;
                    let pa = src[i + 3] as u64;
                    r += src[i] as u64 * pa;
                    g += src[i + 1] as u64 * pa;
                    b += src[i + 2] as u64 * pa;
                    a += pa;
                    n += 1;
                }
            }
            if a == 0 {
                out.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                out.extend_from_slice(&[(r / a) as u8, (g / a) as u8, (b / a) as u8, (a / n) as u8]);
            }
        }
    }
    out
}

fn encode_png(rgba: &[u8], size: u32) -> Vec<u8> {
    let mut data = Vec::new();
    let mut encoder = png::Encoder::new(&mut data, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(rgba).unwrap();
    writer.finish().unwrap();
    data
}

/// .ico containing PNG-compressed images (supported since Windows Vista).
fn build_ico(src: &[u8], w: u32, h: u32) -> Vec<u8> {
    let sizes = [16u32, 32, 48, 256];
    let images: Vec<Vec<u8>> = sizes.iter().map(|&s| encode_png(&resize(src, w, h, s), s)).collect();

    let mut ico = Vec::new();
    ico.extend_from_slice(&[0, 0, 1, 0]); // reserved, type = icon
    ico.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (&s, img) in sizes.iter().zip(&images) {
        let dim = if s >= 256 { 0 } else { s as u8 }; // 0 means 256
        ico.extend_from_slice(&[dim, dim, 0, 0]);
        ico.extend_from_slice(&1u16.to_le_bytes()); // color planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        ico.extend_from_slice(&(img.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += img.len() as u32;
    }
    for img in &images {
        ico.extend_from_slice(img);
    }
    ico
}
