use crate::{config, util::msg::XigmaBot};
use std::convert::TryFrom;
use waproto::whatsapp::{self as wa};
use whatsapp_rust::bot::MessageContext;

const DEFAULT_AMOUNT_1000: u64 = 10_000;
const DEFAULT_NOTE: &str = "Xigma-MD";
const CURRENCY_CODE: &str = "IDR";

fn usage_text() -> &'static str {
    "Contoh:\n/test\n/test 10000\n/test 10000 pesan"
}

fn parse_amount_and_note(args: &str) -> Result<(u64, String), String> {
    let raw = args.trim();
    if raw.is_empty() {
        return Ok((DEFAULT_AMOUNT_1000, DEFAULT_NOTE.to_string()));
    }

    let mut parts = raw.splitn(2, char::is_whitespace);
    let first = parts.next().unwrap_or("").trim();
    let rest = parts.next().unwrap_or("").trim();

    match first.parse::<u64>() {
        Ok(amount1000) if amount1000 > 0 => {
            let note = if rest.is_empty() {
                DEFAULT_NOTE.to_string()
            } else {
                rest.to_string()
            };
            Ok((amount1000, note))
        }
        Ok(_) => Err(format!(
            "Nominal harus lebih dari 0.\n\n{}",
            usage_text()
        )),
        Err(_) => Err(format!(
            "Format nominal tidak valid.\n\n{}",
            usage_text()
        )),
    }
}

pub async fn handle(ctx: &MessageContext, args: &str) -> Result<(), Box<dyn std::error::Error>> {
    let sender = ctx.info.source.sender.to_string();
    if !config::is_owner(&sender) {
        XigmaBot::reply(ctx, "Hanya owner yang boleh memakai fitur ini.", true).await?;
        return Ok(());
    }

    let (amount1000, note_text) = match parse_amount_and_note(args) {
        Ok(parsed) => parsed,
        Err(msg) => {
            XigmaBot::reply(ctx, &msg, true).await?;
            return Ok(());
        }
    };

    let value = match i64::try_from(amount1000) {
        Ok(v) => v,
        Err(_) => {
            XigmaBot::reply(
                ctx,
                "Nominal terlalu besar untuk dikirim sebagai request payment.",
                true,
            )
            .await?;
            return Ok(());
        }
    };

    let note_message = wa::Message {
        extended_text_message: Some(Box::new(wa::message::ExtendedTextMessage {
            text: Some(note_text),
            ..Default::default()
        })),
        ..Default::default()
    };

    let payment_request = wa::message::RequestPaymentMessage {
        note_message: Some(Box::new(note_message)),
        currency_code_iso4217: Some(CURRENCY_CODE.to_string()),
        amount1000: Some(amount1000),
        request_from: None,
        expiry_timestamp: Some(0),
        amount: Some(wa::Money {
            value: Some(value),
            offset: Some(1000),
            currency_code: Some(CURRENCY_CODE.to_string()),
        }),
        background: None,
    };

    let message = wa::Message {
        request_payment_message: Some(Box::new(payment_request)),
        ..Default::default()
    };

    match ctx.send_message(message).await {
        Ok(_) => {
            XigmaBot::reply(
                ctx,
                &format!(
                    "request_payment_message test terkirim. amount1000: {} {}",
                    amount1000, CURRENCY_CODE
                ),
                true,
            )
            .await?
        }
        Err(e) => {
            XigmaBot::reply(
                ctx,
                &format!("Gagal mengirim request_payment_message: {}", e),
                true,
            )
            .await?
        }
    }

    Ok(())
}
