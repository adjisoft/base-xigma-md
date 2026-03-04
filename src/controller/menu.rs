use crate::{config, util::msg::XigmaBot};
use std::collections::HashSet;
use std::fs;
use waproto::whatsapp::{self as wa};
use whatsapp_rust::bot::MessageContext;

const MENU_TEXT_PATH: &str = "menu.txt";
const HANDLER_SOURCE_PATH: &str = "src/handler.rs";

fn command_hint(cmd: &str) -> &'static str {
    match cmd {
        "ytsearch" => "<kueri>",
        "ytmp3" => "<url>",
        "ytmp4" => "<url>",
        "play" | "song" => "<kueri lagu>",
        "igdl" | "igreel" | "instagram" => "<url>",
        "kapan" | "when" | "whenyah" => "<teks>",
        "mode" | "setmode" => "<self|public>",
        "addowner" => "<nomor/tag/reply>",
        "setthumb" | "setthumbnail" => "<url>",
        "bl" | "blacklist" => "<id_grup (opsional)>",
        "bcg" | "bcgroup" | "broadcastgroup" => "<teks/reply media> -d=<detik>",
        "gstatus" | "swgc" => "<teks/reply media> -d=<detik>",
        "ibtn" | "interactive" | "button" | "buttons" => "<all|list|quick|url> [teks]",
        _ => "",
    }
}

fn extract_commands_from_handler() -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(HANDLER_SOURCE_PATH)?;
    let mut rows = Vec::new();

    for line in source.lines() {
        if !line.contains("=>") {
            continue;
        }

        let left = line.split("=>").next().unwrap_or("").trim();
        if left.is_empty() || !left.contains('"') {
            continue;
        }

        let mut aliases = Vec::new();
        for token in left.split('|') {
            let t = token.trim();
            if !t.starts_with('"') {
                continue;
            }
            if let Some(end) = t[1..].find('"') {
                let cmd = &t[1..1 + end];
                if !cmd.is_empty() {
                    aliases.push(cmd.to_string());
                }
            }
        }

        if !aliases.is_empty() {
            rows.push(aliases);
        }
    }

    Ok(rows)
}

fn build_menu_text(bot_name: &str, sender_number: &str, mode: &str, owners: usize) -> String {
    let commands = extract_commands_from_handler().unwrap_or_default();
    let mut seen = HashSet::new();
    let mut lines = Vec::new();

    for aliases in commands {
        let unique_aliases: Vec<String> = aliases
            .into_iter()
            .filter(|a| seen.insert(a.clone()))
            .collect();

        if unique_aliases.is_empty() {
            continue;
        }

        let main = unique_aliases.first().cloned().unwrap_or_default();
        let hint = command_hint(&main);
        let alias_view = unique_aliases
            .iter()
            .map(|a| format!("/{}", a))
            .collect::<Vec<String>>()
            .join(", ");

        if hint.is_empty() {
            lines.push(format!("• *{}*", alias_view));
        } else {
            lines.push(format!("• *{}* {}", alias_view, hint));
        }
    }

    format!(
        "Menu {bot_name} 2026\n\
Halo @{sender_number}\n\n\
Informasi Bot:\n\
- Nama: {bot_name}\n\
- Prefix: / ! .\n\
- Mode: {mode}\n\
- Total owner: {owners}\n\n\
Daftar Menu (auto-sync):\n\
{commands}\n",
        bot_name = bot_name,
        sender_number = sender_number,
        mode = mode,
        owners = owners,
        commands = lines.join("\n"),
    )
}

pub async fn handle(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::get_config();
    let sender = ctx.info.source.sender.to_string();
    let sender_number = sender.split('@').next().unwrap_or("").to_string();

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

    let inner_ctx = wa::ContextInfo {
        stanza_id: Some(ctx.info.id.clone()),
        participant: Some(ctx.info.source.sender.to_string()),
        is_forwarded: Some(true),
        external_ad_reply: Some(ad_info),
        forwarding_score: Some(999),
        mentioned_jid: vec![sender],
        ..Default::default()
    };

    let menu_text = build_menu_text(
        &cfg.nama_bot,
        &sender_number,
        &cfg.bot_mode.to_lowercase(),
        cfg.no_owner.len(),
    );
    let _ = fs::write(MENU_TEXT_PATH, &menu_text);

    let msg = wa::Message {
        extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
            text: Some(menu_text.clone()),
            matched_text: Some(menu_text),
            context_info: Some(Box::new(inner_ctx)),
            ..Default::default()
        })),
        ..Default::default()
    };

    XigmaBot::react(ctx, "🦀").await?;
    if let Err(e) = ctx.send_message(msg).await {
        eprintln!("menu error: {}", e);
    }

    Ok(())
}
