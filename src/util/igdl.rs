use reqwest;
use serde::{Deserialize, Serialize};

// --- MODELS ---
#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub full_name: String,
    pub username: String,
    pub is_verified: String,
    pub profile_pic_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Caption {
    pub hashtags: Vec<String>,
    pub created_at: i64,
    pub mentions: Vec<String>,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Statistics {
    pub comment_count: Option<i64>,
    pub fb_comment_count: Option<i64>,
    pub fb_like_count: Option<i64>,
    pub fb_play_count: Option<i64>,
    pub ig_play_count: Option<i64>,
    pub like_count: Option<i64>,
    pub play_count: Option<i64>,
    pub repost_count: Option<i64>,
    pub save_count: Option<i64>,
    pub share_count: Option<i64>,
    pub user_follower_count: Option<i64>,
    pub user_media_count: Option<i64>,
    pub view_count: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaData {
    #[serde(rename = "type")]
    pub media_type: String,
    pub height: i32,
    pub width: i32,
    pub thumb: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResultData {
    pub profile: Profile,
    pub caption: Caption,
    pub statistics: Statistics,
    pub data: Vec<MediaData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: bool,
    pub status_code: i32,
    pub creator: String,
    pub result: ResultData,
}

// --- FUNCTIONS ---
pub async fn download_instagram_reel(url: &str) -> Result<ApiResponse, Box<dyn std::error::Error>> {
    let encoded_url = urlencoding::encode(url);
    let api_url = format!(
        "https://api.vreden.my.id/api/v1/download/instagram?url={}",
        encoded_url
    );

    println!("Fetching data from API...");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client.get(&api_url).send().await?;

    println!("Status: {}", response.status());

    if !response.status().is_success() {
        return Err(format!("API Error: {}", response.status()).into());
    }

    let api_response: ApiResponse = response.json().await?;

    if !api_response.status {
        return Err("API returned status false".into());
    }

    Ok(api_response)
}
