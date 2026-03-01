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
use whatsapp_rust::pair_code::PairCodeOptions;

use std::fs;
use std::path::Path;
use std::io::{Write, self};

mod config;
mod controller;
mod handler;
mod util;

fn input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).unwrap();
    buffer.trim().to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=================================");
    println!(" Xigma-MD 2026 - Next generation");
    println!(" WhatsApp Bot - Rust Edition");
    println!("=================================\n");

    let method_login = config::get_config()
        .method_login
        .to_lowercase();

    let folder_sesi = "session";
    let db_path = "session/bot.db";
    if !Path::new(folder_sesi).exists() {
        fs::create_dir(folder_sesi)?;
    }

    let backend = Arc::new(SqliteStore::new("session/bot.db").await?);

    let mut builder = Bot::builder()
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

    match method_login.as_str() {
        "pairing" => {
            if !Path::new(db_path).exists() {
                let nowa = input("Masukan nomer WhatsApp (628xxxx): ");
                builder = builder.with_pair_code(PairCodeOptions {
                    phone_number: nowa,
                    ..Default::default()
                });
                println!("Login menggunakan Pair Code...");
            } else {
                println!("Data sesi ditemukan, mencoba login dengan sesi yang tersimpan...");
            }
        }
        "qrcode" => {
            println!("Login menggunakan QR Code...");
        }
        _ => {
            println!("Method login tidak dikenali, fallback ke QR Code...");
        }
    }

    let mut bot = builder
        .on_event(move |event, client| async move {
            match event {

                Event::PairingQrCode { code, timeout } => {
                    match QrCode::new(code.as_bytes()) {
                        Ok(qr) => {
                            let qr_string = qr
                                .render::<char>()
                                .quiet_zone(true)
                                .module_dimensions(2, 1)
                                .dark_color('#')
                                .light_color(' ')
                                .build();

                            println!("\nScan QR berikut:\n");
                            println!("{qr_string}");
                            println!("QR berlaku {} detik\n", timeout.as_secs());
                        }
                        Err(e) => {
                            eprintln!("Gagal generate QR: {}", e);
                        }
                    }
                }

                Event::PairingCode { code, timeout } => {
                    println!("\n=================================");
                    println!("PAIR CODE ({} detik)", timeout.as_secs());
                    println!("Masukan kode ini di:");
                    println!("WhatsApp > Linked Devices > Link with phone number\n");
                    println!(">>> {} <<<", code);
                    println!("=================================\n");
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

                    if let Some(text) = ctx
                        .message
                        .text_content()
                        .or_else(|| ctx.message.get_caption())
                    {
                        if let Err(e) = handler::dispatch(&ctx, text).await {
                            eprintln!("Handler error: {:?}", e);
                        }
                    }
                }

                Event::Notification(node) => {
                    let from = node.attrs.get("from").map(|s| s.as_str()).unwrap_or("");
                    let n_type = node.attrs.get("type").map(|s| s.as_str()).unwrap_or("");

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
                        "add" => println!("👥 User ditambahkan ke group"),
                        "remove" => println!("👥 User dihapus dari group"),
                        "promote" => println!("⭐ User dipromosikan jadi admin"),
                        "demote" => println!("⬇️ User diturunkan dari admin"),
                        "announcement" => println!("📢 Group jadi announcement"),
                        "not_announcement" => println!("📢 Announcement dimatikan"),
                        "locked" => println!("🔒 Group dikunci"),
                        _ => println!("📱 Notifikasi lain: {}", action_tag),
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
            println!("🚀 Bot berjalan...");
            handle.await?;
        }
        Err(e) => {
            eprintln!("❌ Gagal menjalankan bot: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}