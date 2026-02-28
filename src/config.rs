use std::fs;
use std::sync::{LazyLock, RwLock};

use anyhow::{anyhow, Context, Result};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

const CONFIG_PATH: &str = "src/config.ron";

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename = "BotConfig")]
pub struct BotConfig {
    #[serde(rename = "NO_OWNER")]
    pub no_owner: Vec<String>,
    #[serde(rename = "NAMA_OWNER")]
    pub nama_owner: String,
    #[serde(rename = "NAMA_BOT")]
    pub nama_bot: String,
    #[serde(rename = "THUMBNAIL_URL")]
    pub thumbnail_url: String,
}

pub static CONFIG: LazyLock<RwLock<BotConfig>> = LazyLock::new(|| {
    let cfg = load_config().expect("config.ron tidak valid. Periksa format di src/config.ron");
    RwLock::new(cfg)
});

fn load_config() -> Result<BotConfig> {
    let raw = fs::read_to_string(CONFIG_PATH)
        .with_context(|| format!("gagal membaca {}", CONFIG_PATH))?;
    let cfg = ron::from_str::<BotConfig>(&raw)
        .with_context(|| format!("format {} tidak valid", CONFIG_PATH))?;
    Ok(cfg)
}

fn save_config(cfg: &BotConfig) -> Result<()> {
    let pretty = PrettyConfig::new()
        .separate_tuple_members(true)
        .enumerate_arrays(true);
    let serialized =
        ron::ser::to_string_pretty(cfg, pretty).context("gagal serialize config.ron")?;
    fs::write(CONFIG_PATH, serialized).with_context(|| format!("gagal menulis {}", CONFIG_PATH))?;
    Ok(())
}

pub fn get_config() -> BotConfig {
    CONFIG
        .read()
        .expect("gagal read lock config")
        .clone()
}

fn normalize_digits(input: &str) -> String {
    input.chars().filter(|c| c.is_ascii_digit()).collect()
}

pub fn is_owner(raw_sender: &str) -> bool {
    let sender = raw_sender.split('@').next().unwrap_or(raw_sender);
    let sender_digits = normalize_digits(sender);
    let cfg = CONFIG.read().expect("gagal read lock config");
    cfg.no_owner.iter().any(|owner| {
        owner == sender || (!sender_digits.is_empty() && normalize_digits(owner) == sender_digits)
    })
}

pub fn owner_debug_info(raw_sender: &str) -> String {
    let sender = raw_sender.split('@').next().unwrap_or(raw_sender);
    let sender_digits = normalize_digits(sender);
    let cfg = CONFIG.read().expect("gagal read lock config");

    let owners_raw = cfg.no_owner.join(", ");
    let owners_norm: Vec<String> = cfg.no_owner.iter().map(|o| normalize_digits(o)).collect();
    let matched = cfg.no_owner.iter().any(|owner| {
        owner == sender || (!sender_digits.is_empty() && normalize_digits(owner) == sender_digits)
    });

    format!(
        "debug-owner:\n- sender_raw: {}\n- sender_jid: {}\n- sender_digits: {}\n- owners_raw: [{}]\n- owners_digits: [{}]\n- matched: {}",
        raw_sender,
        sender,
        sender_digits,
        owners_raw,
        owners_norm.join(", "),
        matched
    )
}

pub fn add_owner(raw_number: &str) -> Result<bool> {
    let number = normalize_digits(raw_number);
    if number.len() < 8 {
        return Err(anyhow!("nomor owner tidak valid"));
    }

    let mut cfg = CONFIG.write().expect("gagal write lock config");
    if cfg
        .no_owner
        .iter()
        .any(|o| o == &number || normalize_digits(o) == number)
    {
        return Ok(false);
    }

    cfg.no_owner.push(number);
    save_config(&cfg)?;
    Ok(true)
}

pub fn set_thumbnail_url(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(anyhow!("thumbnail harus berupa URL http/https"));
    }

    let mut cfg = CONFIG.write().expect("gagal write lock config");
    cfg.thumbnail_url = trimmed.to_string();
    save_config(&cfg)?;
    Ok(())
}
