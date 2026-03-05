use crate::util::msg::XigmaBot;
use waproto::whatsapp as wa;
use whatsapp_rust::bot::MessageContext;
use whatsapp_rust::proto_helpers::MessageExt;

fn is_group_chat(ctx: &MessageContext) -> bool {
    ctx.info.source.chat.to_string().ends_with("@g.us")
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
    if let Some(doc) = &base.document_message {
        return doc.context_info.as_deref();
    }
    None
}

fn resolve_target_lid(ctx: &MessageContext) -> Option<String> {
    let base = ctx.message.get_base_message();
    let info = extract_context_info(base)?;

    if info.quoted_message.is_some()
        && let Some(participant) = &info.participant
    {
        let lid = participant.trim();
        if !lid.is_empty() {
            return Some(lid.to_string());
        }
    }

    if let Some(mentioned) = info.mentioned_jid.first() {
        let lid = mentioned.trim();
        if !lid.is_empty() {
            return Some(lid.to_string());
        }
    }

    None
}

pub async fn handle(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    if !is_group_chat(ctx) {
        XigmaBot::reply(
            ctx,
            "Perintah ini hanya untuk grup.\nGunakan dengan reply pesan user atau tag @user.",
            true,
        )
        .await?;
        return Ok(());
    }

    let Some(lid) = resolve_target_lid(ctx) else {
        XigmaBot::reply(
            ctx,
            "Tidak ada target.\nReply pesan user atau tag @user untuk ambil LID.",
            true,
        )
        .await?;
        return Ok(());
    };

    XigmaBot::reply(ctx, &format!("LID pengguna:\n{}", lid), true).await?;
    Ok(())
}
