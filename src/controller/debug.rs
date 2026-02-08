use crate::config::NOMER_OWNER;
use waproto::whatsapp as wa;
use whatsapp_rust::bot::MessageContext;

pub async fn handle(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    let raw_sender = ctx.info.source.sender.to_string();
    let sender = raw_sender.split('@').next().unwrap_or("");

    if !NOMER_OWNER.contains(&sender) {
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

    let quoted_msg_struct = ctx
        .message
        .extended_text_message
        .as_ref()
        .and_then(|ext| ext.context_info.as_ref())
        .and_then(|info| info.quoted_message.as_ref());

    let debug_text = match quoted_msg_struct {
        Some(msg) => format!("```\n{:#?}\n```", msg),
        None => "Pesan yang diquoted mungkin tidak didukung atau belum diquote!.".to_string(),
    };

    let reply_msg = wa::Message {
        extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
            text: Some(debug_text),
            ..Default::default()
        })),
        ..Default::default()
    };

    if let Err(e) = ctx.send_message(reply_msg).await {
        eprintln!("Error pada controller debug: {}", e);
    }
    Ok(())
}
