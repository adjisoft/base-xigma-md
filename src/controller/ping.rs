use crate::util::msg::XigmaBot;
use crate::util::stopwatch::Stopwatch;
use whatsapp_rust::bot::MessageContext;

pub async fn handle(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    let mut timer = Stopwatch::new();
    timer.start();

    XigmaBot::reply(ctx, "Pong!!", true).await?;

    let kecepatan = timer.stop();
    let pesan = format!(
        "Speed: ```{}ms```\n> Xigma-MD 2026",
        kecepatan.as_millis()
    );

    XigmaBot::reply_ad(ctx, &pesan).await?;

    Ok(())
}
