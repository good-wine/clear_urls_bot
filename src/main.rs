mod config;
mod models;
mod db;
mod sanitizer;
mod ai_sanitizer;
mod bot;
mod web;
mod i18n;

use teloxide::prelude::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use crate::config::Config;
use crate::db::Db;
use crate::sanitizer::RuleEngine;
use crate::ai_sanitizer::AiEngine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "clear_urls_bot=info,teloxide=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    config.validate();
    
    let db = Db::new(&config.database_url).await?;
    let rules = RuleEngine::new(&config.clearurls_source).await?;
    let ai = AiEngine::new(&config);
    let bot = Bot::new(&config.bot_token);
    
    // Canale per eventi real-time (SSE)
    let (event_tx, _) = tokio::sync::broadcast::channel::<serde_json::Value>(100);

    let bot_task = bot::run_bot(bot, db.clone(), rules.clone(), ai, config.clone(), event_tx.clone());
    let web_task = web::run_server(config, db, event_tx);

    let rules_refresh = rules.clone();
    let refresh_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(86400)); // 24 ore
        loop {
            interval.tick().await;
            if let Err(e) = rules_refresh.refresh().await {
                tracing::error!("Failed to refresh rules: {}", e);
            }
        }
    });

    tokio::select! {
        _ = bot_task => {
            tracing::error!("Bot task finished unexpectedly");
        }
        _ = web_task => {
            tracing::error!("Web server task finished unexpectedly");
        }
        _ = refresh_task => {
            tracing::error!("Refresh task finished unexpectedly");
        }
    }

    Ok(())
}
