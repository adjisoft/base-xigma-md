//dadu.rs

use crate::util::msg::TembagaBot;
use rand::Rng;
use whatsapp_rust::bot::MessageContext;

pub async fn handle(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    let angka: u8 = {
        let mut rng = rand::rng();
        rng.random_range(1..=6)
    };

    let teks = format!("🎲 Kamu dapat angka: *{}*", angka);

    TembagaBot::reply(ctx, &teks, true).await?;
    Ok(())
}
