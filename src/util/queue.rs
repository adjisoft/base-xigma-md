use crate::util::msg::XigmaBot;
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};
use whatsapp_rust::bot::MessageContext;

static ACTIVE_QUEUE: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

pub struct QueuePermit {
    key: String,
}

impl Drop for QueuePermit {
    fn drop(&mut self) {
        if let Ok(mut active) = ACTIVE_QUEUE.lock() {
            active.remove(&self.key);
        }
    }
}

pub async fn acquire(
    ctx: &MessageContext,
    label: &str,
) -> Result<Option<QueuePermit>, Box<dyn std::error::Error>> {
    let key = label.trim().to_lowercase();

    if key.is_empty() {
        return Ok(Some(QueuePermit {
            key: "_default".to_string(),
        }));
    }

    let mut already_running = false;
    {
        let mut active = ACTIVE_QUEUE
            .lock()
            .map_err(|_| "queue lock poisoned")?;
        if active.contains(&key) {
            already_running = true;
        } else {
            active.insert(key.clone());
        }
    }

    if already_running {
        XigmaBot::reply(
            ctx,
            &format!("Perintah `{}` sedang diproses. Tunggu sampai selesai.", label),
            true,
        )
        .await?;
        return Ok(None);
    }

    Ok(Some(QueuePermit { key }))
}
