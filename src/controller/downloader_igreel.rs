use crate::util::igdl;
use crate::util::msg::XigmaBot;
use regex::Regex;
use whatsapp_rust::bot::MessageContext;

pub async fn handle(
    ctx: &MessageContext,
    instagram_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let urlnya =
        Regex::new(r"(?:https?://)?(?:www\.)?instagram\.com/(?:reel|p|reels)/([a-zA-Z0-9_-]+)")?;

    if !urlnya.is_match(instagram_url) {
        XigmaBot::reply(ctx, "Itu bukan url instagram!", true).await?;
        return Ok(());
    }

    XigmaBot::reply(
        ctx,
        "Tunggu bentar...\n(Kalo gak ke kirim kirim brati timeout/gagal)",
        true,
    )
    .await?;

    let apinya = igdl::download_instagram_reel(instagram_url).await?;

    if let Some(video_data) = apinya.result.data.iter().find(|m| m.media_type == "video") {
        let caption = format!(
            "✅ *Instagram Downloader*\n\n\
             👤 *Owner*: {}\n\
             📝 *Caption*: {}\n\n\
             _Xigma-MD_",
            apinya.result.profile.full_name, apinya.result.caption.text
        );

        XigmaBot::send_video(ctx, &video_data.url, &caption, true).await?;
    } else {
        XigmaBot::reply(
            ctx,
            "Gagal menemukan video. Mungkin itu postingan foto?",
            true,
        )
        .await?;
    }

    Ok(())
}
