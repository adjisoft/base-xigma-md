use crate::config;
use crate::controller::ytdl::types::YtAdInfo;
use crate::util::msg::XigmaBot;
use tokio::fs;
use wa::context_info::external_ad_reply_info as ad;
use waproto::whatsapp as wa;
use whatsapp_rust::bot::MessageContext;
use whatsapp_rust::download::MediaType;

fn build_message_context(ctx: &MessageContext, ad_info: Option<&YtAdInfo>) -> wa::ContextInfo {
    let mut info = wa::ContextInfo {
        stanza_id: Some(ctx.info.id.clone()),
        participant: Some(ctx.info.source.sender.to_string()),
        quoted_message: None,
        ..Default::default()
    };

    if let Some(ad_meta) = ad_info {
        let cfg = config::get_config();
        let ad_title = if ad_meta.title.is_empty() {
            "YouTube Downloader".to_string()
        } else {
            ad_meta.title.clone()
        };
        let ad_body = if ad_meta.author.is_empty() {
            "Sumber video YouTube".to_string()
        } else {
            format!("Author: {}", ad_meta.author)
        };
        let thumb_url = if ad_meta.thumbnail_url.is_empty() {
            cfg.thumbnail_url
        } else {
            ad_meta.thumbnail_url.clone()
        };
        let source = if ad_meta.source_url.is_empty() {
            "https://youtube.com".to_string()
        } else {
            ad_meta.source_url.clone()
        };

        info.external_ad_reply = Some(wa::context_info::ExternalAdReplyInfo {
            title: Some(ad_title),
            body: Some(ad_body),
            media_type: Some(ad::MediaType::Image as i32),
            thumbnail_url: Some(thumb_url),
            media_url: Some(source.clone()),
            render_larger_thumbnail: Some(true),
            show_ad_attribution: Some(false),
            thumbnail: None,
            source_type: None,
            source_id: None,
            source_url: Some(source.clone()),
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
            ad_preview_url: Some(source),
        });
    }

    info
}

pub async fn send_audio_file(
    ctx: &MessageContext,
    audio_path: &std::path::Path,
    caption: &str,
    ad_info: Option<&YtAdInfo>,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(audio_path).await?;
    let upload = ctx.client.upload(bytes, MediaType::Audio).await?;
    let context_info = build_message_context(ctx, ad_info);

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
            context_info: Some(Box::new(context_info)),
            ..Default::default()
        })),
        ..Default::default()
    };

    ctx.send_message(msg).await?;
    XigmaBot::reply(ctx, caption, true).await?;
    Ok(())
}

pub async fn send_video_file(
    ctx: &MessageContext,
    video_path: &std::path::Path,
    caption: &str,
    ad_info: Option<&YtAdInfo>,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(video_path).await?;
    let upload = ctx.client.upload(bytes, MediaType::Video).await?;
    let context_info = build_message_context(ctx, ad_info);

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
            context_info: Some(Box::new(context_info)),
            ..Default::default()
        })),
        ..Default::default()
    };

    ctx.send_message(msg).await?;
    Ok(())
}

pub async fn send_image_file(
    ctx: &MessageContext,
    image_path: &std::path::Path,
    caption: &str,
    ad_info: Option<&YtAdInfo>,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(image_path).await?;
    let upload = ctx.client.upload(bytes, MediaType::Image).await?;
    let context_info = build_message_context(ctx, ad_info);

    let msg = wa::Message {
        image_message: Some(Box::new(wa::message::ImageMessage {
            mimetype: Some("image/png".to_string()),
            caption: Some(caption.to_string()),
            url: Some(upload.url),
            direct_path: Some(upload.direct_path),
            media_key: Some(upload.media_key),
            file_enc_sha256: Some(upload.file_enc_sha256),
            file_sha256: Some(upload.file_sha256),
            file_length: Some(upload.file_length),
            context_info: Some(Box::new(context_info)),
            ..Default::default()
        })),
        ..Default::default()
    };

    ctx.send_message(msg).await?;
    Ok(())
}
