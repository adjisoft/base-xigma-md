use crate::config;
use crate::util::msg::XigmaBot;
use whatsapp_rust::bot::MessageContext;

fn jid_to_number(jid: &str) -> String {
    jid.split('@')
        .next()
        .unwrap_or(jid)
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect()
}

fn resolve_owner_target(ctx: &MessageContext, args: &str) -> Option<String> {
    let info = ctx
        .message
        .extended_text_message
        .as_ref()
        .and_then(|ext| ext.context_info.as_ref());

    if let Some(mentioned) = info.and_then(|i| i.mentioned_jid.first()) {
        let number = jid_to_number(mentioned);
        if !number.is_empty() {
            return Some(number);
        }
    }

    if let Some(ctx_info) = info {
        if ctx_info.quoted_message.is_some() {
            if let Some(participant) = &ctx_info.participant {
                let number = jid_to_number(participant);
                if !number.is_empty() {
                    return Some(number);
                }
            }
        }
    }

    let number: String = args.chars().filter(|c| c.is_ascii_digit()).collect();
    if number.is_empty() {
        None
    } else {
        Some(number)
    }
}

async fn ensure_owner(ctx: &MessageContext) -> Result<bool, Box<dyn std::error::Error>> {
    let sender = ctx.info.source.sender.to_string();
    if config::is_owner(&sender) {
        return Ok(true);
    }

    let debug = config::owner_debug_info(&sender);
    XigmaBot::reply(
        ctx,
        &format!("Hanya owner yang boleh memakai fitur ini.\n\n{}", debug),
        true,
    )
    .await?;
    Ok(false)
}

pub async fn add_owner(
    ctx: &MessageContext,
    args: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !ensure_owner(ctx).await? {
        return Ok(());
    }

    let Some(target) = resolve_owner_target(ctx, args) else {
        XigmaBot::reply(
            ctx,
            "Contoh: /addowner <tag/reply/nomor (tanpa awalan +)>",
            true,
        )
        .await?;
        return Ok(());
    };

    match config::add_owner(&target) {
        Ok(true) => XigmaBot::reply(ctx, "Owner baru berhasil ditambahkan.", true).await?,
        Ok(false) => XigmaBot::reply(ctx, "Nomor itu sudah terdaftar sebagai owner.", true).await?,
        Err(e) => XigmaBot::reply(ctx, &format!("Gagal menambah owner: {}", e), true).await?,
    }

    Ok(())
}

pub async fn set_thumbnail(
    ctx: &MessageContext,
    args: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !ensure_owner(ctx).await? {
        return Ok(());
    }

    if args.trim().is_empty() {
        XigmaBot::reply(ctx, "Contoh: .setthumb https://example.com/image.jpg", true).await?;
        return Ok(());
    }

    match config::set_thumbnail_url(args) {
        Ok(()) => XigmaBot::reply(ctx, "Thumbnail bot berhasil diubah.", true).await?,
        Err(e) => XigmaBot::reply(ctx, &format!("Gagal ubah thumbnail: {}", e), true).await?,
    }

    Ok(())
}
