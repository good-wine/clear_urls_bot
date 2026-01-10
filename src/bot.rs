use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters, MessageEntityKind};
use teloxide::utils::html;
use regex::Regex;
use crate::{sanitizer::RuleEngine, ai_sanitizer::AiEngine, db::Db, i18n};

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
    tracing::info!(chat_id = %msg.chat.id, msg_id = %msg.id, "Processing incoming message");
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    tracing::Span::current().record("user_id", user_id);
    
    let user_config = db.get_user_config(user_id).await.unwrap_or_else(|e| {
        tracing::error!(error = %e, "Failed to fetch user config, using default");
        crate::models::UserConfig::default()
    });
    let tr = i18n::get_translations(&user_config.language);

    // 1. Detect URLs early
    let (text, entities) = if let Some(t) = msg.text() {
        (t, msg.entities())
    } else if let Some(c) = msg.caption() {
        (c, msg.caption_entities())
    } else {
        ( "", None )
    };

    let mut has_urls = entities.as_ref().map(|e| e.iter().any(|entity| {
        matches!(entity.kind, MessageEntityKind::Url | MessageEntityKind::TextLink { .. })
    })).unwrap_or(false);

    // Manual fallback detection for schemeless URLs or cases where Telegram detection fails
    if !has_urls {
        // Simple but effective regex for detection
        let url_pattern = r"(?i)(?:https?://|www\.)[a-zA-Z0-9\-\.]+\.[a-zA-Z]{2,}(?:/[^\s]*)?";
        if let Ok(re) = Regex::new(url_pattern) {
            if re.is_match(text) {
                has_urls = true;
                tracing::debug!("URL detected via manual regex fallback");
            }
        }
    }

    // Handle Commands
    if let Some(text_val) = msg.text() {
        if text_val.starts_with('/') {
            let cmd_parts: Vec<&str> = text_val.split('@').collect();
            let cmd = cmd_parts[0];
            let is_private = msg.chat.is_private();
            let bot_username = config.bot_username.to_lowercase();
            
            let is_targeted = if cmd_parts.len() > 1 {
                cmd_parts[1].to_lowercase().starts_with(&bot_username)
            } else {
                is_private
            };

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
                        return Ok(())
                    }
                    "/help" => {
                        bot.send_message(chat_id, tr.help_text).parse_mode(ParseMode::Html).await?;
                        return Ok(())
                    }
                    "/stats" => {
                        let stats_text = tr.stats_text.replace("{}", &user_config.cleaned_count.to_string());
                        bot.send_message(chat_id, stats_text).parse_mode(ParseMode::Html).await?;
                        return Ok(())
                    }
                    _ => {}
                }
            }
        }
    }

    // Persist/Update chat info
    let is_group_context = msg.chat.is_group() || msg.chat.is_supergroup() || msg.chat.is_channel();
    let mut chat_config = db.get_chat_config_or_default(chat_id.0).await.unwrap_or_else(|e| {
        tracing::error!(error = %e, "Failed to fetch chat config, using default");
        crate::models::ChatConfig::default()
    });

    if is_group_context {
        let title = msg.chat.title().map(|s| s.to_string());
        let chat_config_db = db.get_chat_config(chat_id.0).await.unwrap_or(None);
        let chat_exists = chat_config_db.is_some();
        
        // Only save if it's new or title changed
        if !chat_exists || chat_config.title != title {
            chat_config.title = title.clone();
            if !chat_exists {
                chat_config.added_by = user_id;
            }
            let _ = db.save_chat_config(&chat_config).await;
        }

        if !chat_exists && user_id != 0 && has_urls {
            let notify_text = format!(
                "🛡️ <b>ClearURLs attivato!</b>\n\nHo iniziato a proteggere il gruppo: <b>{}</b>\n\nPuoi disattivarlo o cambiare modalità dal tuo dashboard.",
                html::escape(&title.unwrap_or_else(|| "Sconosciuto".to_string()))
            );
            let _ = bot.send_message(ChatId(user_id), notify_text)
                .parse_mode(ParseMode::Html)
                .await;
        }
    }

    if !has_urls {
        return Ok(());
    }

    // Logic: In groups, only check if the group enabled the bot.
    // In private, check if the user enabled the bot.
    let is_enabled = if is_group_context {
        chat_config.is_enabled()
    } else {
        user_config.is_enabled()
    };

    if !is_enabled {
        tracing::info!(is_group_context, chat_id = %chat_id, "Bot is disabled for this context (skipping)");
        return Ok(())
    }

    let ignored_domains: Vec<String> = user_config.ignored_domains.split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let custom_rules = db.get_custom_rules(user_id).await.unwrap_or_default();
    let mut cleaned_urls = Vec::new();

    let mut url_candidates = Vec::new();

    // 1. Get URLs from Telegram Entities
    if let Some(ents) = entities {
        let utf16: Vec<u16> = text.encode_utf16().collect();
        for entity in ents {
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
            if !url_candidates.contains(&url_str) {
                tracing::debug!(url = %url_str, "Found URL via Telegram entity");
                url_candidates.push(url_str);
            }
        }
    }

    // 2. Supplement with Regex Detection
    let url_pattern = r"(?i)(?:https?://|www\.)[a-zA-Z0-9\-\.]+\.[a-zA-Z]{2,}(?:/[^\s]*)?";
    if let Ok(re) = Regex::new(url_pattern) {
        for mat in re.find_iter(text) {
            let url_str = mat.as_str().to_string();
            if !url_candidates.contains(&url_str) {
                tracing::debug!(url = %url_str, "Found URL via Regex fallback");
                url_candidates.push(url_str);
            }
        }
    }

    if url_candidates.is_empty() {
        tracing::debug!("No URL candidates found in message");
        return Ok(());
    }

    // 3. Process candidates
    for url_str in url_candidates {
        // 1. Expand shortened URLs first
        let expanded_url = rules.expand_url(&url_str).await;
        let original_url_str = url_str.clone();
        let mut current_url = expanded_url;

        // 2. Sanitization
        if let Some((cleaned, provider)) = rules.sanitize(&current_url, &custom_rules, &ignored_domains) {
             current_url = cleaned;
             tracing::info!(provider = %provider, "URL sanitized by engine");
             
             if user_config.is_ai_enabled() && config.ai_api_key.is_some() {
                 if let Ok(Some(ai_cleaned)) = ai.sanitize(&current_url).await {
                     current_url = ai_cleaned;
                     let provider_name = format!("AI ({})", provider);
                     cleaned_urls.push((original_url_str, current_url, provider_name));
                     continue;
                 }
             }

             cleaned_urls.push((original_url_str, current_url, provider));
        } else {
             tracing::debug!(url = %current_url, "URL was already clean");
             if user_config.is_ai_enabled() && config.ai_api_key.is_some() {
                 if let Ok(Some(ai_cleaned)) = ai.sanitize(&current_url).await {
                     tracing::info!("URL sanitized by AI fallback");
                     cleaned_urls.push((original_url_str, ai_cleaned, "AI (Deep Scan)".to_string()));
                 }
             }
        }
    }

    if cleaned_urls.is_empty() {
        tracing::info!("Processing finished: no URLs required cleaning");
        return Ok(())
    }

    let _ = db.increment_cleaned_count(user_id, cleaned_urls.len() as i64).await;
    for (orig, clean, prov) in &cleaned_urls {
        let _ = db.log_cleaned_link(user_id, orig, clean, prov).await;
        
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
        return Ok(())
    }

    let mut response = if is_group_context {
        let user_name = msg.from.as_ref().map(|u| u.first_name.clone()).unwrap_or_else(|| "User".into());
        tr.cleaned_for.replace("{}", &html::escape(&user_name))
    } else {
        String::from(tr.cleaned_links)
    };

    if !response.ends_with('\n') {
        response.push('\n');
    }

    const MAX_MESSAGE_LENGTH: usize = 4000; // Leave a buffer for Telegram's 4096 limit

    if cleaned_urls.len() == 1 {
        let clean = cleaned_urls[0].1.trim();
        let escaped_url = html::escape(clean);
        let link_entry = format!("<a href=\"{}\">{}</a>", escaped_url, escaped_url);
        
        if response.len() + link_entry.len() < MAX_MESSAGE_LENGTH {
            response.push_str(&link_entry);
        }
    } else {
        for (_, cleaned, _) in &cleaned_urls {
            let clean = cleaned.trim();
            let escaped_url = html::escape(clean);
            let link_entry = format!("• <a href=\"{}\">{}</a>\n", escaped_url, escaped_url);
            
            if response.len() + link_entry.len() > MAX_MESSAGE_LENGTH {
                response.push_str("... (truncated)");
                break;
            }
            response.push_str(&link_entry);
        }
    }
    
    tracing::info!(chat_id = %chat_id, "Sending cleaned URLs reply");
    
    use teloxide::types::LinkPreviewOptions;
    let mut request = bot.send_message(chat_id, response)
        .reply_parameters(ReplyParameters::new(msg.id))
        .parse_mode(ParseMode::Html)
        .link_preview_options(LinkPreviewOptions {
            is_disabled: true, // Keep groups clean by default
            url: None,
            prefer_small_media: false,
            prefer_large_media: false,
            show_above_text: false,
        });

    // Support for Supergroup topics/threads
    if let Some(thread_id) = msg.thread_id {
        request = request.message_thread_id(thread_id);
    }

    if let Err(e) = request.await {
        tracing::error!(chat_id = %chat_id, error = %e, "Failed to send cleaned URLs reply");
        return Err(e);
    }

    Ok(())
}
