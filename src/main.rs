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
    
    // Create a custom reqwest client with a longer timeout for Telegram polling
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    let bot = Bot::with_client(&config.bot_token, client);
    
    // Canale per eventi real-time (SSE)
    let (event_tx, _) = tokio::sync::broadcast::channel::<serde_json::Value>(100);

    let bot_task = tokio::spawn(bot::run_bot(bot, db.clone(), rules.clone(), ai, config.clone(), event_tx.clone()));
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
        res = bot_task => {
            match res {
                Ok(_) => tracing::error!("Bot task finished unexpectedly"),
                Err(e) => tracing::error!("Bot task panicked: {:?}", e),
            }
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
