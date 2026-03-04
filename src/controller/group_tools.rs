use crate::{config, util::helper, util::msg::XigmaBot};
use anyhow::Result as AnyhowResult;
use std::time::Duration;
use tokio::time::sleep;
use wacore_binary::jid::Jid;
use waproto::whatsapp as wa;
use whatsapp_rust::bot::MessageContext;
use whatsapp_rust::download::MediaType;
use whatsapp_rust::proto_helpers::MessageExt;

const DEFAULT_DELAY_SECS: u64 = 3;
const MIN_DELAY_SECS: u64 = 3;

enum MediaRef<'a> {
    Image(&'a wa::message::ImageMessage),
    Video(&'a wa::message::VideoMessage),
    Audio(&'a wa::message::AudioMessage),
}

async fn ensure_owner(ctx: &MessageContext) -> Result<bool, Box<dyn std::error::Error>> {
    let sender = ctx.info.source.sender.to_string();
    if config::is_owner(&sender) {
        return Ok(true);
    }

    XigmaBot::reply(ctx, "Hanya owner yang boleh memakai fitur ini.", true).await?;
    Ok(false)
}

fn is_group_chat(ctx: &MessageContext) -> bool {
    ctx.info.source.chat.to_string().ends_with("@g.us")
}

fn resolve_group_target(ctx: &MessageContext, args: &str) -> Option<String> {
    let trimmed = args.trim();
    if !trimmed.is_empty() {
        return config::normalize_group_target(trimmed);
    }

    if is_group_chat(ctx) {
        return config::normalize_group_target(&ctx.info.source.chat.to_string());
    }

    None
}

fn text_message(text: &str) -> wa::Message {
    let ad_context = ad_context_info();
    wa::Message {
        extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
            text: Some(text.to_string()),
            context_info: Some(Box::new(ad_context)),
            ..Default::default()
        })),
        ..Default::default()
    }
}

fn ad_context_info() -> wa::ContextInfo {
    use wa::context_info::external_ad_reply_info as ad;

    let cfg = config::get_config();
    let ad_info = wa::context_info::ExternalAdReplyInfo {
        title: Some(cfg.nama_bot),
        body: Some("Bot WhatsApp 100% Rust".to_string()),
        media_type: Some(ad::MediaType::Image as i32),
        thumbnail_url: Some(cfg.thumbnail_url),
        media_url: Some("https://github.com/magercode".to_string()),
        render_larger_thumbnail: Some(true),
        show_ad_attribution: Some(false),
        thumbnail: None,
        source_type: None,
        source_id: None,
        source_url: Some("https://github.com/magercode".to_string()),
        contains_auto_reply: None,
        ctwa_clid: None,
        r#ref: None,
        click_to_whatsapp_call: None,
        ad_context_preview_dismissed: None,
        source_app: None,
        automated_greeting_message_shown: None,
        greeting_message_body: None,
        cta_payload: None,
        disable_nudge: None,
        original_image_url: None,
        automated_greeting_message_cta_type: None,
        wtwa_ad_format: None,
        ad_type: None,
        wtwa_website_url: None,
        ad_preview_url: Some("https://github.com/magercode".to_string()),
    };

    wa::ContextInfo {
        external_ad_reply: Some(ad_info),
        ..Default::default()
    }
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
    if let Some(aud) = &base.audio_message {
        return aud.context_info.as_deref();
    }
    None
}

fn media_from_message(msg: &wa::Message) -> Option<MediaRef<'_>> {
    if let Some(image) = msg.image_message.as_ref() {
        return Some(MediaRef::Image(image));
    }
    if let Some(video) = msg.video_message.as_ref() {
        return Some(MediaRef::Video(video));
    }
    if let Some(audio) = msg.audio_message.as_ref() {
        return Some(MediaRef::Audio(audio));
    }
    None
}

fn resolve_media_target(ctx: &MessageContext) -> Option<MediaRef<'_>> {
    let base = ctx.message.get_base_message();

    if let Some(media) = media_from_message(base) {
        return Some(media);
    }

    let quoted = extract_context_info(base).and_then(|info| info.quoted_message.as_ref())?;
    media_from_message(quoted)
}

fn parse_delay_arg(args: &str) -> Result<(String, u64), String> {
    let mut delay = DEFAULT_DELAY_SECS;
    let mut plain_parts: Vec<&str> = Vec::new();

    for token in args.split_whitespace() {
        if let Some(raw_delay) = token.strip_prefix("-d=") {
            delay = raw_delay
                .parse::<u64>()
                .map_err(|_| "Delay tidak valid. Contoh: -d=5".to_string())?;
        } else {
            plain_parts.push(token);
        }
    }

    if delay < MIN_DELAY_SECS {
        return Err(format!(
            "Delay minimal {} detik untuk menghindari spam/ban.",
            MIN_DELAY_SECS
        ));
    }

    Ok((plain_parts.join(" ").trim().to_string(), delay))
}

async fn build_broadcast_message(
    ctx: &MessageContext,
    text: &str,
) -> AnyhowResult<wa::Message> {
    if let Some(media) = resolve_media_target(ctx) {
        let message = match media {
            MediaRef::Image(img) => {
                let data = ctx.client.download(img).await?;
                let upload = ctx.client.upload(data, MediaType::Image).await?;
                let ad_context = ad_context_info();

                wa::Message {
                    image_message: Some(Box::new(wa::message::ImageMessage {
                        url: Some(upload.url),
                        direct_path: Some(upload.direct_path),
                        media_key: Some(upload.media_key),
                        file_enc_sha256: Some(upload.file_enc_sha256),
                        file_sha256: Some(upload.file_sha256),
                        file_length: Some(upload.file_length),
                        mimetype: img
                            .mimetype
                            .clone()
                            .or_else(|| Some("image/jpeg".to_string())),
                        caption: if text.is_empty() {
                            None
                        } else {
                            Some(text.to_string())
                        },
                        context_info: Some(Box::new(ad_context)),
                        height: img.height,
                        width: img.width,
                        ..Default::default()
                    })),
                    ..Default::default()
                }
            }
            MediaRef::Video(vid) => {
                let data = ctx.client.download(vid).await?;
                let upload = ctx.client.upload(data, MediaType::Video).await?;
                let ad_context = ad_context_info();

                wa::Message {
                    video_message: Some(Box::new(wa::message::VideoMessage {
                        url: Some(upload.url),
                        direct_path: Some(upload.direct_path),
                        media_key: Some(upload.media_key),
                        file_enc_sha256: Some(upload.file_enc_sha256),
                        file_sha256: Some(upload.file_sha256),
                        file_length: Some(upload.file_length),
                        mimetype: vid
                            .mimetype
                            .clone()
                            .or_else(|| Some("video/mp4".to_string())),
                        caption: if text.is_empty() {
                            None
                        } else {
                            Some(text.to_string())
                        },
                        context_info: Some(Box::new(ad_context)),
                        seconds: vid.seconds,
                        height: vid.height,
                        width: vid.width,
                        gif_playback: vid.gif_playback,
                        ..Default::default()
                    })),
                    ..Default::default()
                }
            }
            MediaRef::Audio(aud) => {
                let data = ctx.client.download(aud).await?;
                let upload = ctx.client.upload(data, MediaType::Audio).await?;
                let ad_context = ad_context_info();

                wa::Message {
                    audio_message: Some(Box::new(wa::message::AudioMessage {
                        url: Some(upload.url),
                        direct_path: Some(upload.direct_path),
                        media_key: Some(upload.media_key),
                        file_enc_sha256: Some(upload.file_enc_sha256),
                        file_sha256: Some(upload.file_sha256),
                        file_length: Some(upload.file_length),
                        mimetype: aud
                            .mimetype
                            .clone()
                            .or_else(|| Some("audio/ogg; codecs=opus".to_string())),
                        context_info: Some(Box::new(ad_context)),
                        seconds: aud.seconds,
                        ptt: aud.ptt,
                        ..Default::default()
                    })),
                    ..Default::default()
                }
            }
        };

        return Ok(message);
    }

    if text.trim().is_empty() {
        return Err(anyhow::anyhow!("Pesan kosong"));
    }

    Ok(text_message(text))
}

pub async fn blacklist(ctx: &MessageContext, args: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !ensure_owner(ctx).await? {
        return Ok(());
    }

    let Some(target) = resolve_group_target(ctx, args) else {
        XigmaBot::reply(
            ctx,
            "Contoh:\n- /bl (kirim dari grup target)\n- /blacklist 1203630xxxxx@g.us",
            true,
        )
        .await?;
        return Ok(());
    };

    match config::add_blacklist_group(&target) {
        Ok(true) => {
            XigmaBot::reply(ctx, &format!("Grup diblacklist: {}", target), true).await?;
        }
        Ok(false) => {
            XigmaBot::reply(ctx, "Grup sudah ada di blacklist.", true).await?;
        }
        Err(e) => {
            XigmaBot::reply(ctx, &format!("Gagal blacklist grup: {}", e), true).await?;
        }
    }

    Ok(())
}

pub async fn group_id(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    if !is_group_chat(ctx) {
        XigmaBot::reply(ctx, "Perintah ini hanya bisa dipakai di grup.", true).await?;
        return Ok(());
    }

    let jid = ctx.info.source.chat.to_string();
    let subject = helper::fetch_group_metadata(ctx, &jid)
        .await
        .map(|meta| meta.subject)
        .unwrap_or_else(|_| "-".to_string());

    XigmaBot::reply(
        ctx,
        &format!("ID Grup:\n{}\n\nNama Grup:\n{}", jid, subject),
        true,
    )
    .await?;

    Ok(())
}

pub async fn broadcast_groups(
    ctx: &MessageContext,
    args: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !ensure_owner(ctx).await? {
        return Ok(());
    }

    let (text, delay_secs) = match parse_delay_arg(args) {
        Ok(parsed) => parsed,
        Err(msg) => {
            XigmaBot::reply(ctx, &msg, true).await?;
            return Ok(());
        }
    };

    let payload = match build_broadcast_message(ctx, &text).await {
        Ok(msg) => msg,
        Err(_) => {
            XigmaBot::reply(
                ctx,
                "Contoh:\n- /bcg Halo semua grup -d=5\n- kirim/reply gambar|video|audio lalu /bcg <caption opsional> -d=3",
                true,
            )
            .await?;
            return Ok(());
        }
    };

    let all_groups = helper::fetch_all_groups_jids(ctx).await?;
    let total_groups = all_groups.len();
    if total_groups == 0 {
        XigmaBot::reply(
            ctx,
            "Bot belum terdeteksi ada di grup manapun.",
            true,
        )
        .await?;
        return Ok(());
    }

    let targets: Vec<String> = all_groups
        .into_iter()
        .filter(|group_id| !config::is_group_blacklisted(group_id))
        .collect();

    if targets.is_empty() {
        XigmaBot::reply(ctx, "Semua grup masuk blacklist, tidak ada target broadcast.", true).await?;
        return Ok(());
    }

    let mut success = 0usize;
    let mut failed = 0usize;

    for (idx, target) in targets.iter().enumerate() {
        if let Ok(jid) = target.parse::<Jid>()
            && ctx
                .client
                .send_message(jid, payload.clone())
                .await
                .is_ok()
        {
            success += 1;
        } else {
            failed += 1;
        }

        if idx + 1 < targets.len() {
            sleep(Duration::from_secs(delay_secs)).await;
        }
    }

    let skipped = total_groups.saturating_sub(targets.len());

    XigmaBot::reply(
        ctx,
        &format!(
            "Broadcast grup selesai.\nTotal grup: {}\nTerkirim: {}\nDiblacklist (skip): {}\nGagal: {}\nDelay: {} detik/grup",
            total_groups, success, skipped, failed, delay_secs
        ),
        true,
    )
    .await?;

    Ok(())
}
