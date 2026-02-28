use std::sync::Arc;

use anyhow::Result;
use qrcode::QrCode;
use wacore_binary::node::NodeContent;
use waproto::whatsapp::device_props;
use whatsapp_rust::bot::{Bot, MessageContext};
use whatsapp_rust::proto_helpers::MessageExt;
use whatsapp_rust::store::SqliteStore;
use whatsapp_rust::types::events::Event;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;
use std::fs;
use std::path::Path;

mod config;
mod controller;
mod handler;
mod util;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=================================");
    println!(" Xigma-MD 2026 - Next generation");
    println!(" WhatsApp Bot - Rust Edition");
    println!(" github   : magercode");
    println!(" telegram : xigmachat");
    println!("=================================");
    println!("Bot udah berjalan!\n");
    
    let folder_sesi = "session";
    if Path::new(folder_sesi).exists() {
    } else {
        fs::create_dir(folder_sesi)?;
    }
    let backend = Arc::new(SqliteStore::new("session/bot.db").await?);

    let builder = Bot::builder()
        .with_backend(backend)
        .with_os_info(
            Some("Ubuntu".to_string()),
            Some(device_props::AppVersion {
                primary: Some(2),
                secondary: Some(3000),
                tertiary: Some(1015901307),
                ..Default::default()
            }),
        )
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new());
    let mut bot = builder
        .on_event(move |event, client| async move {
            match event {
                Event::PairingQrCode { code, timeout } => {
                    let qr = QrCode::new(code.as_bytes()).unwrap();
                    let qr_string = qr
                        .render::<char>()
                        .quiet_zone(true)
                        .module_dimensions(2, 1)
                        .dark_color('#')
                        .light_color(' ')
                        .build();

                    println!("\nScan QR berikut di WhatsApp:\n");
                    println!("{qr_string}");
                    println!("\nQR code valid selama: {} detik", timeout.as_secs());
                }

                Event::PairSuccess(success) => {
                    println!("✅ Berhasil login sebagai: {}", success.id);
                }

                Event::Message(msg, info) => {
                    let ctx = MessageContext {
                        message: msg,
                        info,
                        client,
                    };

                    if let Some(text) = ctx.message.text_content() {
                        let _ = handler::dispatch(&ctx, text).await;
                    }
                }

                Event::Notification(node) => {
                    let from = node.attrs.get("from").map(|s| s.as_str()).unwrap_or("");
                    let n_type = node.attrs.get("type").map(|s| s.as_str()).unwrap_or("");

                    /*@> abaikan sw <@*/
                    if from == "status@broadcast" || n_type == "status" {
                        return;
                    }

                    let action_tag = if node.tag == "notification" {
                        node.content
                            .as_ref()
                            .and_then(|content| match content {
                                NodeContent::Nodes(nodes) => {
                                    nodes.first().map(|n| n.tag.as_str())
                                }
                                _ => None,
                            })
                            .unwrap_or(node.tag.as_str())
                    } else {
                        node.tag.as_str()
                    };

                    match action_tag {
                        "add" => {
                            println!("👥 User ditambahkan ke group");
                        }
                        "remove" => {
                            println!("👥 User dihapus dari group");
                        }
                        "promote" => {
                            println!("⭐ User dipromosikan jadi admin");
                        }
                        "demote" => {
                            println!("⬇️ User diturunkan dari admin");
                        }
                        "announcement" => {
                            println!("📢 Group dijadikan announcement");
                        }
                        "not_announcement" => {
                            println!("📢 Announcement dimatikan");
                        }
                        "locked" => {
                            println!("🔒 Group dikunci");
                        }
                        _ => {
                            println!("📱 Notifikasi lain: {:#?}", node);
                        }
                    }
                }

                Event::Disconnected(reason) => {
                    println!("❌ Disconnected: {:?}", reason);
                }

                Event::LoggedOut(logged_out) => {
                    println!("🚪 Logged out: {:?}", logged_out.reason);
                }

                Event::ClientOutdated(info) => {
                    println!("⚠️ Client WhatsApp outdated!\n{:?}", info);
                }

                Event::Connected(_) => {
                    println!("🌐 Connected to WhatsApp servers");
                }
                _ => {}
            }
        })
        .build()
        .await?;

    match bot.run().await {
        Ok(handle) => {
            println!("Bot udah berjalan...");
            handle.await?
        }
        Err(e) => {
            eprintln!("Error woi coba benerin :v {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
