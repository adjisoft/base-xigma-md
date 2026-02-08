use crate::config::NOMER_OWNER;
use crate::util::msg::TembagaBot;
use whatsapp_rust::bot::MessageContext;

pub async fn handle(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    let mut kontak = Vec::new();

    for (index, own) in NOMER_OWNER.iter().enumerate() {
        let nama_owner = if NOMER_OWNER.len() == 1 {
            "👑 Owner Tembaga-MD".to_string()
        } else {
            match index {
                0 => "👑 Owner Utama".to_string(),
                _ => format!("🤝 Owner {}", index + 1),
            }
        };

        kontak.push((nama_owner, *own));
    }

    let kontak_refs: Vec<(&str, &str)> = kontak
        .iter()
        .map(|(nama, no)| (nama.as_str(), *no))
        .collect();

    TembagaBot::send_contacts(ctx, "👑 Tembaga-MD Owners", kontak_refs, true).await?;

    TembagaBot::reply(
        ctx,
        &format!(
            "📞 *Kontak Owner Tembaga-MD*\n\n\
            Total owner: {}\n\n\
            Hubungi untuk:\n\
            - 🐛 Laporan bug\n\
            - 💡 Saran fitur\n\
            - 🤝 Kolaborasi\n\
            - 🔧 Bantuan teknis\n\n\
            _Jangan spam atau hubungi untuk hal tidak penting!_",
            NOMER_OWNER.len()
        ),
        true,
    )
    .await?;

    Ok(())
}
