use crate::util::msg::XigmaBot;
use rand::prelude::*;
use whatsapp_rust::bot::MessageContext;

pub async fn handle(
    ctx: &MessageContext,
    pertanyaan: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let waktu = [
        "besok",
        "sekarang",
        "besok lusa",
        "tahun depan",
        "bulan depan",
        "minggu depan",
        "hari lagi",
        "gak tau 😅",
        "pas kamu siap",
        "nanti juga kejadian",
    ];

    let jawaban = {
        let mut rng = rand::rng();
        waktu.choose(&mut rng).unwrap().to_string()
    };

    let teks = format!("❓ *{}*\n\n🕒 Jawaban: *{}*", pertanyaan, jawaban);

    XigmaBot::reply(ctx, &teks, true).await
}
