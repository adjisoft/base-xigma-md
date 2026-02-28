use crate::config;
use crate::util::msg::XigmaBot;
use whatsapp_rust::bot::MessageContext;

pub async fn handle(ctx: &MessageContext) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::get_config();
    let mut contacts = Vec::new();

    for (index, owner) in cfg.no_owner.iter().enumerate() {
        let owner_name = if cfg.no_owner.len() == 1 {
            "Owner Xigma-MD".to_string()
        } else if index == 0 {
            "Owner Utama".to_string()
        } else {
            format!("Owner {}", index + 1)
        };

        contacts.push((owner_name, owner.as_str()));
    }

    let contact_refs: Vec<(&str, &str)> = contacts
        .iter()
        .map(|(name, number)| (name.as_str(), *number))
        .collect();

    XigmaBot::send_contacts(ctx, "Kontak Owner Xigma-MD", contact_refs, true).await?;

    XigmaBot::reply(
        ctx,
        &format!(
            "Kontak owner Xigma-MD\n\nTotal owner: {}\n\nHubungi untuk:\n- Laporan bug\n- Saran fitur\n- Kolaborasi\n- Bantuan teknis",
            cfg.no_owner.len()
        ),
        true,
    )
    .await?;

    Ok(())
}
