use clear_urls_bot::config::Config;
use clear_urls_bot::db::Db;
use clear_urls_bot::sanitizer::RuleEngine;
use clear_urls_bot::ai_sanitizer::AiEngine;
use clear_urls_bot::bot;
use clear_urls_bot::web;
use clear_urls_bot::logging;
use teloxide::Bot;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init_logging();

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
