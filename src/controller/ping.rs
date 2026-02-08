use crate::util::msg::TembagaBot;
use crate::util::stopwatch::Stopwatch;
use whatsapp_rust::bot::MessageContext;

pub async fn handle(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    let mut timer = Stopwatch::new();
    timer.start();

    TembagaBot::reply(ctx, "Pong!!", true).await?;

    let kecepatan = timer.stop();
    let pesan = format!(
        "Speed: ```{}ms```\n> Tembaga-MD 2026",
        kecepatan.as_millis()
    );

    TembagaBot::reply_ad(ctx, &pesan).await?;

    Ok(())
}
