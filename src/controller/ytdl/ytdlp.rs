use crate::controller::ytdl::types::{
    runtime_tuning, ytdlp_audio_postprocessor_args, ytdlp_concurrent_fragment_args, YtAdInfo,
    YtSearchItem, SEARCH_LIMIT,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::process::Command;

pub async fn run_yt_dlp(args: &[String]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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

pub async fn list_supported_extractors() -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let out = run_yt_dlp(&["--list-extractors".to_string()]).await?;
    let text = String::from_utf8_lossy(&out);

    let mut extractors = HashSet::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.contains("(CURRENTLY BROKEN)") {
            continue;
        }
        let name = trimmed
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if !name.is_empty() {
            extractors.insert(name);
        }
    }

    Ok(extractors)
}

pub async fn detect_extractor(url: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let out = run_yt_dlp(&[
        "--skip-download".to_string(),
        "--print".to_string(),
        "%(extractor)s".to_string(),
        "--no-warnings".to_string(),
        url.to_string(),
    ])
    .await?;
    let text = String::from_utf8_lossy(&out);
    let ext = text.lines().next().unwrap_or("").trim().to_lowercase();
    if ext.is_empty() {
        Ok(None)
    } else {
        Ok(Some(ext))
    }
}

pub async fn yt_search(query: &str) -> Result<Vec<YtSearchItem>, Box<dyn std::error::Error>> {
    let q = format!("ytsearch{}:{}", SEARCH_LIMIT, query);
    let stdout = run_yt_dlp(&[
        "--skip-download".to_string(),
        "--print".to_string(),
        "%(title)s\t%(webpage_url)s".to_string(),
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

        let Some((title, url)) = trimmed.rsplit_once('\t') else {
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

pub fn resolve_watch_url(item: &YtSearchItem) -> Option<String> {
    if item.webpage_url.trim().is_empty() {
        None
    } else {
        Some(item.webpage_url.clone())
    }
}

fn first_non_empty(lines: &[&str], idx: usize) -> String {
    lines
        .get(idx)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_string()
}

pub async fn fetch_ad_info(url: &str) -> Result<Option<YtAdInfo>, Box<dyn std::error::Error>> {
    let stdout = run_yt_dlp(&[
        "--skip-download".to_string(),
        "--no-warnings".to_string(),
        "--print".to_string(),
        "%(title)s".to_string(),
        "--print".to_string(),
        "%(uploader)s".to_string(),
        "--print".to_string(),
        "%(channel)s".to_string(),
        "--print".to_string(),
        "%(thumbnail)s".to_string(),
        "--print".to_string(),
        "%(webpage_url)s".to_string(),
        url.to_string(),
    ])
    .await?;

    let text = String::from_utf8_lossy(&stdout);
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return Ok(None);
    }

    let title = first_non_empty(&lines, 0);
    let uploader = first_non_empty(&lines, 1);
    let channel = first_non_empty(&lines, 2);
    let author = if !uploader.is_empty() {
        uploader
    } else {
        channel
    };
    let thumbnail_url = first_non_empty(&lines, 3);
    let mut source_url = first_non_empty(&lines, 4);
    if source_url.is_empty() {
        source_url = url.to_string();
    }

    if title.is_empty() && author.is_empty() && thumbnail_url.is_empty() {
        return Ok(None);
    }

    Ok(Some(YtAdInfo {
        title,
        author,
        thumbnail_url,
        source_url,
    }))
}

fn is_image_ext(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp"
    )
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

async fn find_downloaded_image_file(dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut rd = fs::read_dir(dir).await?;
    while let Some(entry) = rd.next_entry().await? {
        let path = entry.path();
        let meta = entry.metadata().await?;
        if !meta.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if is_image_ext(&ext) {
            return Ok(path);
        }
    }
    Err("File image hasil download tidak ditemukan".into())
}

pub async fn download_mp3(url: &str, workdir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = workdir.join("audio.%(ext)s");
    let tuning = runtime_tuning();
    let mut args = vec![
        "-x".to_string(),
        "--audio-format".to_string(),
        "mp3".to_string(),
        "--audio-quality".to_string(),
        "0".to_string(),
        "--no-playlist".to_string(),
        "-o".to_string(),
        output.to_string_lossy().to_string(),
    ];
    args.extend(ytdlp_concurrent_fragment_args(tuning));
    args.extend(ytdlp_audio_postprocessor_args(tuning));
    args.push(url.to_string());

    run_yt_dlp(&args).await?;
    find_downloaded_file(workdir).await
}

pub async fn download_mp4(url: &str, workdir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = workdir.join("video.%(ext)s");
    let tuning = runtime_tuning();
    let mut args = vec![
        "-f".to_string(),
        "bv*+ba/b".to_string(),
        "--merge-output-format".to_string(),
        "mp4".to_string(),
        "--no-playlist".to_string(),
        "-o".to_string(),
        output.to_string_lossy().to_string(),
    ];
    args.extend(ytdlp_concurrent_fragment_args(tuning));
    args.push(url.to_string());

    run_yt_dlp(&args).await?;
    find_downloaded_file(workdir).await
}

pub async fn download_img(url: &str, workdir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = workdir.join("image.%(ext)s");
    let tuning = runtime_tuning();
    let mut args = vec![
        "--no-playlist".to_string(),
        "--skip-download".to_string(),
        "--write-thumbnail".to_string(),
        "--convert-thumbnails".to_string(),
        "png".to_string(),
        "-o".to_string(),
        output.to_string_lossy().to_string(),
    ];
    args.extend(ytdlp_concurrent_fragment_args(tuning));
    args.push(url.to_string());

    run_yt_dlp(&args).await?;

    find_downloaded_image_file(workdir).await
}

fn now_id() -> Result<u128, Box<dyn std::error::Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
}

pub async fn create_temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut temp_dir = std::env::temp_dir();
    temp_dir.push(format!("xigma-{}-{}", prefix, now_id()?));
    fs::create_dir_all(&temp_dir).await?;
    Ok(temp_dir)
}
