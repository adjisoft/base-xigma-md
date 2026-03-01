use crate::{controller, util::msg::XigmaBot};
use waproto::whatsapp as wa;
use whatsapp_rust::bot::MessageContext;

pub async fn dispatch(ctx: &MessageContext, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let text = text.trim();

    println!("Pesan {} -> {}", ctx.info.source.sender, text);
    //println!("{:#?}", ctx.info);

    let mut chars = text.chars();

    let _prefix = match chars.next() {
        Some(c) if "/.!".contains(c) => c,
        _ => return Ok(()),
    };

    let noprefix = chars.as_str();
    let mut bagian = noprefix.split_whitespace();

    let cmd = bagian.next().unwrap_or("");
    let args = bagian.collect::<Vec<&str>>().join(" ");

    match cmd {
        /*@> menu & testing<@*/
        "menu" | "help" => controller::menu::handle(ctx).await?,
        "debug" | "d" => controller::debug::handle(ctx).await?,

        /*@> fun & downloader <@*/
        "dadu" | "roll" => controller::fun_dadu::handle(ctx).await?,
        "ping" | "speed" => controller::ping::handle(ctx).await?,
        "owner" | "own" => controller::owner::handle(ctx).await?,
        "addowner" => controller::owner_tools::add_owner(ctx, &args).await?,
        "setthumb" | "setthumbnail" => controller::owner_tools::set_thumbnail(ctx, &args).await?,
        "igdl" | "igreel" | "instagram" => {
            if args.is_empty() {
                let _ =
                    XigmaBot::reply(ctx, "📌 *Contoh:* .igdl https://instagram.com/xxx", true)
                        .await?;
                return Ok(());
            }
            controller::downloader_igreel::handle(ctx, &args).await?
        }
        "kapan" | "when" | "whenyah" => {
            if args.is_empty() {
                let _ = XigmaBot::reply(ctx, "Contoh: .kapan aku kaya", true).await;
                return Ok(());
            }
            controller::fun_kapan::handle(ctx, &args).await?
        }
        _ => {
            ctx.send_message(wa::Message {
                conversation: Some("Coba ketik .menu".to_string()),
                ..Default::default()
            })
            .await?;
        }
    }

    Ok(())
}

