use crate::util::igdl;
use crate::util::msg::TembagaBot;
use regex::Regex;
use whatsapp_rust::bot::MessageContext;

pub async fn handle(
    ctx: &MessageContext,
    instagram_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let urlnya =
        Regex::new(r"(?:https?://)?(?:www\.)?instagram\.com/(?:reel|p|reels)/([a-zA-Z0-9_-]+)")?;

    if !urlnya.is_match(instagram_url) {
        TembagaBot::reply(ctx, "Itu bukan url instagram!", true).await?;
        return Ok(());
    }

    TembagaBot::reply(ctx, "Tunggu bentar...", true).await?;

    let apinya = igdl::download_instagram_reel(instagram_url).await?;

    if let Some(video_data) = apinya.result.data.iter().find(|m| m.media_type == "video") {
        let caption = format!(
            "✅ *Instagram Downloader*\n\n\
             👤 *Owner*: {}\n\
             📝 *Caption*: {}\n\n\
             _Tembaga-MD_",
            apinya.result.profile.full_name, apinya.result.caption.text
        );

        TembagaBot::send_video(ctx, &video_data.url, &caption, true).await?;
    } else {
        TembagaBot::reply(
            ctx,
            "Gagal menemukan video. Mungkin itu postingan foto?",
            true,
        )
        .await?;
    }

    Ok(())
}
