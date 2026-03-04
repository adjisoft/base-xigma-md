use crate::{
    config,
    util::{converter, msg::XigmaBot},
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use waproto::whatsapp as wa;
use whatsapp_rust::bot::MessageContext;
use whatsapp_rust::download::MediaType;
use whatsapp_rust::proto_helpers::MessageExt;

enum StickerSource<'a> {
    Image(&'a wa::message::ImageMessage, bool),
    Video(&'a wa::message::VideoMessage, u32),
}

fn extract_context_info(base: &wa::Message) -> Option<&wa::ContextInfo> {
    if let Some(ext) = &base.extended_text_message {
        return ext.context_info.as_deref();
    }
    if let Some(img) = &base.image_message {
        return img.context_info.as_deref();
    }
    if let Some(vid) = &base.video_message {
        return vid.context_info.as_deref();
    }
    if let Some(stk) = &base.sticker_message {
        return stk.context_info.as_deref();
    }
    None
}

fn resolve_sticker_source(ctx: &MessageContext) -> Option<StickerSource<'_>> {
    let base = ctx.message.get_base_message();

    if let Some(img) = base.image_message.as_ref() {
        let is_gif = img
            .mimetype
            .as_deref()
            .is_some_and(|m| m.eq_ignore_ascii_case("image/gif"));
        return Some(StickerSource::Image(img, is_gif));
    }
    if let Some(vid) = base.video_message.as_ref() {
        return Some(StickerSource::Video(vid, vid.seconds.unwrap_or_default()));
    }

    let quoted = extract_context_info(base).and_then(|c| c.quoted_message.as_ref())?;
    if let Some(img) = quoted.image_message.as_ref() {
        let is_gif = img
            .mimetype
            .as_deref()
            .is_some_and(|m| m.eq_ignore_ascii_case("image/gif"));
        return Some(StickerSource::Image(img, is_gif));
    }
    if let Some(vid) = quoted.video_message.as_ref() {
        return Some(StickerSource::Video(vid, vid.seconds.unwrap_or_default()));
    }
    None
}

fn resolve_non_sticker_media(ctx: &MessageContext) -> bool {
    let base = ctx.message.get_base_message();
    if base.image_message.is_some() || base.video_message.is_some() {
        return true;
    }

    let quoted = extract_context_info(base).and_then(|c| c.quoted_message.as_ref());
    quoted
        .map(|q| q.image_message.is_some() || q.video_message.is_some())
        .unwrap_or(false)
}

fn resolve_sticker_input(ctx: &MessageContext) -> bool {
    let base = ctx.message.get_base_message();
    if base.sticker_message.is_some() {
        return true;
    }

    let quoted = extract_context_info(base).and_then(|c| c.quoted_message.as_ref());
    quoted.map(|q| q.sticker_message.is_some()).unwrap_or(false)
}

fn resolve_sticker_message(ctx: &MessageContext) -> Option<&wa::message::StickerMessage> {
    let base = ctx.message.get_base_message();
    if let Some(stk) = base.sticker_message.as_ref() {
        return Some(stk);
    }
    extract_context_info(base)
        .and_then(|c| c.quoted_message.as_ref())
        .and_then(|q| q.sticker_message.as_ref())
        .map(|v| &**v)
}

fn make_temp_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let mut dir = std::env::temp_dir();
    dir.push(format!("xigma-sticker-{}", ts));
    Ok(dir)
}

pub async fn to_sticker(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    if resolve_sticker_input(ctx) {
        XigmaBot::reply(
            ctx,
            "Media yang direply adalah stiker, bukan gambar/gif/video. Kirim gambar, gif, atau video (maks 8 detik).",
            true,
        )
        .await?;
        return Ok(());
    }

    let Some(source) = resolve_sticker_source(ctx) else {
        XigmaBot::reply(
            ctx,
            "Kirim/reply gambar, gif, atau video lalu ketik /sticker (/s /stiker).",
            true,
        )
        .await?;
        return Ok(());
    };

    let temp_dir = make_temp_dir()?;
    fs::create_dir_all(&temp_dir).await?;

    let in_path = temp_dir.join("input.bin");
    let out_path = temp_dir.join("output.webp");

    let (bytes, animated) = match source {
        StickerSource::Image(img, is_gif) => (ctx.client.download(img).await?, is_gif),
        StickerSource::Video(vid, seconds) => {
            if seconds > 8 {
                XigmaBot::reply(
                    ctx,
                    "Video terlalu panjang. Maksimal durasi untuk /sticker adalah 8 detik.",
                    true,
                )
                .await?;
                let _ = fs::remove_dir_all(&temp_dir).await;
                return Ok(());
            }
            (ctx.client.download(vid).await?, true)
        }
    };

    fs::write(&in_path, bytes).await?;
    let cfg = config::get_config();
    if let Err(err) = converter::source_to_sticker_webp(
        &in_path,
        &out_path,
        animated,
        &cfg.nama_bot,
        &cfg.nama_owner,
    )
    .await
    {
        let _ = fs::remove_dir_all(&temp_dir).await;
        XigmaBot::reply(ctx, &format!("Gagal membuat stiker: {}", err), true).await?;
        return Ok(());
    }

    let webp_bytes = fs::read(&out_path).await?;
    let upload = ctx.client.upload(webp_bytes, MediaType::Sticker).await?;
    let quoted_ctx = wa::ContextInfo {
        stanza_id: Some(ctx.info.id.clone()),
        participant: Some(ctx.info.source.sender.to_string()),
        quoted_message: None,
        ..Default::default()
    };

    let sticker_msg = wa::Message {
        sticker_message: Some(Box::new(wa::message::StickerMessage {
            url: Some(upload.url),
            direct_path: Some(upload.direct_path),
            media_key: Some(upload.media_key),
            file_enc_sha256: Some(upload.file_enc_sha256),
            file_sha256: Some(upload.file_sha256),
            file_length: Some(upload.file_length),
            mimetype: Some("image/webp".to_string()),
            width: Some(512),
            height: Some(512),
            is_animated: Some(animated),
            context_info: Some(Box::new(quoted_ctx)),
            ..Default::default()
        })),
        ..Default::default()
    };

    let send_res = ctx.send_message(sticker_msg).await;
    let _ = fs::remove_dir_all(&temp_dir).await;
    send_res?;
    Ok(())
}

pub async fn sticker_to_media(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    if resolve_non_sticker_media(ctx) {
        XigmaBot::reply(
            ctx,
            "Media yang direply bukan stiker. Gunakan /toimg atau /tovid hanya untuk stiker.",
            true,
        )
        .await?;
        return Ok(());
    }

    let Some(sticker) = resolve_sticker_message(ctx) else {
        XigmaBot::reply(
            ctx,
            "Kirim/reply stiker lalu ketik /toimg atau /tovid.",
            true,
        )
        .await?;
        return Ok(());
    };

    let temp_dir = make_temp_dir()?;
    fs::create_dir_all(&temp_dir).await?;

    let in_path = temp_dir.join("input.webp");
    fs::write(&in_path, ctx.client.download(sticker).await?).await?;

    let is_animated = sticker.is_animated.unwrap_or(false);

    if is_animated {
        let out_path = temp_dir.join("output.mp4");
        if let Err(err) = converter::sticker_webp_to_mp4(&in_path, &out_path).await {
            let _ = fs::remove_dir_all(&temp_dir).await;
            XigmaBot::reply(
                ctx,
                &format!(
                    "Gagal konversi stiker animasi ke video.\nDetail: {}\nCoba kirim stiker lain atau gunakan /toimg untuk ambil frame pertama.",
                    err
                ),
                true,
            )
            .await?;
            return Ok(());
        }
        let video_bytes = fs::read(&out_path).await?;
        let upload = ctx.client.upload(video_bytes, MediaType::Video).await?;

        let send_res = ctx
            .send_message(wa::Message {
                video_message: Some(Box::new(wa::message::VideoMessage {
                    mimetype: Some("video/mp4".to_string()),
                    url: Some(upload.url),
                    direct_path: Some(upload.direct_path),
                    media_key: Some(upload.media_key),
                    file_enc_sha256: Some(upload.file_enc_sha256),
                    file_sha256: Some(upload.file_sha256),
                    file_length: Some(upload.file_length),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .await;
        let _ = fs::remove_dir_all(&temp_dir).await;
        send_res?;
    } else {
        let out_path = temp_dir.join("output.png");
        if converter::sticker_webp_to_png(&in_path, &out_path)
            .await
            .is_err()
        {
            let _ = fs::remove_dir_all(&temp_dir).await;
            XigmaBot::reply(
                ctx,
                "Gagal konversi stiker ke gambar. Pastikan sticker valid lalu coba lagi.",
                true,
            )
            .await?;
            return Ok(());
        }
        let image_bytes = fs::read(&out_path).await?;
        let upload = ctx.client.upload(image_bytes, MediaType::Image).await?;
        let quoted_ctx = wa::ContextInfo {
            stanza_id: Some(ctx.info.id.clone()),
            participant: Some(ctx.info.source.sender.to_string()),
            quoted_message: None,
            ..Default::default()
        };

        let send_res = ctx
            .send_message(wa::Message {
                image_message: Some(Box::new(wa::message::ImageMessage {
                    mimetype: Some("image/png".to_string()),
                    url: Some(upload.url),
                    direct_path: Some(upload.direct_path),
                    media_key: Some(upload.media_key),
                    file_enc_sha256: Some(upload.file_enc_sha256),
                    file_sha256: Some(upload.file_sha256),
                    file_length: Some(upload.file_length),
                    context_info: Some(Box::new(quoted_ctx)),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .await;
        let _ = fs::remove_dir_all(&temp_dir).await;
        send_res?;
    }

    Ok(())
}
