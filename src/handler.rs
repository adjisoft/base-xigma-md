use crate::{config, controller, util::msg::XigmaBot};
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use whatsapp_rust::bot::MessageContext;

const MAX_SPAM_PENALTY_SECS: u64 = 30;

#[derive(Clone, Copy)]
struct SpamState {
    last_command_at: Instant,
    blocked_until: Option<Instant>,
    strikes: u8,
}

static SPAM_GUARD: LazyLock<Mutex<HashMap<String, SpamState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

async fn blocked_by_mode(ctx: &MessageContext) -> Result<bool, Box<dyn std::error::Error>> {
    if !config::is_self_mode() {
        return Ok(false);
    }

    let sender = ctx.info.source.sender.to_string();
    if ctx.info.source.is_from_me || config::is_owner(&sender) {
        return Ok(false);
    }

    XigmaBot::reply(
        ctx,
        "Bot sedang di mode self. Hanya owner yang bisa memakai perintah.",
        true,
    )
    .await?;
    Ok(true)
}

async fn blocked_by_spam(ctx: &MessageContext) -> Result<bool, Box<dyn std::error::Error>> {
    let sender = ctx.info.source.sender.to_string();
    if ctx.info.source.is_from_me || config::is_owner(&sender) {
        return Ok(false);
    }
    let base_delay_secs = config::queue_delay_secs();
    let base_delay = Duration::from_secs(base_delay_secs);

    let key = sender;
    let now = Instant::now();
    let mut state_map = SPAM_GUARD.lock().await;
    let state = state_map.entry(key).or_insert(SpamState {
        last_command_at: now - base_delay,
        blocked_until: None,
        strikes: 0,
    });

    if let Some(until) = state.blocked_until
        && now < until
    {
        let remaining = until.duration_since(now).as_secs().max(1);
        drop(state_map);
        XigmaBot::reply(
            ctx,
            &format!("Terlalu cepat kirim perintah. Tunggu {} detik lagi.", remaining),
            true,
        )
        .await?;
        return Ok(true);
    }

    let since_last = now.duration_since(state.last_command_at);
    state.last_command_at = now;
    state.blocked_until = None;

    if since_last < base_delay {
        state.strikes = state.strikes.saturating_add(1).min(10);
        let penalty = (u64::from(state.strikes) * 2).min(MAX_SPAM_PENALTY_SECS);
        state.blocked_until = Some(now + Duration::from_secs(penalty));
        drop(state_map);
        XigmaBot::reply(
            ctx,
            &format!(
                "Anti-spam aktif. Kamu kena delay {} detik karena terlalu cepat.",
                penalty
            ),
            true,
        )
        .await?;
        return Ok(true);
    }

    if state.strikes > 0 {
        state.strikes -= 1;
    }

    Ok(false)
}

pub async fn dispatch(ctx: &MessageContext, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let text = text.trim();

    println!("Pesan {} -> {}", ctx.info.source.sender, text);
    //println!("{:#?}", ctx.info);

    let mut chars = text.chars();

    let _prefix = match chars.next() {
        Some(c) if "/.!".contains(c) => c,
        _ => return Ok(()),
    };

    let noprefix = chars.as_str();
    let cmd_end = noprefix
        .find(char::is_whitespace)
        .unwrap_or(noprefix.len());
    let cmd = noprefix[..cmd_end].trim();
    let args = if cmd_end < noprefix.len() {
        noprefix[cmd_end..].trim_start().to_string()
    } else {
        String::new()
    };

    if blocked_by_mode(ctx).await? {
        return Ok(());
    }

    if blocked_by_spam(ctx).await? {
        return Ok(());
    }

    match cmd {
        /*@> menu & testing<@*/
        "menu" | "help" => controller::menu::handle(ctx).await?,
        "debug" | "d" => controller::debug::handle(ctx).await?,
        "getlid" | "lid" => controller::get_lid::handle(ctx).await?,

        /*@> fun & downloader <@*/
        "dadu" | "roll" => controller::fun_dadu::handle(ctx).await?,
        "ping" | "speed" => controller::ping::handle(ctx).await?,
        "owner" | "own" => controller::owner::handle(ctx).await?,
        "addowner" => controller::owner_tools::add_owner(ctx, &args).await?,
        "setthumb" | "setthumbnail" => controller::owner_tools::set_thumbnail(ctx, &args).await?,
        "set" => controller::settings::handle(ctx, &args).await?,
        "requestpay" | "requestpayment" | "rpm" | "test" => {
            controller::request_payment_message::handle(ctx, &args).await?
        },
        "gid" | "groupid" => controller::group_tools::group_id(ctx).await?,
        "bl" | "blacklist" => controller::group_tools::blacklist(ctx, &args).await?,
        "bcg" | "bcgroup" | "broadcastgroup" => {
            controller::group_tools::broadcast_groups(ctx, &args).await?
        }
        "gstatus" | "swgc" => controller::swgc::handle(ctx, &args).await?,
        "ytsearch" => controller::ytdl::ytsearch(ctx, &args).await?,
        "ytmp3" => controller::ytdl::ytmp3(ctx, &args).await?,
        "ytmp4" => controller::ytdl::ytmp4(ctx, &args).await?,
        "aio" => controller::ytdl::aio(ctx, &args).await?,
        "play" | "song" => controller::ytdl::play_or_song(ctx, &args).await?,
        "sticker" | "s" | "stiker" => controller::sticker::to_sticker(ctx).await?,
        "toimg" | "tovid" => controller::sticker::sticker_to_media(ctx).await?,
        "igdl" | "igreel" | "instagram" => {
            if args.is_empty() {
                let _ = XigmaBot::reply(ctx, "📌 *Contoh:* .igdl https://instagram.com/xxx", true)
                    .await?;
                return Ok(());
            }
            controller::downloader_igreel::handle(ctx, &args).await?
        }
        "kapan" | "when" | "whenyah" => {
            if args.is_empty() {
                let _ = XigmaBot::reply(ctx, "Contoh: .kapan aku kaya", true).await;
                return Ok(());
            }
            controller::fun_kapan::handle(ctx, &args).await?
        }
        _ => {
            XigmaBot::react(ctx, "🦀").await?;
        }
    }

    Ok(())
}
