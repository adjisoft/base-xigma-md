use crate::{config, util::msg::XigmaBot};
use whatsapp_rust::bot::MessageContext;

fn normalize_option(raw: &str) -> String {
    raw.to_ascii_lowercase()
        .replace('_', "")
        .replace('-', "")
}

fn usage_text() -> String {
    let cfg = config::get_config();
    format!(
        "Penggunaan:\n\
/set queue_delay <detik>\n\
/set queue-delay <detik>\n\
/set broadcast_delay <detik>\n\
/set broadcast-delay <detik>\n\
/set botmode <public|self>\n\
/set modebot <public|self>\n\
/set thumbnail <url>\n\n\
Nilai saat ini:\n\
- queue_delay: {}\n\
- broadcast_delay: {}\n\
- botmode: {}\n\
- thumbnail: {}",
        cfg.queue_delay.max(1),
        cfg.broadcast_delay.max(1),
        cfg.bot_mode.to_lowercase(),
        cfg.thumbnail_url
    )
}

pub async fn handle(ctx: &MessageContext, args: &str) -> Result<(), Box<dyn std::error::Error>> {
    let sender = ctx.info.source.sender.to_string();
    if !config::is_owner(&sender) {
        XigmaBot::reply(ctx, "Hanya owner yang boleh memakai fitur ini.", true).await?;
        return Ok(());
    }

    let raw = args.trim();
    if raw.is_empty() {
        XigmaBot::reply(ctx, &usage_text(), true).await?;
        return Ok(());
    }

    let mut parts = raw.splitn(2, char::is_whitespace);
    let option_raw = parts.next().unwrap_or("").trim();
    let value = parts.next().unwrap_or("").trim();

    if option_raw.is_empty() || value.is_empty() {
        XigmaBot::reply(ctx, &usage_text(), true).await?;
        return Ok(());
    }

    let option = normalize_option(option_raw);
    match option.as_str() {
        "queuedelay" => match value.parse::<u64>() {
            Ok(secs) => match config::set_queue_delay(secs) {
                Ok(()) => {
                    XigmaBot::reply(ctx, &format!("queue_delay diubah ke {} detik.", secs), true)
                        .await?
                }
                Err(e) => XigmaBot::reply(ctx, &format!("Gagal set queue_delay: {}", e), true).await?,
            },
            Err(_) => {
                XigmaBot::reply(ctx, "Value queue_delay harus angka. Contoh: /set queue_delay 2", true)
                    .await?
            }
        },
        "broadcastdelay" => match value.parse::<u64>() {
            Ok(secs) => match config::set_broadcast_delay(secs) {
                Ok(()) => {
                    XigmaBot::reply(
                        ctx,
                        &format!("broadcast_delay diubah ke {} detik.", secs),
                        true,
                    )
                    .await?
                }
                Err(e) => {
                    XigmaBot::reply(ctx, &format!("Gagal set broadcast_delay: {}", e), true).await?
                }
            },
            Err(_) => XigmaBot::reply(
                ctx,
                "Value broadcast_delay harus angka. Contoh: /set broadcast_delay 3",
                true,
            )
            .await?,
        },
        "botmode" | "modebot" => match config::set_bot_mode(value) {
            Ok(()) => {
                XigmaBot::reply(
                    ctx,
                    &format!("botmode diubah ke `{}`.", value.to_lowercase()),
                    true,
                )
                .await?
            }
            Err(e) => XigmaBot::reply(
                ctx,
                &format!("Gagal set botmode: {}\nContoh: /set botmode public", e),
                true,
            )
            .await?,
        },
        "thumbnail" => match config::set_thumbnail_url(value) {
            Ok(()) => XigmaBot::reply(ctx, "thumbnail berhasil diubah.", true).await?,
            Err(e) => XigmaBot::reply(ctx, &format!("Gagal set thumbnail: {}", e), true).await?,
        },
        _ => {
            XigmaBot::reply(ctx, &format!("Opsi tidak dikenal: `{}`\n\n{}", option_raw, usage_text()), true)
                .await?
        }
    }

    Ok(())
}
