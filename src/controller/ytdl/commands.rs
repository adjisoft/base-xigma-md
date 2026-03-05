use crate::controller::ytdl::context::{resolve_youtube_urls, urls_from_quoted_message, urls_from_text};
use crate::controller::ytdl::sender::{send_audio_file, send_image_file, send_video_file};
use crate::controller::ytdl::types::{runtime_tuning, THREAD_USAGE_PERCENT};
use crate::controller::ytdl::ytdlp::{
    create_temp_dir, detect_extractor, download_img, download_mp3, download_mp4, fetch_ad_info,
    list_supported_extractors, resolve_watch_url, yt_search,
};
use crate::util::{msg::XigmaBot, queue};
use futures::stream::{self, StreamExt};
use regex::Regex;
use std::collections::HashSet;
use std::path::PathBuf;
use whatsapp_rust::bot::MessageContext;

const MAX_YTMP_BATCH_URLS: usize = 8;

fn smart_batch_workers(total_urls: usize) -> usize {
    let tuning = runtime_tuning();
    let by_cpu = std::cmp::max(1, tuning.worker_threads / 2);
    std::cmp::min(total_urls, std::cmp::min(by_cpu, MAX_YTMP_BATCH_URLS))
}

async fn process_single_ytmp3(
    ctx: &MessageContext,
    target_url: &str,
    idx: usize,
    total: usize,
) -> Result<(), String> {
    let ad_info = fetch_ad_info(target_url).await.ok().flatten();
    let temp_dir = create_temp_dir("ytmp3")
        .await
        .map_err(|e| e.to_string())?;

    let result = download_mp3(target_url, &temp_dir)
        .await
        .map_err(|e| e.to_string());

    let send_result = match result {
        Ok(file_path) => {
            let caption = if total <= 1 {
                "✅ Selesai: audio MP3 berhasil dikirim.".to_string()
            } else {
                format!("✅ [{}/{}] audio MP3 berhasil dikirim.", idx + 1, total)
            };
            send_audio_file(ctx, &file_path, &caption, ad_info.as_ref())
                .await
                .map_err(|e| e.to_string())
        }
        Err(err_text) => Err(err_text),
    };

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    send_result
}

async fn process_single_ytmp4(
    ctx: &MessageContext,
    target_url: &str,
    idx: usize,
    total: usize,
) -> Result<(), String> {
    let ad_info = fetch_ad_info(target_url).await.ok().flatten();
    let temp_dir = create_temp_dir("ytmp4")
        .await
        .map_err(|e| e.to_string())?;

    let result = download_mp4(target_url, &temp_dir)
        .await
        .map_err(|e| e.to_string());

    let send_result = match result {
        Ok(file_path) => {
            let caption = if total <= 1 {
                "✅ Selesai: video MP4 berhasil dikirim.".to_string()
            } else {
                format!("✅ [{}/{}] video MP4 berhasil dikirim.", idx + 1, total)
            };
            send_video_file(ctx, &file_path, &caption, ad_info.as_ref())
                .await
                .map_err(|e| e.to_string())
        }
        Err(err_text) => Err(err_text),
    };

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    send_result
}

pub async fn ytmp3(ctx: &MessageContext, args: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut target_urls = resolve_youtube_urls(ctx, args);
    if target_urls.len() > MAX_YTMP_BATCH_URLS {
        target_urls.truncate(MAX_YTMP_BATCH_URLS);
    }

    if target_urls.is_empty() {
        XigmaBot::reply(
            ctx,
            "Contoh: /ytmp3 https://youtube.com/watch?v=xxxx\nAtau reply pesan berisi URL YouTube lalu kirim /ytmp3",
            true,
        )
        .await?;
        return Ok(());
    }
    let Some(_permit) = queue::acquire(ctx, "ytmp3").await? else {
        return Ok(());
    };

    let tuning = runtime_tuning();
    let total = target_urls.len();
    let workers = smart_batch_workers(total);
    XigmaBot::reply(
        ctx,
        &format!(
            "Sedang mengunduh audio dari YouTube...\nTotal URL: {}\nMultiprocessing: {} worker\nTuning CPU: {}% thread perangkat ({} dari {} thread).",
            total,
            workers,
            THREAD_USAGE_PERCENT,
            tuning.worker_threads,
            tuning.cpu_threads_total
        ),
        true,
    )
    .await?;

    let results = stream::iter(target_urls.into_iter().enumerate())
        .map(|(idx, target_url)| async move {
            let outcome = process_single_ytmp3(ctx, &target_url, idx, total).await;
            (idx, target_url, outcome)
        })
        .buffer_unordered(workers)
        .collect::<Vec<_>>()
        .await;

    let mut success = 0usize;
    let mut failed = 0usize;
    for (idx, url, outcome) in results {
        if let Err(err_text) = outcome {
            failed += 1;
            let _ = XigmaBot::reply(
                ctx,
                &format!("Gagal ytmp3 URL {}:\n{}\n{}", idx + 1, url, err_text),
                true,
            )
            .await;
        } else {
            success += 1;
        }
    }
    XigmaBot::reply(
        ctx,
        &format!(
            "Batch ytmp3 selesai.\nTotal: {}\nSukses: {}\nGagal: {}",
            total, success, failed
        ),
        true,
    )
    .await?;

    Ok(())
}

pub async fn ytmp4(ctx: &MessageContext, args: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut target_urls = resolve_youtube_urls(ctx, args);
    if target_urls.len() > MAX_YTMP_BATCH_URLS {
        target_urls.truncate(MAX_YTMP_BATCH_URLS);
    }

    if target_urls.is_empty() {
        XigmaBot::reply(
            ctx,
            "Contoh: /ytmp4 https://youtube.com/watch?v=xxxx\nAtau reply pesan berisi URL YouTube lalu kirim /ytmp4",
            true,
        )
        .await?;
        return Ok(());
    }
    let Some(_permit) = queue::acquire(ctx, "ytmp4").await? else {
        return Ok(());
    };

    let tuning = runtime_tuning();
    let total = target_urls.len();
    let workers = smart_batch_workers(total);
    XigmaBot::reply(
        ctx,
        &format!(
            "Sedang mengunduh video dari YouTube...\nTotal URL: {}\nSmart multiprocessing: {} worker\nTuning CPU: {}% thread perangkat ({} dari {} thread).",
            total,
            workers,
            THREAD_USAGE_PERCENT,
            tuning.worker_threads,
            tuning.cpu_threads_total
        ),
        true,
    )
    .await?;

    let results = stream::iter(target_urls.into_iter().enumerate())
        .map(|(idx, target_url)| async move {
            let outcome = process_single_ytmp4(ctx, &target_url, idx, total).await;
            (idx, target_url, outcome)
        })
        .buffer_unordered(workers)
        .collect::<Vec<_>>()
        .await;

    let mut success = 0usize;
    let mut failed = 0usize;
    for (idx, url, outcome) in results {
        if let Err(err_text) = outcome {
            failed += 1;
            let _ = XigmaBot::reply(
                ctx,
                &format!("Gagal ytmp4 URL {}:\n{}\n{}", idx + 1, url, err_text),
                true,
            )
            .await;
        } else {
            success += 1;
        }
    }
    XigmaBot::reply(
        ctx,
        &format!(
            "Batch ytmp4 selesai.\nTotal: {}\nSukses: {}\nGagal: {}",
            total, success, failed
        ),
        true,
    )
    .await?;

    Ok(())
}

pub async fn ytsearch(ctx: &MessageContext, query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let query = query.trim();
    if query.is_empty() {
        XigmaBot::reply(ctx, "Contoh: /ytsearch Peterpan - Mimpi Yang Sempurna", true).await?;
        return Ok(());
    }
    let Some(_permit) = queue::acquire(ctx, "ytsearch").await? else {
        return Ok(());
    };

    XigmaBot::reply(ctx, "Mencari di YouTube...", true).await?;

    let search_result = yt_search(query).await.map_err(|e| e.to_string());
    match search_result {
        Ok(items) if !items.is_empty() => {
            let mut lines = vec![format!("🔎 Hasil pencarian untuk: {}", query)];
            for (idx, item) in items.iter().enumerate() {
                let url = resolve_watch_url(item).unwrap_or_else(|| "-".to_string());
                lines.push(format!("\n{}. {}\nURL: {}", idx + 1, item.title, url));
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
    let Some(_permit) = queue::acquire(ctx, "play/song").await? else {
        return Ok(());
    };

    let tuning = runtime_tuning();
    XigmaBot::reply(
        ctx,
        &format!(
            "Mencari lagu di YouTube...\nTuning: {}% thread perangkat ({} dari {} thread).",
            THREAD_USAGE_PERCENT, tuning.worker_threads, tuning.cpu_threads_total
        ),
        true,
    )
    .await?;

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

    let ad_info = fetch_ad_info(&url).await.ok().flatten();
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
            send_audio_file(ctx, &file_path, &caption, ad_info.as_ref()).await?;
        }
        Err(err_text) => {
            XigmaBot::reply(ctx, &format!("Gagal memproses lagu: {}", err_text), true).await?;
        }
    }
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

    Ok(())
}

fn parse_aio_args(args: &str) -> Result<(Vec<String>, String), String> {
    let re = Regex::new(r"(?i)(^|\s)-(mp3|mp4|img|auto)(?=\s|$)")
        .map_err(|_| "Gagal membaca opsi aio".to_string())?;

    let mut mode = "auto".to_string();
    for cap in re.captures_iter(args) {
        if let Some(m) = cap.get(2) {
            mode = m.as_str().to_lowercase();
        }
    }

    let cleaned = re.replace_all(args, " ").to_string();
    let urls = urls_from_text(&cleaned);

    if urls.is_empty() {
        return Err(
            "Contoh: /aio <url1> <url2> <url3> [opsi -auto|-mp3|-mp4|-img]\nAtau reply pesan berisi URL lalu kirim /aio -auto"
                .to_string(),
        );
    }

    Ok((urls, mode))
}

pub async fn aio(ctx: &MessageContext, args: &str) -> Result<(), Box<dyn std::error::Error>> {
    let Some(_permit) = queue::acquire(ctx, "AIO downloader").await? else {
        return Ok(());
    };

    let (mut urls, mode) = parse_aio_args(args).unwrap_or((Vec::new(), "auto".to_string()));
    if urls.is_empty() {
        urls = urls_from_quoted_message(ctx);
    }
    if urls.is_empty() {
        XigmaBot::reply(
            ctx,
            "Contoh: /aio <url1> <url2> <url3> [opsi -auto|-mp3|-mp4|-img]\nAtau reply pesan berisi URL lalu kirim /aio -auto",
            true,
        )
        .await?;
        return Ok(());
    }

    let mut dedupe = HashSet::new();
    urls.retain(|u| dedupe.insert(u.clone()));
    if urls.len() > 8 {
        urls.truncate(8);
    }

    XigmaBot::reply(
        ctx,
        &format!(
            "AIO downloader memproses {} URL...\nMode: {}",
            urls.len(),
            mode
        ),
        true,
    )
    .await?;

    let supported_extractors = list_supported_extractors().await.unwrap_or_default();
    let mut success = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for (idx, url) in urls.iter().enumerate() {
        let ad_info = fetch_ad_info(url).await.ok().flatten();
        let detected_extractor = detect_extractor(url).await.ok().flatten();
        if let Some(ext) = detected_extractor
            && !supported_extractors.is_empty()
            && !supported_extractors.contains(&ext)
        {
            skipped += 1;
            let _ = XigmaBot::reply(
                ctx,
                &format!(
                    "Skip URL {}: extractor `{}` terdeteksi rusak/tidak aktif.",
                    idx + 1,
                    ext
                ),
                true,
            )
            .await;
            continue;
        }

        let temp_dir = create_temp_dir("aio").await?;
        let result: Result<(String, PathBuf), String> = match mode.as_str() {
            "mp3" => download_mp3(url, &temp_dir)
                .await
                .map(|p| ("mp3".to_string(), p))
                .map_err(|e| e.to_string()),
            "mp4" => download_mp4(url, &temp_dir)
                .await
                .map(|p| ("mp4".to_string(), p))
                .map_err(|e| e.to_string()),
            "img" => download_img(url, &temp_dir)
                .await
                .map(|p| ("img".to_string(), p))
                .map_err(|e| e.to_string()),
            _ => {
                if let Ok(path) = download_mp4(url, &temp_dir).await {
                    Ok(("mp4".to_string(), path))
                } else if let Ok(path) = download_img(url, &temp_dir).await {
                    Ok(("img".to_string(), path))
                } else {
                    download_mp3(url, &temp_dir)
                        .await
                        .map(|p| ("mp3".to_string(), p))
                        .map_err(|e| e.to_string())
                }
            }
        };

        match result {
            Ok((kind, path)) => {
                let send_result = match kind.as_str() {
                    "mp3" => {
                        send_audio_file(
                            ctx,
                            &path,
                            "[AIO]: audio berhasil diunduh dan dikirim.",
                            ad_info.as_ref(),
                        )
                        .await
                    }
                    "img" => {
                        send_image_file(
                            ctx,
                            &path,
                            "[AIO]: gambar berhasil diunduh dan dikirim.",
                            ad_info.as_ref(),
                        )
                        .await
                    }
                    _ => {
                        send_video_file(
                            ctx,
                            &path,
                            "[AIO]: video berhasil diunduh dan dikirim.",
                            ad_info.as_ref(),
                        )
                        .await
                    }
                };
                if send_result.is_ok() {
                    success += 1;
                } else {
                    failed += 1;
                }
            }
            Err(err_text) => {
                failed += 1;
                let _ = XigmaBot::reply(ctx, &format!("Gagal URL {}: {}", idx + 1, err_text), true)
                    .await;
            }
        }

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    XigmaBot::reply(
        ctx,
        &format!(
            "AIO selesai.\nTotal URL: {}\nSukses: {}\nGagal: {}\nSkip extractor rusak: {}",
            urls.len(),
            success,
            failed,
            skipped
        ),
        true,
    )
    .await?;

    Ok(())
}
