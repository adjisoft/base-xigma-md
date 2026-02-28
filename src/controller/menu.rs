use crate::config;
use waproto::whatsapp::{self as wa};
use whatsapp_rust::bot::MessageContext;

pub async fn handle(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::get_config();
    let sender = ctx.info.source.sender.to_string();
    let sender_number = sender
        .split('@')
        .next()
        .unwrap_or("")
        .to_string();

    use wa::context_info::external_ad_reply_info as ad;
    let ad_info = wa::context_info::ExternalAdReplyInfo {
        title: Some(cfg.nama_bot.clone()),
        body: Some("Bot WhatsApp 100% Rust".to_string()),
        media_type: Some(ad::MediaType::Image as i32),
        thumbnail_url: Some(cfg.thumbnail_url.clone()),
        media_url: Some("https://github.com/magercode".to_string()),
        render_larger_thumbnail: Some(true),
        show_ad_attribution: Some(false),
        thumbnail: None,
        source_type: None,
        source_id: None,
        source_url: None,
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
        ad_preview_url: None,
    };

    let inner_ctx = wa::ContextInfo {
        stanza_id: Some(ctx.info.id.clone()),
        participant: Some(ctx.info.source.sender.to_string()),
        is_forwarded: Some(true),
        external_ad_reply: Some(ad_info),
        forwarding_score: Some(999),
        mentioned_jid: vec![sender],
        ..Default::default()
    };

    let menu_text = format!(
        r#"
Menu {0} 2026
Halo @{1}, bot WhatsApp ini dibuat dengan 100% Rust

Detail Bot:
Nama: Xigma-MD Beta
Prefix: / ! .
Author: MagerCode

List Menu:

Utama:
- menu
- ping
- debug
- owner

Downloader:
- igdl
- instagram
- igreel

Fun:
- roll
- dadu
- kapan
- when
- whenyah

Owner Tools (owner-only):
- /addowner <tag/reply/nomor (tanpa awalan +)>
- setthumb https://...
"#,
        cfg.nama_bot, sender_number
    );

    let msg = wa::Message {
        extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
            text: Some(menu_text.clone()),
            matched_text: Some(menu_text),
            context_info: Some(Box::new(inner_ctx)),
            ..Default::default()
        })),
        ..Default::default()
    };

    if let Err(e) = ctx.send_message(msg).await {
        eprintln!("menu error: {}", e);
    }

    Ok(())
}
