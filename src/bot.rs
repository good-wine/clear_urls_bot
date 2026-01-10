use teloxide::{
    prelude::*,
    types::{ParseMode, ReplyParameters, MessageEntityKind},
    utils::html,
};
use crate::{sanitizer::RuleEngine, ai_sanitizer::AiEngine, db::Db, models::ChatConfig, i18n};

pub async fn run_bot(
    bot: Bot, 
    db: Db, 
    rules: RuleEngine, 
    ai: AiEngine,
    config: crate::config::Config,
    event_tx: tokio::sync::broadcast::Sender<serde_json::Value>,
) {
    let handler = Update::filter_message()
        .endpoint(handle_message);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db, rules, ai, config, event_tx])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[tracing::instrument(
    skip(bot, db, rules, ai, config, event_tx),
    fields(chat_id = %msg.chat.id, user_id)
)]
async fn handle_message(
    bot: Bot,
    msg: Message,
    db: Db,
    rules: RuleEngine,
    ai: AiEngine,
    config: crate::config::Config,
    event_tx: tokio::sync::broadcast::Sender<serde_json::Value>,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    tracing::Span::current().record("user_id", user_id);
    
    let user_config = db.get_user_config(user_id).await.unwrap_or_default();
    let tr = i18n::get_translations(&user_config.language);

    // Handle Commands
    if let Some(text) = msg.text() {
        if text.starts_with('/') {
            let cmd = text.split('@').next().unwrap_or("");
            let is_private = msg.chat.is_private();
            let bot_username = format!("@{}", config.bot_username);
            let is_targeted = text.contains(&bot_username) || is_private;

            if is_targeted {
                match cmd {
                    "/start" => {
                        tracing::info!("Handling /start command for user {}", user_id);
                        let keyboard = teloxide::types::InlineKeyboardMarkup::new(vec![
                            vec![teloxide::types::InlineKeyboardButton::url(
                                tr.open_dashboard,
                                config.dashboard_url.clone(),
                            )],
                            vec![teloxide::types::InlineKeyboardButton::web_app(
                                "📱 Open Web App",
                                teloxide::types::WebAppInfo { url: config.dashboard_url.clone() },
                            )]
                        ]);

                        let welcome_text = tr.welcome.replace("{}", &user_id.to_string());
                        bot.send_message(chat_id, welcome_text)
                            .parse_mode(ParseMode::Html)
                            .reply_markup(keyboard)
                            .await?;
                        return Ok(());
                    }
                    "/help" => {
                        bot.send_message(chat_id, tr.help_text).parse_mode(ParseMode::Html).await?;
                        return Ok(());
                    }
                    "/stats" => {
                        let stats_text = tr.stats_text.replace("{}", &user_config.cleaned_count.to_string());
                        bot.send_message(chat_id, stats_text).parse_mode(ParseMode::Html).await?;
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }

    // Persist/Update chat info
    if msg.chat.is_group() || msg.chat.is_supergroup() || msg.chat.is_channel() {
        let title = msg.chat.title().map(|s| s.to_string());
        let chat_config_db = db.get_chat_config(chat_id.0).await.unwrap_or(None);
        let chat_exists = chat_config_db.is_some();
        
        let _ = db.save_chat_config(&ChatConfig {
            chat_id: chat_id.0,
            title: title.clone(),
            enabled: true,
            added_by: user_id,
            mode: chat_config_db.map(|c| c.mode).unwrap_or_else(|| "default".to_string()),
        }).await;

        // If it's a new chat, notify the user privately
        if !chat_exists && user_id != 0 {
            let notify_text = format!(
                "🛡️ <b>ClearURLs attivato!</b>\n\nHo iniziato a proteggere il gruppo: <b>{}</b>\n\nPuoi disattivarlo o cambiare modalità dal tuo dashboard.",
                html::escape(&title.unwrap_or_else(|| "Sconosciuto".to_string()))
            );
            let _ = bot.send_message(ChatId(user_id), notify_text)
                .parse_mode(ParseMode::Html)
                .await;
        }
    }

    let chat_config = db.get_chat_config_or_default(chat_id.0).await.unwrap_or_default();

    let chat_enabled = chat_config.enabled;
    let user_enabled = user_config.enabled;

    if !chat_enabled || !user_enabled {
        return Ok(())
    }

    let (text, entities) = if let Some(t) = msg.text() {
        (t, msg.entities())
    } else if let Some(c) = msg.caption() {
        (c, msg.caption_entities())
    } else {
        return Ok(());
    };

    let entities = match entities {
        Some(e) => e,
        None => return Ok(()),
    };

    let ignored_domains: Vec<String> = user_config.ignored_domains.split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let custom_rules = db.get_custom_rules(user_id).await.unwrap_or_default();

    let mut cleaned_urls = Vec::new();
    let utf16: Vec<u16> = text.encode_utf16().collect();

    for entity in entities {
        let url_str = match &entity.kind {
            MessageEntityKind::Url => {
                let start = entity.offset;
                let end = start + entity.length;
                if end > utf16.len() { continue; }
                String::from_utf16_lossy(&utf16[start..end])
            },
            MessageEntityKind::TextLink { url } => {
                url.to_string()
            },
            _ => continue,
        };

        let original_url_str = url_str.clone();
        let mut current_url = url_str;

        if let Some((cleaned, provider)) = rules.sanitize(&current_url, &custom_rules, &ignored_domains) {
             current_url = cleaned;
             
             // 3. AI Deep Scan (if enabled and standard rules/custom rules changed something OR we want it as final pass)
             if user_config.ai_enabled && config.ai_api_key.is_some() {
                 if let Ok(Some(ai_cleaned)) = ai.sanitize(&current_url).await {
                     current_url = ai_cleaned;
                     // We keep the provider name but mark AI was involved
                     let provider_name = format!("AI ({})", provider);
                     cleaned_urls.push((original_url_str, current_url, provider_name));
                     continue;
                 }
             }

             cleaned_urls.push((original_url_str, current_url, provider));
        } else if user_config.ai_enabled && config.ai_api_key.is_some() {
             // If standard rules didn't change it, maybe AI can
             if let Ok(Some(ai_cleaned)) = ai.sanitize(&current_url).await {
                 cleaned_urls.push((original_url_str, ai_cleaned, "AI (Deep Scan)".to_string()));
             }
        }
    }

    if cleaned_urls.is_empty() {
        return Ok(())
    }

    // Increment stats
    let _ = db.increment_cleaned_count(user_id, cleaned_urls.len() as i64).await;
    for (orig, clean, prov) in &cleaned_urls {
        let _ = db.log_cleaned_link(user_id, orig, clean, prov).await;
        
        // Broadcast SSE event
        let _ = event_tx.send(serde_json::json!({
            "user_id": user_id,
            "original_url": orig,
            "cleaned_url": clean,
            "provider_name": prov,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        }));
    }

    let mode = match chat_config.mode.as_str() {
        "default" | "" => user_config.mode.clone(),
        m => m.to_string(),
    };

    if mode == "delete" && bot.delete_message(chat_id, msg.id).await.is_ok() {
        let user_name = msg.from.as_ref().map(|u| u.first_name.clone()).unwrap_or_else(|| "User".into());
        let mut response = tr.cleaned_for.replace("{}", &html::escape(&user_name));
        for (_, cleaned, _) in &cleaned_urls {
            response.push_str(&format!("• <a href=\"{}\">{}</a>\n", html::escape(cleaned), html::escape(cleaned)));
        }
        bot.send_message(chat_id, response).parse_mode(ParseMode::Html).await?;
        return Ok(());
    }

    let mut response = String::from(tr.cleaned_links);
    for (_, cleaned, _) in &cleaned_urls {
         response.push_str(&format!("• {}\n", html::escape(cleaned)));
    }
    
    bot.send_message(chat_id, response)
        .reply_parameters(ReplyParameters::new(msg.id))
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}
