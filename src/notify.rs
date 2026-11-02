use anyhow::{Context, Result};
use reqwest::Client;

/// Envia uma mensagem pelo bot do Telegram.
/// Variáveis de ambiente: TELEGRAM_TOKEN e TELEGRAM_CHAT_ID.
pub async fn send(text: &str) -> Result<()> {
    let token = std::env::var("TELEGRAM_TOKEN").context("defina TELEGRAM_TOKEN")?;
    let chat = std::env::var("TELEGRAM_CHAT_ID").context("defina TELEGRAM_CHAT_ID")?;
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");

    let client = Client::new();
    client
        .post(&url)
        .json(&serde_json::json!({
            "chat_id": chat,
            "text": text,
            "disable_web_page_preview": true
        }))
        .send()
        .await
        .context("falha de rede ao falar com o Telegram")?;
    Ok(())
}
