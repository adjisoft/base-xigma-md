use crate::{config, util::msg::XigmaBot};
use std::time::Duration;
use tokio::time::sleep;
use wacore_binary::jid::Jid;
use waproto::whatsapp::{self as wa};
use whatsapp_rust::bot::MessageContext;
use whatsapp_rust::download::MediaType;
use whatsapp_rust::proto_helpers::MessageExt;

enum MediaRef<'a> {
    Image(&'a wa::message::ImageMessage),
    Video(&'a wa::message::VideoMessage),
    Audio(&'a wa::message::AudioMessage),
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

fn status_text_style(text: &str) -> wa::Message {
    let palette: [u32; 6] = [
        0xFF1F2A44, 0xFF5C2E91, 0xFF106D5B, 0xFF8A3B12, 0xFF9B1C1C, 0xFF1B5E20,
    ];
    let idx = text
        .bytes()
        .fold(0usize, |acc, b| acc.wrapping_add(b as usize))
        % palette.len();

    wa::Message {
        extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
            text: Some(text.to_string()),
            text_argb: Some(0xFFFFFFFF),
            background_argb: Some(palette[idx]),
            font: Some(wa::message::extended_text_message::FontType::SystemBold as i32),
            ..Default::default()
        })),
        ..Default::default()
    }
}

pub async fn handle(ctx: &MessageContext, args: &str) -> Result<(), Box<dyn std::error::Error>> {
    let sender = ctx.info.source.sender.to_string();
    if !config::is_owner(&sender) {
        XigmaBot::reply(ctx, "Hanya owner yang boleh memakai fitur ini.", true).await?;
        return Ok(());
    }

    let text = args.trim().to_string();
    let delay_secs = config::broadcast_delay_secs();

    let nested_message = if let Some(media) = resolve_media_target(ctx) {
        match media {
            MediaRef::Image(img) => {
                let data = ctx.client.download(img).await?;
                let upload = ctx.client.upload(data, MediaType::Image).await?;

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
                            Some(text.clone())
                        },
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
                            Some(text.clone())
                        },
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
                        seconds: aud.seconds,
                        ptt: aud.ptt,
                        ..Default::default()
                    })),
                    ..Default::default()
                }
            }
        }
    } else {
        if text.is_empty() {
            XigmaBot::reply(
                ctx,
                "Contoh:\n- /swgc teks status\n- reply gambar/video/audio lalu /swgc <caption opsional>",
                true,
            )
            .await?;
            return Ok(());
        }

        status_text_style(&text)
    };

    let group_status_message_v2 = wa::Message {
        group_status_message_v2: Some(Box::new(wa::message::FutureProofMessage {
            message: Some(Box::new(nested_message)),
        })),
        ..Default::default()
    };

    let all_groups = ctx.client.groups().get_participating().await?;
    if all_groups.is_empty() {
        XigmaBot::reply(ctx, "Bot belum terdeteksi ada di grup manapun.", true).await?;
        return Ok(());
    }

    let mut group_ids: Vec<String> = all_groups.keys().cloned().collect();
    group_ids.sort();

    let mut success = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for (idx, group_id) in group_ids.iter().enumerate() {
        if config::is_group_blacklisted(group_id) {
            skipped += 1;
            continue;
        }

        if let Ok(jid) = group_id.parse::<Jid>()
            && ctx
                .client
                .send_message(jid, group_status_message_v2.clone())
                .await
                .is_ok()
        {
            success += 1;
        } else {
            failed += 1;
        }

        if idx + 1 < group_ids.len() {
            sleep(Duration::from_secs(delay_secs)).await;
        }
    }

    XigmaBot::reply(
        ctx,
        &format!(
            "SWGC broadcast selesai.\nTotal grup: {}\nTerkirim: {}\nDiblacklist (skip): {}\nGagal: {}\nDelay: {} detik/grup",
            group_ids.len(),
            success,
            skipped,
            failed,
            delay_secs
        ),
        true,
    )
    .await?;

    Ok(())
}
