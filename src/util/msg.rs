use crate::config;
use wa::context_info::external_ad_reply_info as ad;
use wacore::download::MediaType;
use waproto::whatsapp::{self as wa};
use whatsapp_rust::bot::MessageContext;
use whatsapp_rust::upload::UploadResponse;
//use crate::util::img;

#[warn(dead_code)]
pub struct XigmaBot;

impl XigmaBot {
    pub async fn reply(
        ctx: &MessageContext,
        text: &str,
        quoted: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut context_info = wa::ContextInfo::default();
        if quoted {
            context_info.stanza_id = Some(ctx.info.id.clone());
            context_info.participant = Some(ctx.info.source.sender.to_string());
            context_info.quoted_message = Some(ctx.message.clone());
        }
        let msg = wa::Message {
            extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
                text: Some(text.to_string()),
                context_info: Some(Box::new(context_info)),
                ..Default::default()
            })),
            ..Default::default()
        };
        ctx.send_message(msg)
            .await
            .map(|_| ())
            .map_err(|e| e.into())
    }

    pub async fn reply_ad(
        ctx: &MessageContext,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cfg = config::get_config();
        let sender = ctx.info.source.sender.to_string();

        let ad_info = wa::context_info::ExternalAdReplyInfo {
            title: Some(cfg.nama_bot.clone()),
            body: Some("Bot WhatsApp 100% Rust 🦀".to_string()),
            media_type: Some(ad::MediaType::Image as i32),
            thumbnail_url: Some(cfg.thumbnail_url.clone()),
            media_url: Some("https://github.com/magercode".to_string()),
            render_larger_thumbnail: Some(false),
            show_ad_attribution: Some(false),
            ..Default::default()
        };

        let ctx_info = wa::ContextInfo {
            external_ad_reply: Some(ad_info),
            mentioned_jid: vec![sender],
            ..Default::default()
        };

        let msg = wa::Message {
            extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
                text: Some(text.to_string()),
                context_info: Some(Box::new(ctx_info)),
                ..Default::default()
            })),
            ..Default::default()
        };

        ctx.send_message(msg).await?;
        Ok(())
    }

    pub async fn react(
        ctx: &MessageContext,
        emoji: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64;

        let reaction = wa::Message {
            reaction_message: Some(wa::message::ReactionMessage {
                key: Some(wa::MessageKey {
                    remote_jid: Some(ctx.info.source.chat.to_string()),
                    from_me: Some(true),
                    id: Some(ctx.info.id.clone()),
                    participant: None,
                    ..Default::default()
                }),
                text: Some(emoji.to_string()),
                grouping_key: None,
                sender_timestamp_ms: Some(ts),
            }),
            ..Default::default()
        };

        ctx.send_message(reaction).await?;
        Ok(())
    }

    pub async fn send_video(
        ctx: &MessageContext,
        video_url: &str,
        caption: &str,
        quoted: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut context_info = wa::ContextInfo::default();
        if quoted {
            context_info.stanza_id = Some(ctx.info.id.clone());
            context_info.participant = Some(ctx.info.source.sender.to_string());
            context_info.quoted_message = Some(ctx.message.clone());
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;

        let video_bytes = client.get(video_url).send().await?.bytes().await?;

        let upload: UploadResponse = ctx
            .client
            .upload(video_bytes.to_vec(), MediaType::Video)
            .await?;

        let video_msg = wa::message::VideoMessage {
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
        };

        ctx.send_message(wa::Message {
            video_message: Some(Box::new(video_msg)),
            ..Default::default()
        })
        .await?;

        Ok(())
    }

    fn build_vcard(name: &str, phone: &str) -> String {
        format!(
            "BEGIN:VCARD\n\
         VERSION:3.0\n\
         FN:{name}\n\
         TEL;type=CELL;type=VOICE;waid={phone}:{phone}\n\
         END:VCARD",
            name = name,
            phone = phone
        )
    }

    pub async fn send_contacts(
        ctx: &MessageContext,
        title: &str,
        contacts: Vec<(&str, &str)>,
        quoted: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut context_info = wa::ContextInfo::default();

        if quoted {
            context_info.stanza_id = Some(ctx.info.id.clone());
            context_info.participant = Some(ctx.info.source.sender.to_string());
            context_info.quoted_message = Some(ctx.message.clone());
        }

        let contact_list = contacts
            .into_iter()
            .map(|(name, phone)| wa::message::ContactMessage {
                display_name: Some(name.to_string()),
                vcard: Some(Self::build_vcard(name, phone)),
                context_info: None,
            })
            .collect();

        let msg = wa::Message {
            contacts_array_message: Some(Box::new(wa::message::ContactsArrayMessage {
                display_name: Some(title.to_string()),
                contacts: contact_list,
                context_info: Some(Box::new(context_info)),
            })),
            ..Default::default()
        };

        ctx.send_message(msg).await?;
        Ok(())
    }
}

