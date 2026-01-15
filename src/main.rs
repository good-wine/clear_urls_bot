use clear_urls_bot::{
    ai_sanitizer::AiEngine,
    bot,
    config::Config,
    db::Db,
    logging,
    sanitizer::RuleEngine,
};
use std::time::Duration;
use teloxide::Bot;
use tokio::time::interval;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init_logging();

    let pid = std::process::id();
    tracing::info!(pid = %pid, "ClearURLs Bot starting up");

    let config = Config::from_env();
    config.validate();

    let db = Db::new(&config.database_url).await?;
    let rules = RuleEngine::new_lazy(&config.clearurls_source);
    let ai = AiEngine::new(&config);

    // Create a custom reqwest client with a longer timeout for Telegram polling
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    let bot = Bot::with_client(&config.bot_token, client);

    // Canale per eventi real-time (SSE) - kept for bot logic, though not used in GraphQL yet
    let (event_tx, _) = tokio::sync::broadcast::channel::<serde_json::Value>(100);

    let bot_task = tokio::spawn(bot::run_bot(
        bot,
        db.clone(),
        rules.clone(),
        ai,
        config.clone(),
        event_tx.clone(),
    ));

    let rules_refresh = rules.clone();
    let refresh_task = tokio::spawn(async move {
        if let Err(e) = rules_refresh.refresh().await {
            tracing::error!("Failed initial rules fetch: {}", e);
        }
        let mut interval = interval(Duration::from_secs(86400));
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(e) = rules_refresh.refresh().await {
                tracing::error!("Failed to refresh rules: {}", e);
            }
        }
    });

    tokio::select! {
        res = bot_task => tracing::error!("Bot task finished: {:?}", res),
        res = refresh_task => tracing::error!("Refresh task finished: {:?}", res),
    }

    Ok(())
}