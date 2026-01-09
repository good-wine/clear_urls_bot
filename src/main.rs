mod config;
mod models;
mod db;
mod sanitizer;
mod bot;
mod web;

use teloxide::prelude::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use crate::config::Config;
use crate::db::Db;
use crate::sanitizer::RuleEngine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "clear_urls_bot=info,teloxide=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    
    let db = Db::new(&config.database_url).await?;
    let rules = RuleEngine::new(&config.clearurls_source).await?;
    let bot = Bot::new(&config.bot_token);
    
    let bot_task = bot::run_bot(bot, db.clone(), rules.clone());
    let web_task = web::run_server(config, db);

    tokio::select! {
        _ = bot_task => {
            tracing::error!("Bot task finished unexpectedly");
        }
        _ = web_task => {
            tracing::error!("Web server task finished unexpectedly");
        }
    }

    Ok(())
}
