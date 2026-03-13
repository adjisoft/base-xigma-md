use crate::{config, util::helper, util::msg::XigmaBot};
use std::time::{SystemTime, UNIX_EPOCH};
use wacore_binary::builder::NodeBuilder;
use wacore_binary::jid::Jid;
use wacore_binary::node::NodeContent;
use waproto::whatsapp as wa;
use whatsapp_rust::bot::MessageContext;
use whatsapp_rust::proto_helpers::MessageExt;
use whatsapp_rust::request::InfoQuery;

fn is_group_chat(ctx: &MessageContext) -> bool {
    ctx.info.source.is_group || ctx.info.source.chat.to_string().ends_with("@g.us")
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
    if let Some(doc) = &base.document_message {
        return doc.context_info.as_deref();
    }
    if let Some(audio) = &base.audio_message {
        return audio.context_info.as_deref();
    }
    None
}

fn normalize_phone_number(raw: &str) -> Option<String> {
    let mut number: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if number.starts_with("00") {
        number.replace_range(0..2, "");
    }
    if let Some(rest) = number.strip_prefix('0') {
        number = format!("62{}", rest);
    } else if !number.starts_with("62") && number.starts_with('8') {
        number = format!("62{}", number);
    }

    if number.len() < 8 {
        None
    } else {
        Some(number)
    }
}

fn parse_participant_jid(raw: &str) -> Result<Jid, Box<dyn std::error::Error>> {
    let jid_str = format!("{}@s.whatsapp.net", raw);
    Ok(jid_str.parse::<Jid>()?)
}

fn parse_target_jid_from_text(raw: &str) -> Option<Jid> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.contains('@') {
        return trimmed.parse::<Jid>().ok();
    }

    let normalized = normalize_phone_number(trimmed.trim_start_matches('@'))?;
    parse_participant_jid(&normalized).ok()
}

fn resolve_target_from_mention_or_reply(ctx: &MessageContext) -> Option<Jid> {
    let info = extract_context_info(ctx.message.get_base_message())?;

    if let Some(mentioned) = info.mentioned_jid.first() {
        if let Ok(jid) = mentioned.parse::<Jid>() {
            return Some(jid);
        }
    }

    if info.quoted_message.is_some()
        && let Some(participant) = &info.participant
    {
        if let Ok(jid) = participant.parse::<Jid>() {
            return Some(jid);
        }
    }

    None
}

fn resolve_quoted_key(ctx: &MessageContext) -> Option<wa::MessageKey> {
    let info = extract_context_info(ctx.message.get_base_message())?;
    let stanza_id = info.stanza_id.clone()?;

    Some(wa::MessageKey {
        remote_jid: Some(ctx.info.source.chat.to_string()),
        id: Some(stanza_id),
        from_me: Some(false),
        participant: info.participant.clone(),
    })
}

async fn is_actor_allowed(
    ctx: &MessageContext,
    group_jid: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let sender = ctx.info.source.sender.to_string();
    if config::is_owner(&sender) {
        return Ok(true);
    }

    if helper::is_group_admin(ctx, group_jid, &sender).await? {
        return Ok(true);
    }

    XigmaBot::reply(ctx, "Perintah ini hanya bisa dipakai admin grup atau owner bot.", true)
        .await?;
    Ok(false)
}

async fn ensure_group_context(ctx: &MessageContext) -> Result<String, Box<dyn std::error::Error>> {
    if !is_group_chat(ctx) {
        XigmaBot::reply(ctx, "Perintah ini hanya bisa dipakai di grup.", true).await?;
        return Err("not a group chat".into());
    }

    Ok(ctx.info.source.chat.to_string())
}

async fn ensure_bot_is_admin(
    ctx: &MessageContext,
    group_jid: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let own_pn = ctx.client.get_pn().await;
    let own_lid = ctx.client.get_lid().await;

    println!(
        "[ensure-bot-admin] group={} own_pn={} own_lid={}",
        group_jid,
        own_pn
            .as_ref()
            .map(|jid| jid.to_string())
            .unwrap_or_else(|| "-".to_string()),
        own_lid
            .as_ref()
            .map(|jid| jid.to_string())
            .unwrap_or_else(|| "-".to_string())
    );

    let is_admin = if let Some(jid) = own_pn.as_ref() {
        helper::is_group_admin(ctx, group_jid, &jid.to_string()).await?
    } else {
        false
    } || if let Some(jid) = own_lid.as_ref() {
        helper::is_group_admin(ctx, group_jid, &jid.to_string()).await?
    } else {
        false
    };

    if is_admin {
        return Ok(());
    }

    XigmaBot::reply(ctx, "Bot harus jadi admin grup dulu untuk menjalankan perintah ini.", true)
        .await?;
    Err("bot is not admin".into())
}

async fn send_group_participant_iq(
    ctx: &MessageContext,
    group_jid: &str,
    action: &str,
    participants: &[Jid],
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "[group-participant-iq] action={} group={} participants={}",
        action,
        group_jid,
        participants
            .iter()
            .map(|jid| jid.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let group = group_jid.parse::<Jid>()?;
    let participant_nodes: Vec<_> = participants
        .iter()
        .map(|jid| NodeBuilder::new("participant").attr("jid", jid.to_string()).build())
        .collect();
    let action_node = NodeBuilder::new(action).children(participant_nodes).build();

    ctx.client
        .send_iq(InfoQuery::set(
            "w:g2",
            group,
            Some(NodeContent::Nodes(vec![action_node])),
        ))
        .await?;

    Ok(())
}

async fn send_group_toggle_iq(
    ctx: &MessageContext,
    group_jid: &str,
    action: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let group = group_jid.parse::<Jid>()?;
    let action_node = NodeBuilder::new(action).build();

    ctx.client
        .send_iq(InfoQuery::set(
            "w:g2",
            group,
            Some(NodeContent::Nodes(vec![action_node])),
        ))
        .await?;

    Ok(())
}

pub async fn kick(ctx: &MessageContext, args: &str) -> Result<(), Box<dyn std::error::Error>> {
    let group_jid = match ensure_group_context(ctx).await {
        Ok(jid) => jid,
        Err(_) => return Ok(()),
    };

    if !is_actor_allowed(ctx, &group_jid).await? {
        return Ok(());
    }
    if ensure_bot_is_admin(ctx, &group_jid).await.is_err() {
        return Ok(());
    }

    let Some(target_jid) = resolve_target_from_mention_or_reply(ctx)
        .or_else(|| parse_target_jid_from_text(args))
    else {
        XigmaBot::reply(ctx, "Contoh: /kick @tag atau reply pesan target dengan /kick", true)
            .await?;
        return Ok(());
    };

    println!(
        "[kick] sender={} group={} raw_args='{}' resolved_target={}",
        ctx.info.source.sender,
        group_jid,
        args,
        target_jid
    );

    send_group_participant_iq(ctx, &group_jid, "remove", &[target_jid]).await?;

    XigmaBot::reply(ctx, "Member berhasil dikeluarkan.", true).await?;
    Ok(())
}

pub async fn add(ctx: &MessageContext, args: &str) -> Result<(), Box<dyn std::error::Error>> {
    let group_jid = match ensure_group_context(ctx).await {
        Ok(jid) => jid,
        Err(_) => return Ok(()),
    };

    if !is_actor_allowed(ctx, &group_jid).await? {
        return Ok(());
    }
    if ensure_bot_is_admin(ctx, &group_jid).await.is_err() {
        return Ok(());
    }

    let raw_numbers: Vec<&str> = args
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect();

    if raw_numbers.is_empty() {
        XigmaBot::reply(ctx, "Contoh: /add 62812xxxx,62813xxxx", true).await?;
        return Ok(());
    }

    let mut participants = Vec::new();
    let mut invalid = Vec::new();

    for raw in raw_numbers {
        match normalize_phone_number(raw) {
            Some(number) => participants.push(parse_participant_jid(&number)?),
            None => invalid.push(raw.to_string()),
        }
    }

    if participants.is_empty() {
        XigmaBot::reply(ctx, "Semua nomor tidak valid. Contoh: /add 62812xxxx,62813xxxx", true)
            .await?;
        return Ok(());
    }

    send_group_participant_iq(ctx, &group_jid, "add", &participants).await?;

    if invalid.is_empty() {
        XigmaBot::reply(ctx, "Member berhasil ditambahkan.", true).await?;
    } else {
        XigmaBot::reply(
            ctx,
            &format!(
                "Sebagian nomor berhasil diproses. Nomor tidak valid: {}",
                invalid.join(", ")
            ),
            true,
        )
        .await?;
    }

    Ok(())
}

pub async fn group(ctx: &MessageContext, args: &str) -> Result<(), Box<dyn std::error::Error>> {
    let group_jid = match ensure_group_context(ctx).await {
        Ok(jid) => jid,
        Err(_) => return Ok(()),
    };

    if !is_actor_allowed(ctx, &group_jid).await? {
        return Ok(());
    }
    if ensure_bot_is_admin(ctx, &group_jid).await.is_err() {
        return Ok(());
    }

    match args.trim().to_ascii_lowercase().as_str() {
        "close" => {
            send_group_toggle_iq(ctx, &group_jid, "announcement").await?;
            XigmaBot::reply(ctx, "Grup berhasil ditutup. Hanya admin yang bisa kirim pesan.", true)
                .await?;
        }
        "open" => {
            send_group_toggle_iq(ctx, &group_jid, "not_announcement").await?;
            XigmaBot::reply(ctx, "Grup berhasil dibuka untuk semua member.", true).await?;
        }
        _ => {
            XigmaBot::reply(ctx, "Contoh: /group close atau /group open", true).await?;
        }
    }

    Ok(())
}

async fn send_pin_action(
    ctx: &MessageContext,
    pin_type: wa::message::pin_in_chat_message::Type,
) -> Result<(), Box<dyn std::error::Error>> {
    let group_jid = match ensure_group_context(ctx).await {
        Ok(jid) => jid,
        Err(_) => return Ok(()),
    };

    if !is_actor_allowed(ctx, &group_jid).await? {
        return Ok(());
    }
    if ensure_bot_is_admin(ctx, &group_jid).await.is_err() {
        return Ok(());
    }

    let Some(mut key) = resolve_quoted_key(ctx) else {
        XigmaBot::reply(ctx, "Reply pesan target lalu kirim /pin atau /unpin", true).await?;
        return Ok(());
    };

    let own_pn = ctx.client.get_pn().await;
    let own_lid = ctx.client.get_lid().await;
    let participant = key.participant.clone().unwrap_or_default();
    let is_from_me = own_pn
        .as_ref()
        .map(|jid| helper::same_jid_identity(&participant, &jid.to_string()))
        .unwrap_or(false)
        || own_lid
            .as_ref()
            .map(|jid| helper::same_jid_identity(&participant, &jid.to_string()))
            .unwrap_or(false);
    key.from_me = Some(is_from_me);

    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64;
    let msg = wa::Message {
        pin_in_chat_message: Some(wa::message::PinInChatMessage {
            key: Some(key),
            r#type: Some(pin_type as i32),
            sender_timestamp_ms: Some(ts),
        }),
        ..Default::default()
    };

    ctx.send_message(msg).await?;

    Ok(())
}

pub async fn pin(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    send_pin_action(ctx, wa::message::pin_in_chat_message::Type::PinForAll).await?;
    XigmaBot::reply(ctx, "Pesan berhasil dipin.", true).await?;
    Ok(())
}

pub async fn unpin(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    send_pin_action(ctx, wa::message::pin_in_chat_message::Type::UnpinForAll).await?;
    XigmaBot::reply(ctx, "Pin pesan berhasil dilepas.", true).await?;
    Ok(())
}
