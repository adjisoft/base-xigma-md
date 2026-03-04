use crate::util::msg::XigmaBot;
use std::time::Instant;
use whatsapp_rust::bot::MessageContext;

pub async fn handle(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    XigmaBot::reply(ctx, "Pong!!", true).await?;

    let duration = start.elapsed();

    let pesan = format!("Speed: ```{} ms```\n> Xigma-MD 2026", duration.as_millis());

    XigmaBot::reply_ad(ctx, &pesan).await?;

    Ok(())
}
