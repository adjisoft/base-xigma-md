use crate::config;
use ron::ser::PrettyConfig;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use waproto::whatsapp as wa;
use whatsapp_rust::bot::MessageContext;
use whatsapp_rust::download::MediaType;
use whatsapp_rust::proto_helpers::MessageExt;

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
    if let Some(doc) = &base.document_message {
        return doc.context_info.as_deref();
    }
    None
}

pub async fn handle(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    let raw_sender = ctx.info.source.sender.to_string();
    let sender = raw_sender.split('@').next().unwrap_or("");

    if !config::is_owner(sender) {
        let reply_msg = wa::Message {
            extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
                text: Some("⚠️ Hanya owner yang boleh menggunakan command ini.".to_string()),
                ..Default::default()
            })),
            ..Default::default()
        };

        let _ = ctx.send_message(reply_msg).await;
        return Ok(());
    }

    let base = ctx.message.get_base_message();
    let quoted_msg_struct = extract_context_info(base).and_then(|info| info.quoted_message.as_ref());

    let ron_text = match quoted_msg_struct {
        Some(msg) => ron::ser::to_string_pretty(msg, PrettyConfig::new().separate_tuple_members(true))
            .unwrap_or_else(|_| format!("(fallback_debug: {:?})", msg)),
        None => {
            let _ = ctx
                .send_message(wa::Message {
                    extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
                        text: Some("Pesan yang diquoted mungkin tidak didukung atau belum diquote!.".to_string()),
                        ..Default::default()
                    })),
                    ..Default::default()
                })
                .await;
            return Ok(());
        }
    };

    let ron_bytes = ron_text.into_bytes();
    let upload = ctx
        .client
        .upload(ron_bytes.clone(), MediaType::Document)
        .await?;

    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let file_name = format!("debug-message-{}.ron", ts);
    let debug_dir = Path::new("message_debugs");
    if !debug_dir.exists() {
        fs::create_dir_all(debug_dir)?;
    }
    fs::write(debug_dir.join(&file_name), &ron_bytes)?;

    let reply_msg = wa::Message {
        document_message: Some(Box::new(wa::message::DocumentMessage {
            mimetype: Some("application/ron".to_string()),
            title: Some(file_name.clone()),
            file_name: Some(file_name),
            caption: Some("Debug result".to_string()),
            url: Some(upload.url),
            direct_path: Some(upload.direct_path),
            media_key: Some(upload.media_key),
            file_enc_sha256: Some(upload.file_enc_sha256),
            file_sha256: Some(upload.file_sha256),
            file_length: Some(upload.file_length),
            ..Default::default()
        })),
        ..Default::default()
    };

    if let Err(e) = ctx.send_message(reply_msg).await {
        eprintln!("Error pada controller debug: {}", e);
    }
    Ok(())
}
