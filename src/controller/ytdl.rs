use crate::util::msg::XigmaBot;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::process::Command;
use waproto::whatsapp as wa;
use whatsapp_rust::bot::MessageContext;
use whatsapp_rust::download::MediaType;

const SEARCH_LIMIT: usize = 5;

#[derive(Debug, Clone)]
struct YtSearchItem {
    title: String,
    webpage_url: String,
}

fn now_id() -> Result<u128, Box<dyn std::error::Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
}

fn looks_like_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://")
}

fn resolve_watch_url(item: &YtSearchItem) -> Option<String> {
    if item.webpage_url.trim().is_empty() {
        None
    } else {
        Some(item.webpage_url.clone())
    }
}

async fn run_yt_dlp(args: &[String]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let output = Command::new("yt-dlp")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp gagal: {}", err.trim()).into());
    }

    Ok(output.stdout)
}

async fn yt_search(query: &str) -> Result<Vec<YtSearchItem>, Box<dyn std::error::Error>> {
    let q = format!("ytsearch{}:{}", SEARCH_LIMIT, query);
    let stdout = run_yt_dlp(&[
        "--skip-download".to_string(),
        "--print".to_string(),
        "%(title)s | %(webpage_url)s".to_string(),
        "--no-warnings".to_string(),
        q,
    ])
    .await?;
    let out = String::from_utf8_lossy(&stdout);

    let mut items = Vec::new();
    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Some((title, url)) = trimmed.rsplit_once(" | ") else {
            continue;
        };

        let title = title.trim();
        let url = url.trim();
        if title.is_empty() || url.is_empty() {
            continue;
        }

        items.push(YtSearchItem {
            title: title.to_string(),
            webpage_url: url.to_string(),
        });
    }

    Ok(items)
}

async fn find_downloaded_file(dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut rd = fs::read_dir(dir).await?;
    while let Some(entry) = rd.next_entry().await? {
        let meta = entry.metadata().await?;
        if meta.is_file() {
            return Ok(entry.path());
        }
    }
    Err("File hasil download tidak ditemukan".into())
}

async fn download_mp3(url: &str, workdir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = workdir.join("audio.%(ext)s");
    run_yt_dlp(&[
        "-x".to_string(),
        "--audio-format".to_string(),
        "mp3".to_string(),
        "--audio-quality".to_string(),
        "0".to_string(),
        "--no-playlist".to_string(),
        "-o".to_string(),
        output.to_string_lossy().to_string(),
        url.to_string(),
    ])
    .await?;
    find_downloaded_file(workdir).await
}

async fn download_mp4(url: &str, workdir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = workdir.join("video.%(ext)s");
    run_yt_dlp(&[
        "-f".to_string(),
        "bv*+ba/b".to_string(),
        "--merge-output-format".to_string(),
        "mp4".to_string(),
        "--no-playlist".to_string(),
        "-o".to_string(),
        output.to_string_lossy().to_string(),
        url.to_string(),
    ])
    .await?;
    find_downloaded_file(workdir).await
}

async fn send_audio_file(
    ctx: &MessageContext,
    audio_path: &Path,
    caption: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(audio_path).await?;
    let upload = ctx.client.upload(bytes, MediaType::Audio).await?;

    let msg = wa::Message {
        audio_message: Some(Box::new(wa::message::AudioMessage {
            mimetype: Some("audio/mpeg".to_string()),
            ptt: Some(false),
            url: Some(upload.url),
            direct_path: Some(upload.direct_path),
            media_key: Some(upload.media_key),
            file_enc_sha256: Some(upload.file_enc_sha256),
            file_sha256: Some(upload.file_sha256),
            file_length: Some(upload.file_length),
            context_info: Some(Box::new(wa::ContextInfo {
                stanza_id: Some(ctx.info.id.clone()),
                participant: Some(ctx.info.source.sender.to_string()),
                quoted_message: None,
                ..Default::default()
            })),
            ..Default::default()
        })),
        ..Default::default()
    };

    ctx.send_message(msg).await?;
    XigmaBot::reply(ctx, caption, true).await?;
    Ok(())
}

async fn send_video_file(
    ctx: &MessageContext,
    video_path: &Path,
    caption: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(video_path).await?;
    let upload = ctx.client.upload(bytes, MediaType::Video).await?;

    let msg = wa::Message {
        video_message: Some(Box::new(wa::message::VideoMessage {
            mimetype: Some("video/mp4".to_string()),
            caption: Some(caption.to_string()),
            url: Some(upload.url),
            direct_path: Some(upload.direct_path),
            media_key: Some(upload.media_key),
            file_enc_sha256: Some(upload.file_enc_sha256),
            file_sha256: Some(upload.file_sha256),
            file_length: Some(upload.file_length),
            context_info: Some(Box::new(wa::ContextInfo {
                stanza_id: Some(ctx.info.id.clone()),
                participant: Some(ctx.info.source.sender.to_string()),
                quoted_message: None,
                ..Default::default()
            })),
            ..Default::default()
        })),
        ..Default::default()
    };

    ctx.send_message(msg).await?;
    Ok(())
}

async fn create_temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut temp_dir = std::env::temp_dir();
    temp_dir.push(format!("xigma-{}-{}", prefix, now_id()?));
    fs::create_dir_all(&temp_dir).await?;
    Ok(temp_dir)
}

pub async fn ytmp3(ctx: &MessageContext, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !looks_like_url(url) {
        XigmaBot::reply(ctx, "Contoh: /ytmp3 https://youtube.com/watch?v=xxxx", true).await?;
        return Ok(());
    }

    XigmaBot::reply(ctx, "Sedang mengunduh audio dari YouTube...", true).await?;

    let temp_dir = create_temp_dir("ytmp3").await?;
    let result = download_mp3(url.trim(), &temp_dir).await.map_err(|e| e.to_string());
    match result {
        Ok(file_path) => {
            send_audio_file(ctx, &file_path, "✅ Selesai: audio MP3 berhasil dikirim.").await?
        }
        Err(err_text) => {
            XigmaBot::reply(ctx, &format!("Gagal ytmp3: {}", err_text), true).await?;
        }
    }
    let _ = fs::remove_dir_all(&temp_dir).await;

    Ok(())
}

pub async fn ytmp4(ctx: &MessageContext, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !looks_like_url(url) {
        XigmaBot::reply(ctx, "Contoh: /ytmp4 https://youtube.com/watch?v=xxxx", true).await?;
        return Ok(());
    }

    XigmaBot::reply(ctx, "Sedang mengunduh video dari YouTube...", true).await?;

    let temp_dir = create_temp_dir("ytmp4").await?;
    let result = download_mp4(url.trim(), &temp_dir).await.map_err(|e| e.to_string());
    match result {
        Ok(file_path) => {
            send_video_file(ctx, &file_path, "✅ Selesai: video MP4 berhasil dikirim.").await?
        }
        Err(err_text) => {
            XigmaBot::reply(ctx, &format!("Gagal ytmp4: {}", err_text), true).await?;
        }
    }
    let _ = fs::remove_dir_all(&temp_dir).await;

    Ok(())
}

pub async fn ytsearch(ctx: &MessageContext, query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let query = query.trim();
    if query.is_empty() {
        XigmaBot::reply(ctx, "Contoh: /ytsearch Peterpan - Mimpi Yang Sempurna", true).await?;
        return Ok(());
    }

    XigmaBot::reply(ctx, "Mencari di YouTube...", true).await?;

    let search_result: Result<Vec<YtSearchItem>, String> =
        yt_search(query).await.map_err(|e| e.to_string());
    match search_result {
        Ok(items) if !items.is_empty() => {
            let mut lines = vec![format!("🔎 Hasil pencarian untuk: {}", query)];
            for (idx, item) in items.iter().enumerate() {
                let url = resolve_watch_url(item).unwrap_or_else(|| "-".to_string());
                lines.push(format!(
                    "\n{}. {}\nURL: {}",
                    idx + 1,
                    item.title,
                    url
                ));
            }
            XigmaBot::reply(ctx, &lines.join("\n"), true).await?;
        }
        Ok(_) => {
            XigmaBot::reply(ctx, "Tidak ada hasil untuk kueri tersebut.", true).await?;
        }
        Err(err_text) => {
            XigmaBot::reply(ctx, &format!("Gagal ytsearch: {}", err_text), true).await?;
        }
    }

    Ok(())
}

pub async fn play_or_song(
    ctx: &MessageContext,
    query: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = query.trim();
    if query.is_empty() {
        XigmaBot::reply(ctx, "Contoh: /play Nidji - Laskar Pelangi", true).await?;
        return Ok(());
    }

    XigmaBot::reply(ctx, "Mencari lagu di YouTube...", true).await?;

    let items = match yt_search(query).await.map_err(|e| e.to_string()) {
        Ok(v) => v,
        Err(err_text) => {
            XigmaBot::reply(ctx, &format!("Gagal mencari lagu: {}", err_text), true).await?;
            return Ok(());
        }
    };

    let Some(best) = items.first() else {
        XigmaBot::reply(ctx, "Lagu tidak ditemukan di YouTube.", true).await?;
        return Ok(());
    };

    let Some(url) = resolve_watch_url(best) else {
        XigmaBot::reply(ctx, "Hasil ditemukan tapi URL tidak valid.", true).await?;
        return Ok(());
    };

    let title = best.title.clone();

    XigmaBot::reply(
        ctx,
        &format!("Ditemukan:\n{}\nMemproses MP3...", title),
        true,
    )
    .await?;

    let temp_dir = create_temp_dir("play").await?;
    let result = download_mp3(&url, &temp_dir).await.map_err(|e| e.to_string());
    match result {
        Ok(file_path) => {
            let caption = format!("🎵 {}", title);
            send_audio_file(ctx, &file_path, &caption).await?;
        }
        Err(err_text) => {
            XigmaBot::reply(ctx, &format!("Gagal memproses lagu: {}", err_text), true).await?;
        }
    }
    let _ = fs::remove_dir_all(&temp_dir).await;

    Ok(())
}
