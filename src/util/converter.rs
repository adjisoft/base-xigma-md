use std::path::Path;

use anyhow::{Context, Result, bail};
use image::imageops::{FilterType, overlay};
use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};
use serde_json::json;
use tokio::fs;
use tokio::process::Command;

const STICKER_SIZE: u32 = 512;

async fn run_ffmpeg(args: &[&str]) -> Result<()> {
    let output = Command::new("ffmpeg")
        .args(args)
        .output()
        .await
        .context("gagal menjalankan ffmpeg")?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("ffmpeg gagal: {}", stderr.trim());
}

async fn run_webpmux(args: &[&str]) -> Result<()> {
    let output = Command::new("webpmux").args(args).output().await;
    let output = match output {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!("webpmux tidak ditemukan. Install libwebp tools agar konversi stiker stabil");
        }
        Err(e) => return Err(e).context("gagal menjalankan webpmux"),
    };

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("webpmux gagal: {}", stderr.trim());
}

fn format_image_canvas(input_bytes: &[u8]) -> Result<Vec<u8>> {
    let image = image::load_from_memory(input_bytes).context("gagal membaca image input")?;
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        bail!("dimensi gambar tidak valid");
    }

    let scale = f32::min(
        STICKER_SIZE as f32 / width as f32,
        STICKER_SIZE as f32 / height as f32,
    );
    let resized_w = ((width as f32 * scale).round() as u32)
        .max(1)
        .min(STICKER_SIZE);
    let resized_h = ((height as f32 * scale).round() as u32)
        .max(1)
        .min(STICKER_SIZE);

    let resized = image.resize_exact(resized_w, resized_h, FilterType::Lanczos3);
    let mut canvas = RgbaImage::new(STICKER_SIZE, STICKER_SIZE);
    let x = ((STICKER_SIZE - resized_w) / 2) as i64;
    let y = ((STICKER_SIZE - resized_h) / 2) as i64;
    overlay(&mut canvas, &resized.to_rgba8(), x, y);

    let mut out = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(canvas)
        .write_to(&mut out, ImageFormat::Png)
        .context("gagal menulis PNG sementara")?;

    Ok(out.into_inner())
}

fn build_exif_payload(pack_name: &str, publisher: &str) -> Result<Vec<u8>> {
    let metadata = json!({
        "sticker-pack-id": "xigma-md",
        "sticker-pack-name": pack_name,
        "sticker-pack-publisher": publisher,
        "emojis": [""]
    });

    let json_bytes = serde_json::to_vec(&metadata).context("gagal serialize metadata sticker")?;
    let mut exif = vec![
        0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x41, 0x57, 0x07, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00,
    ];
    let json_len = json_bytes.len() as u32;
    exif[14..18].copy_from_slice(&json_len.to_le_bytes());
    exif.extend_from_slice(&json_bytes);
    Ok(exif)
}

async fn add_exif_with_webpmux(webp_path: &Path, pack_name: &str, publisher: &str) -> Result<()> {
    let exif = build_exif_payload(pack_name, publisher)?;
    let mut exif_path = webp_path.to_path_buf();
    exif_path.set_extension("metadata.exif");

    let mut muxed_path = webp_path.to_path_buf();
    muxed_path.set_extension("with_exif.webp");

    fs::write(&exif_path, exif).await?;

    let in_s = webp_path.to_string_lossy().to_string();
    let out_s = muxed_path.to_string_lossy().to_string();
    let exif_s = exif_path.to_string_lossy().to_string();

    let mux_result = run_webpmux(&["-set", "exif", &exif_s, &in_s, "-o", &out_s]).await;
    let _ = fs::remove_file(&exif_path).await;

    mux_result?;
    fs::copy(&muxed_path, webp_path).await?;
    let _ = fs::remove_file(&muxed_path).await;
    Ok(())
}

async fn convert_static_to_webp(input_path: &Path, output_path: &Path) -> Result<()> {
    let input_bytes = fs::read(input_path).await?;
    let formatted_png = format_image_canvas(&input_bytes)?;

    let mut temp_png = output_path.to_path_buf();
    temp_png.set_extension("prepared.png");
    fs::write(&temp_png, formatted_png).await?;

    let in_s = temp_png.to_string_lossy().to_string();
    let out_s = output_path.to_string_lossy().to_string();
    let result = run_ffmpeg(&[
        "-y",
        "-i",
        &in_s,
        "-vcodec",
        "libwebp",
        "-lossless",
        "1",
        "-preset",
        "picture",
        "-an",
        "-frames:v",
        "1",
        &out_s,
    ])
    .await;

    let _ = fs::remove_file(temp_png).await;
    result
}

async fn convert_animated_to_webp(input_path: &Path, output_path: &Path) -> Result<()> {
    let in_s = input_path.to_string_lossy().to_string();
    let out_s = output_path.to_string_lossy().to_string();
    run_ffmpeg(&[
        "-y",
        "-i",
        &in_s,
        "-vf",
        "fps=15,scale=512:512:force_original_aspect_ratio=decrease:flags=lanczos,pad=512:512:(ow-iw)/2:(oh-ih)/2:color=0x00000000,format=rgba",
        "-an",
        "-vsync",
        "0",
        "-loop",
        "0",
        "-t",
        "8",
        "-vcodec",
        "libwebp",
        "-lossless",
        "0",
        "-q:v",
        "55",
        "-preset",
        "default",
        &out_s,
    ])
    .await
}

pub async fn source_to_sticker_webp(
    input_path: &Path,
    output_path: &Path,
    animated: bool,
    pack_name: &str,
    publisher: &str,
) -> Result<()> {
    if animated {
        convert_animated_to_webp(input_path, output_path).await?;
    } else {
        convert_static_to_webp(input_path, output_path).await?;
    }

    add_exif_with_webpmux(output_path, pack_name, publisher).await?;
    Ok(())
}

pub async fn sticker_webp_to_png(input_path: &Path, output_path: &Path) -> Result<()> {
    let in_s = input_path.to_string_lossy().to_string();
    let out_s = output_path.to_string_lossy().to_string();
    run_ffmpeg(&["-y", "-i", &in_s, "-frames:v", "1", &out_s]).await
}

pub async fn sticker_webp_to_mp4(input_path: &Path, output_path: &Path) -> Result<()> {
    let in_s = input_path.to_string_lossy().to_string();
    let out_s = output_path.to_string_lossy().to_string();
    // Jalur utama: decode animated webp langsung dan encode ke h264 mp4.
    let primary = run_ffmpeg(&[
        "-y",
        "-ignore_loop",
        "0",
        "-i",
        &in_s,
        "-vf",
        "fps=20,scale=trunc(iw/2)*2:trunc(ih/2)*2:flags=lanczos,format=yuv420p",
        "-c:v",
        "libx264",
        "-preset",
        "veryfast",
        "-crf",
        "28",
        "-movflags",
        "+faststart",
        "-an",
        &out_s,
    ])
    .await;

    if primary.is_ok() {
        return Ok(());
    }

    // Fallback: paksa demux sebagai webp_pipe untuk build ffmpeg tertentu.
    let fallback = run_ffmpeg(&[
        "-y",
        "-f",
        "webp_pipe",
        "-i",
        &in_s,
        "-vf",
        "fps=20,scale=trunc(iw/2)*2:trunc(ih/2)*2:flags=lanczos,format=yuv420p",
        "-c:v",
        "libx264",
        "-preset",
        "veryfast",
        "-crf",
        "28",
        "-movflags",
        "+faststart",
        "-an",
        &out_s,
    ])
    .await;

    match (primary, fallback) {
        (Err(e1), Err(e2)) => bail!(
            "konversi webp animasi ke mp4 gagal (primary dan fallback). primary: {}; fallback: {}",
            e1,
            e2
        ),
        _ => Ok(()),
    }
}
